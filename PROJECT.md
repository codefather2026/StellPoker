# Stellar Poker — Trustless Texas Hold'em with ZK-MPC

[![CI](https://github.com/HitEmPoka/StellPoker/actions/workflows/ci.yml/badge.svg)](https://github.com/HitEmPoka/StellPoker/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Live Demo](https://img.shields.io/badge/demo-live-brightgreen)](https://stell-poker.vercel.app)

---

## The Problem

Every onchain card game has one flaw: someone always sees your cards. A trusted server deals them, or the smart contract holds them in plaintext. ZK proofs alone don't solve this — whoever generates the proof necessarily knows all inputs, including the full deck.

This is not a UI problem. It is a cryptographic one.

---

## The Solution

Stellar Poker is the first onchain card game where **no single party ever sees the full deck** — not the server, not the coordinator, not any individual node.

We achieve this by combining two cryptographic primitives:

- **REP3 Secret Sharing (MPC)** — The deck exists only as secret shares distributed across 3 independent TACEO coNoir nodes. Collusion requires compromising at least 2 nodes simultaneously.
- **UltraHonk ZK Proofs** — Every deal, community card reveal, and showdown is accompanied by a ZK proof verified onchain, guaranteeing the MPC committee computed honestly without revealing any private state.

The result: provably fair poker with cryptographically private hands, trustless settlement, and no reliance on any trusted third party.

---

## Why This Is Only Possible Now

This project is a direct product of Stellar's Protocol 25 and 26 upgrades:

| Upgrade | What it added | How we use it |
|---|---|---|
| Protocol 25 (X-Ray) | Native BN254 elliptic-curve ops, Poseidon2 hashing | ZK-friendly commitments; UltraHonk proof verification |
| Protocol 26 (Yardstick) | Multi-scalar multiplication, scalar-field arithmetic, curve-membership checks | Shplemini polynomial opening scheme inside the verifier |

Without these host functions, verifying a single UltraHonk proof onchain would exceed Soroban's instruction budget. Protocol 25 and 26 are not incidental — they are what make this project possible.

---

## Architecture

```
Players
  │
  ▼
Next.js Frontend          Freighter wallet, game UI
  │
  ▼
Coordinator (Axum)        Orchestrates MPC sessions, submits proofs to Soroban
  │
  ├── MPC Node 0 ──┐
  ├── MPC Node 1 ──┼──  TACEO coNoir · REP3 secret sharing · co-SNARK proving
  └── MPC Node 2 ──┘
  │
  ▼
Soroban Smart Contracts
  ├── PokerTable          Betting, state machine, pot settlement
  ├── ZKVerifier          UltraHonk proof verification (BN254 native ops)
  └── CommitteeRegistry   MPC node registration and slashing
```

---

## ZK Circuits

Three Noir circuits cover the full game lifecycle:

| Circuit | Proves |
|---|---|
| `deal_valid` | Deck is a valid 52-card permutation; Merkle root matches hand commitments; each player received the correct cards |
| `reveal_board_valid` | Community cards match the committed deck; no index is reused across rounds |
| `showdown_valid` | Hole cards match commitments; hand evaluation is correct; declared winner has the best hand |

All three circuits are proved inside the MPC network using coSNARKs, so private inputs (the deck, salts) never leave the secret-shared domain.

---

## Deployed Contracts — Stellar Testnet

| Contract | Address |
|---|---|
| Poker Table | [`CB7M3V3P...XCYGL`](https://stellar.expert/explorer/testnet/contract/CB7M3V3POQJR66425J3ILLHS3T4EUBRY67R7AVKSM255WBWOZG7XCYGL) |
| Committee Registry | [`GBTYELEQ...KRDU`](https://stellar.expert/explorer/testnet/account/GBTYELEQ2YZH2W6SXLHT4AX6TYBHHU7LNNPKJV7J37VS3S5GPA75KRDU) |

ZK proofs are generated and verified onchain in every hand of solo mode. This is not mocked.

---

## Beyond Poker

The patterns here are general-purpose and reusable:

- **`stellar-zk-cards`** — a standalone Rust crate for card encoding and hand evaluation, usable by any Soroban app
- **`zk-verifier` contract** — a general-purpose UltraHonk verifier; swap the verification key to verify any Noir circuit
- **MPC committee pattern** — directly applicable to sealed-bid auctions, private voting, threshold key custody, and blind order matching in DeFi
- **Coordinator + node services** — structured for reuse in any coSNARK application

---

## What Is Live

| Component | Status |
|---|---|
| Soroban contracts | ✅ Live on testnet |
| ZK proof verification | ✅ Verified onchain every hand |
| Frontend | ✅ Hosted on Vercel |
| Solo mode (vs AI) | ✅ Full deal → bet → showdown → settlement |
| Multiplayer (2–6 players) | ✅ Functional with Freighter + testnet XLM |
| MPC nodes | ⚠️ Demo infrastructure (production deployment requires independent node operators) |

---

## Tech Stack

| Layer | Technology |
|---|---|
| Smart contracts | Soroban · Rust · soroban-sdk 22.0.0 |
| ZK circuits | Noir · UltraHonk · Barretenberg |
| MPC | TACEO coNoir · REP3 · 3-party |
| Hash function | Poseidon2 |
| Frontend | Next.js 15 · Freighter wallet |
| Coordinator | Axum (Rust) |

---

**[Live Demo](https://stell-poker.vercel.app)** · **[Repository](https://github.com/HitEmPoka/StellPoker)** · **[Architecture Slide Deck](https://www.canva.com/design/DAHB5JrdEAk/XThK1QgbEATHwZ0rX-W2aA/view?utm_content=DAHB5JrdEAk&utm_campaign=designshare&utm_medium=link2&utm_source=uniquelinks&utlId=hb4aca74548)**
