#![no_std]
#![allow(deprecated)]

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Symbol, Vec};

/// Committee Registry contract.
///
/// Manages MPC committee membership, staking bonds, and slashing hooks.
/// The committee is responsible for:
/// - Shuffling the deck via MPC
/// - Generating ZK proofs via coNoir
/// - Delivering private cards to players
/// - Responding to reveal requests within timeout
#[contract]
pub struct CommitteeRegistryContract;

#[contracttype]
#[derive(Clone, Debug)]
pub struct CommitteeMember {
    pub address: Address,
    pub stake: i128,
    pub endpoint: soroban_sdk::String, // MPC node endpoint URL
    pub active: bool,
    pub slash_count: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct CommitteeEpoch {
    pub epoch_id: u32,
    pub members: Vec<Address>,
    pub threshold: u32, // Minimum members needed (2 of 3)
    pub start_ledger: u32,
    pub end_ledger: u32, // 0 = no end (current epoch)
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum RegistryKey {
    Admin,
    StakeToken,
    MinStake,
    Member(Address),
    CurrentEpoch,
    Epoch(u32),
    SlashEvent(u32), // slash event counter
    Paused,
}

#[contractimpl]
impl CommitteeRegistryContract {
    /// Initialize the registry.
    pub fn initialize(env: Env, admin: Address, stake_token: Address, min_stake: i128) {
        admin.require_auth();
        assert!(
            !env.storage().instance().has(&RegistryKey::Admin),
            "already initialized"
        );

        env.storage().instance().set(&RegistryKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&RegistryKey::StakeToken, &stake_token);
        env.storage()
            .instance()
            .set(&RegistryKey::MinStake, &min_stake);
    }

    /// Pause the registry (admin only). All mutable operations revert while paused.
    /// NOTE: for production consider a timelock or multi-sig for unpause.
    pub fn pause(env: Env, admin: Address) {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&RegistryKey::Admin)
            .expect("not initialized");
        assert!(admin == stored_admin, "not admin");
        env.storage().instance().set(&RegistryKey::Paused, &true);
        env.events()
            .publish((Symbol::new(&env, "registry_paused"),), admin);
    }

    /// Unpause the registry (admin only).
    /// NOTE: for production consider a timelock or multi-sig here.
    pub fn unpause(env: Env, admin: Address) {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&RegistryKey::Admin)
            .expect("not initialized");
        assert!(admin == stored_admin, "not admin");
        env.storage().instance().set(&RegistryKey::Paused, &false);
        env.events()
            .publish((Symbol::new(&env, "registry_unpaused"),), admin);
    }

