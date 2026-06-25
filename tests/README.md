# Integration Tests

End-to-end tests that exercise the full stack: MPC nodes → coordinator → Soroban contracts.

## Prerequisites

A running local stack:
```bash
docker-compose up -d
```

## Run

```bash
python3 scripts/test-flow.py
```

Or against testnet:
```bash
NETWORK=testnet python3 scripts/test-flow.py
```

Or against the checked-in staging environment:
```bash
TEST_ENV_FILE=.env.staging \
COORDINATOR_URL=$(grep '^NEXT_PUBLIC_COORDINATOR_URL=' .env.staging | cut -d= -f2-) \
python3 scripts/test-flow.py
```

For the full staging smoke path, use:
```bash
./scripts/run-staging-checks.sh
```

## Coverage

| Test | Description |
|---|---|
| `test_solo_hand` | Full solo hand: deal → betting → showdown → settlement |
| `test_multiplayer_hand` | 2-player hand with fold |
| `test_zk_proof_verification` | Verifies deal/reveal/showdown proofs are accepted onchain |
| `test_timeout_autofold` | Player misses action window, auto-fold triggers |
| `test_committee_slashing` | Misbehaving MPC node gets slashed in committee-registry |

See `scripts/test-flow.py` for the full implementation.
