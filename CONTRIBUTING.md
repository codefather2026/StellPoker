# Contributing to Stellar Poker

Thank you for your interest in contributing. This document explains how to get started.

## Development Setup

```bash
./scripts/setup.sh
```

See the [README](README.md) for full prerequisites.

## Project Structure

| Directory | Description |
|---|---|
| `contracts/` | Soroban smart contracts (Rust) |
| `circuits/` | Noir ZK circuits |
| `stellar-zk-cards/` | Reusable card-game library crate |
| `services/coordinator/` | MPC session orchestrator (Axum) |
| `services/node/` | MPC node (TACEO coNoir participant) |
| `app/` | Next.js frontend |
| `scripts/` | Build, deploy, and test scripts |

## Workflow

1. Fork the repository and create a branch from `main`.
2. Make your changes. Run the relevant tests before opening a PR.
3. Open a pull request with a clear description of what changed and why.

## Running Tests

**Soroban contracts:**
```bash
cargo test -p poker-table
```

**Noir circuits:**
```bash
cd circuits/lib && nargo test
```

**Integration tests:**
```bash
./scripts/test-flow.py
```

**Frontend:**
```bash
cd app && npm test
```

## Code Style

- Rust: `cargo fmt` and `cargo clippy --all-targets`
- TypeScript: the project uses ESLint (`cd app && npm run lint`)
- Noir: follow the existing module structure in `circuits/lib/`

## Local Stack (docker-compose)

```bash
docker-compose up
```

Each service (`soroban`, `mpc-node-0/1/2`, `coordinator`) defines a `healthcheck`,
and the `coordinator` only starts once Soroban and all three MPC nodes report
healthy. Check status with:

```bash
docker-compose ps
```

If you're running services directly instead of via docker-compose, use
`./scripts/start-local.sh` — it polls each node's `/health` endpoint (and the
coordinator's `/api/health`) and won't print the ready message until every
service actually responds:

```
=== Stack is ready — open http://localhost:3000 ===
```

### Common Startup Errors

**`co-noir not found`**
`./scripts/start-local.sh` requires the `co-noir` MPC binary on your `PATH`.
Install it with:
```bash
cargo install --git https://github.com/TaceoLabs/co-snarks --branch main co-noir
```

**Circuit not compiled**
If you see `ERROR: Circuit <name> not compiled`, run
`./scripts/compile-circuits.sh` (or let `start-local.sh` do it automatically —
set `SKIP_CIRCUIT_COMPILE=1` only if you've already compiled them).

**MPC node never becomes healthy / `start-local.sh` times out waiting**
This almost always means key generation hasn't finished or the CRS is
missing. Check:
- `./scripts/download-crs.sh` was run and `crs/` is populated.
- The node's stdout/stderr (printed inline by `start-local.sh`, or
  `docker-compose logs mpc-node-0`) for a panic or bind-address conflict.
- Ports `8101-8103` and `10000-10002` aren't already in use by a previous run
  (`pkill -f mpc-node` or `docker-compose down` to clean up stale processes).

**Coordinator fails health check, MPC nodes are healthy**
The coordinator depends on all three MPC nodes and Soroban being healthy
first (`depends_on: condition: service_healthy` in `docker-compose.yml`). If
it's still failing after the nodes are up, check `SOROBAN_RPC` is reachable
from inside the container — `http://soroban:8000` (not `localhost`) when using
docker-compose, and `${SOROBAN_RPC}` from `.env.local` when running locally.

**`No .env.local found — Soroban submission disabled`**
Run `./scripts/deploy-local.sh` first; it deploys the contracts and writes
the `.env.local` that `start-local.sh` and the coordinator read.

## Reporting Issues

Open a GitHub issue with steps to reproduce, expected behaviour, and actual behaviour.

## License

By contributing you agree that your contributions will be licensed under the [MIT License](LICENSE).