    /// Returns true if the registry is currently paused.
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get::<RegistryKey, bool>(&RegistryKey::Paused)
            .unwrap_or(false)
    }

    /// Register as a committee member with a stake.
    pub fn register_member(env: Env, member: Address, stake: i128, endpoint: soroban_sdk::String) {
        member.require_auth();
        assert!(
            !env.storage()
                .instance()
                .get::<RegistryKey, bool>(&RegistryKey::Paused)
                .unwrap_or(false),
            "contract paused"
        );

        let min_stake: i128 = env
            .storage()
            .instance()
            .get(&RegistryKey::MinStake)
            .expect("not initialized");
        assert!(stake >= min_stake, "insufficient stake");

        // Transfer stake to contract
        let token_addr: Address = env
            .storage()
            .instance()
            .get(&RegistryKey::StakeToken)
            .unwrap();
        let token = token::Client::new(&env, &token_addr);
        token.transfer(&member, &env.current_contract_address(), &stake);

        let member_state = CommitteeMember {
            address: member.clone(),
            stake,
            endpoint,
            active: true,
            slash_count: 0,
        };

        env.storage()
            .persistent()
            .set(&RegistryKey::Member(member.clone()), &member_state);

        env.events()
            .publish((Symbol::new(&env, "member_registered"),), member);
    }

    /// Withdraw stake and deregister (only when not in active epoch).
    pub fn deregister_member(env: Env, member: Address) -> i128 {
        member.require_auth();
        assert!(
            !env.storage()
                .instance()
                .get::<RegistryKey, bool>(&RegistryKey::Paused)
                .unwrap_or(false),
            "contract paused"
        );

        let mut m: CommitteeMember = env
            .storage()
            .persistent()
            .get(&RegistryKey::Member(member.clone()))
            .expect("not a member");

        // Check not in active epoch
        if let Some(epoch) = Self::get_current_epoch(env.clone()) {
            for i in 0..epoch.members.len() {
                assert!(
                    epoch.members.get(i).unwrap() != member,
                    "cannot deregister during active epoch"
                );
            }
        }

        let stake = m.stake;
        m.active = false;
        m.stake = 0;

        // Return stake
        let token_addr: Address = env
            .storage()
            .instance()
            .get(&RegistryKey::StakeToken)
            .unwrap();
        let token = token::Client::new(&env, &token_addr);
        token.transfer(&env.current_contract_address(), &member, &stake);

        env.storage()
            .persistent()
            .set(&RegistryKey::Member(member.clone()), &m);

        env.events()
            .publish((Symbol::new(&env, "member_deregistered"),), member);

        stake
    }

    /// Admin creates a new committee epoch with selected members.
    pub fn create_epoch(env: Env, admin: Address, members: Vec<Address>, threshold: u32) -> u32 {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&RegistryKey::Admin)
            .expect("not initialized");
        assert!(admin == stored_admin, "not admin");
        assert!(
            !env.storage()
                .instance()
                .get::<RegistryKey, bool>(&RegistryKey::Paused)
                .unwrap_or(false),
            "contract paused"
        );
        assert!(
            members.len() >= threshold,
            "not enough members for threshold"
        );

        // Verify all members are registered and active
        for i in 0..members.len() {
            let addr = members.get(i).unwrap();
            let m: CommitteeMember = env
                .storage()
                .persistent()
                .get(&RegistryKey::Member(addr.clone()))
                .expect("member not registered");
            assert!(m.active, "member not active");
        }

        // Close previous epoch
        let prev_epoch_id: u32 = env
            .storage()
            .instance()
            .get(&RegistryKey::CurrentEpoch)
            .unwrap_or(0);

        if prev_epoch_id > 0 {
            let mut prev: CommitteeEpoch = env
                .storage()
                .persistent()
                .get(&RegistryKey::Epoch(prev_epoch_id))
                .unwrap();
            prev.end_ledger = env.ledger().sequence();
            env.storage()
                .persistent()
                .set(&RegistryKey::Epoch(prev_epoch_id), &prev);
        }

        let epoch_id = prev_epoch_id + 1;
        let epoch = CommitteeEpoch {
            epoch_id,
            members: members.clone(),
            threshold,
            start_ledger: env.ledger().sequence(),
            end_ledger: 0,
        };

        env.storage()
            .persistent()
            .set(&RegistryKey::Epoch(epoch_id), &epoch);
        env.storage()
            .instance()
            .set(&RegistryKey::CurrentEpoch, &epoch_id);

        env.events()
            .publish((Symbol::new(&env, "epoch_created"), epoch_id), members);

        epoch_id
    }

    /// Trigger a slashing event against a committee member.
    /// Called by PokerTable contract when committee fails to act within timeout.
    pub fn report_slash(env: Env, reporter: Address, member: Address, reason: Symbol) {
        reporter.require_auth();
        assert!(
            !env.storage()
                .instance()
                .get::<RegistryKey, bool>(&RegistryKey::Paused)
                .unwrap_or(false),
            "contract paused"
        );

        // In production, verify reporter is an authorized PokerTable contract
        // For v1, any address can report (admin will adjudicate)

        let mut m: CommitteeMember = env
            .storage()
            .persistent()
            .get(&RegistryKey::Member(member.clone()))
            .expect("not a member");

        m.slash_count += 1;

        // Emit slash event for off-chain monitoring
        env.events().publish(
            (Symbol::new(&env, "slash_reported"), m.slash_count),
            (member.clone(), reason),
        );

        // If slash count exceeds threshold, deactivate and slash stake
        if m.slash_count >= 3 {
            let slashed = m.stake / 2; // Slash 50%
            m.stake -= slashed;
            m.active = false;
            // Slashed funds stay in contract (can be distributed to affected players)
        }

        env.storage()
            .persistent()
            .set(&RegistryKey::Member(member), &m);
    }

    /// View the current epoch.
    pub fn get_current_epoch(env: Env) -> Option<CommitteeEpoch> {
        let epoch_id: u32 = env
            .storage()
            .instance()
            .get(&RegistryKey::CurrentEpoch)
            .unwrap_or(0);

        if epoch_id == 0 {
            return None;
        }

        env.storage()
            .persistent()
            .get(&RegistryKey::Epoch(epoch_id))
    }

    /// View a member's state.
    pub fn get_member(env: Env, member: Address) -> CommitteeMember {
        env.storage()
            .persistent()
            .get(&RegistryKey::Member(member))
            .expect("not a member")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::Address as _,
        token::{StellarAssetClient, TokenClient},
        Address, Env, String, Vec,
    };

    fn setup() -> (
        Env,
        CommitteeRegistryContractClient<'static>,
        Address,
        TokenClient<'static>,
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(CommitteeRegistryContract, ());
        let client = CommitteeRegistryContractClient::new(&env, &contract_id);

        let token_admin = Address::generate(&env);
        let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token = TokenClient::new(&env, &sac.address());
        let token_sac = StellarAssetClient::new(&env, &sac.address());

        let admin = Address::generate(&env);
        client.initialize(&admin, &sac.address(), &100);

        // Mint tokens for a member
        let member = Address::generate(&env);
        token_sac.mint(&member, &1000);
        let _ = member; // avoid unused warning; caller mints their own

        (env, client, admin, token)
    }

    #[test]
    fn test_pause_and_unpause() {
        let (_env, client, admin, _token) = setup();
        assert!(!client.is_paused());
        client.pause(&admin);
        assert!(client.is_paused());
        client.unpause(&admin);
        assert!(!client.is_paused());
    }

    #[test]
    #[should_panic(expected = "contract paused")]
    fn test_paused_blocks_register_member() {
        let (env, client, admin, _token) = setup();
        client.pause(&admin);

        let member = Address::generate(&env);
        let endpoint = String::from_str(&env, "http://node0:8101");
        client.register_member(&member, &500, &endpoint);
    }

    #[test]
    #[should_panic(expected = "contract paused")]
    fn test_paused_blocks_create_epoch() {
        let (env, client, admin, _token) = setup();
        client.pause(&admin);

        let members: Vec<Address> = Vec::new(&env);
        client.create_epoch(&admin, &members, &2);
    }

    #[test]
    fn test_admin_can_read_while_paused() {
        let (_env, client, admin, _token) = setup();
        client.pause(&admin);
        // get_current_epoch is a read and must not panic
        let epoch = client.get_current_epoch();
        assert!(epoch.is_none());
    }

    #[test]
    fn test_unpause_allows_operations_again() {
        let (env, client, admin, _token) = setup();

        // Mint enough tokens for the member
        let token_admin = Address::generate(&env);
        let sac2 = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_sac2 = StellarAssetClient::new(&env, &sac2.address());
        let admin2 = Address::generate(&env);
        let contract_id2 = env.register(CommitteeRegistryContract, ());
        let client2 = CommitteeRegistryContractClient::new(&env, &contract_id2);
        client2.initialize(&admin2, &sac2.address(), &100);

        let member = Address::generate(&env);
        token_sac2.mint(&member, &500);

        client2.pause(&admin2);
        client2.unpause(&admin2);

        let endpoint = String::from_str(&env, "http://node0:8101");
        client2.register_member(&member, &500, &endpoint);
        let m = client2.get_member(&member);
        assert!(m.active);
    }

    #[test]
    #[should_panic(expected = "not admin")]
    fn test_non_admin_cannot_pause() {
        let (env, client, _admin, _token) = setup();
        let stranger = Address::generate(&env);
        client.pause(&stranger);
    }
}
