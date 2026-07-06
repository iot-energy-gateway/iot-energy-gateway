# IoT PAYG Energy Gateway — Stellar / Soroban

A pay-as-you-go energy gateway that bridges real-time hardware telemetry with
streaming escrow smart contracts on **Stellar (Soroban)**. Inspired by the
**Drips Wave** paradigm: if a payment stream pauses or an escrow runs dry, the
gateway daemon safely cuts off the relay — no credit, no power.

## System Architecture

```
┌──────────────┐    settle_telemetry()   ┌──────────────────────┐
│   Daemon     │ ──────────────────────►  │  EnergyGateway       │
│  (Rust CLI)  │ ◄──────────────────────  │  Soroban Contract    │
│              │  get_stream_status()     │                      │
│              │                          │  Storage (Persistent)│
│  ┌────────┐  │                          │  ┌────────────────┐  │
│  │ Relay  │  │                          │  │ Admin          │  │
│  │ GPIO / │  │                          │  │ HardwareId     │  │
│  │ Network│  │                          │  │ TokenAddress   │  │
│  └───┬────┘  │                          │  │ RatePerWh      │  │
│      │       │                          │  │ StreamBalances │  │
│      ▼       │                          │  └────────────────┘  │
│   Cut-off    │                          └──────────────────────┘
│   signal     │                                   │
└──────────────┘                          ┌────────▼────────┐
                                          │  Stellar Asset  │
                                          │  Contract (SAC) │
                                          └─────────────────┘
```

### How it Works

1. **Admin** deploys the contract and registers a hardware gateway ID, token
   contract address, and a per-watt-hour billing rate.
2. **Consumers** pre-approve the contract on the SAC token and call
   `deposit_stream()` to fund their escrow balance. Tokens are transferred from
   the consumer to the contract.
3. **Daemon** polls periodically: it samples watt-hour consumption from the
   hardware and calls `settle_telemetry()` (admin-authorized). The contract
   calculates the cost, deducts from the consumer's stream balance, and
   transfers the equivalent tokens to the admin.
4. When all stream balances are exhausted, `get_stream_status()` returns
   `false`, the contract emits a `RELAY_TRIGGERED` event, and the daemon
   physically opens the relay circuit.

### Contracts

| Function | Auth | Description |
|----------|------|-------------|
| `initialize(admin, hardware_id, token, rate_per_wh)` | admin | One-time setup |
| `deposit_stream(user, amount)` | user | Pulls tokens via SAC `transfer_from`, credits stream balance |
| `settle_telemetry(hardware_id, watt_hours)` | admin | Deducts cost from stream balances, transfers tokens to admin |
| `get_stream_status(hardware_id)` | — | Returns `true` if any funded balance remains |

### Events

| Topic | Payload | When |
|-------|---------|------|
| `SFUND` | `(user, new_balance, amount)` | Tokens deposited |
| `TSTL` | `(hardware_id, wh, cost, has_funds)` | Telemetry settled |
| `RELY` | `(hardware_id, cost)` | Last balance exhausted — relay must cut |

## Repository Structure

```
iot-energy-gateway/
├── .github/workflows/
│   ├── contract.yml          # CI for contract
│   └── daemon.yml            # CI for daemon
├── contracts/
│   └── energy_gateway/       # Soroban smart contract
│       ├── src/
│       │   ├── lib.rs        # Core contract logic
│       │   └── tests.rs      # Unit test suite
│       └── Cargo.toml
├── daemon/                   # IoT hardware daemon (Rust)
│   ├── src/
│   │   ├── main.rs           # Async event loop
│   │   ├── telemetry.rs      # Simulated Wh sensor
│   │   └── relay.rs          # Relay state mock
│   └── Cargo.toml
├── scripts/
│   └── deploy-testnet.sh     # Automated testnet deployment
├── ui/                       # Dashboard (Tauri / React — coming soon)
├── Cargo.toml                # Workspace manifest
├── .env.example              # Environment variables template
└── README.md
```

## Quickstart

### Prerequisites

- [Rust](https://rustup.rs) nightly with `wasm32v1-none` target
- [stellar CLI](https://github.com/stellar/stellar-cli) v22+
- A funded Testnet identity (or run the deploy script to generate one)

### Build the contract

```bash
cargo build --package energy_gateway --target wasm32v1-none --release
```

### Run tests

```bash
# Tests require Rust < 1.96 (use rustup to install e.g. 1.84):
#   rustup install 1.84.1 && rustup run 1.84.1 cargo test --package energy_gateway --release
cargo test --package energy_gateway --release
```

### Deploy to Testnet

```bash
chmod +x scripts/deploy-testnet.sh
./scripts/deploy-testnet.sh
```

The script will:
1. Create + fund a `deployer` identity if it does not exist
2. Build the contract to an optimised WASM binary
3. Deploy to `soroban-testnet.stellar.org:443`
4. Print the deployed **Contract ID** for use in `.env`

### Run the daemon simulator

```bash
cp .env.example .env
# Fill in:
#   STELLAR_RPC_URL=https://soroban-testnet.stellar.org:443
#   CONTRACT_ID=<deployed-contract-id>
#   HARDWARE_GATEWAY_KEY=<admin-secret-key>
cargo run --package energy-daemon
```

## Deployment

### Testnet

| Parameter | Value |
|-----------|-------|
| Network Passphrase | `Test SDF Network ; September 2015` |
| Soroban RPC URL | `https://soroban-testnet.stellar.org:443` |
| Contract ID | *(output of `scripts/deploy-testnet.sh`)* |

### Futurenet / Mainnet

Update the `RPC_URL` and `NETWORK_PASSPHRASE` in the deploy script, or pass
`--network futurenet` to the `stellar` CLI commands.

## Drips Wave Integration

The contract emulates a Drips-style streaming balance model:

- Each consumer maintains an independent `StreamBalance` in persistent storage.
- When all balances are zero, `get_stream_status()` returns `false`, signalling
  the daemon to cut the relay.
- The admin cannot withdraw tokens that belong to a funded stream — only
  settlement of verified telemetry unlocks those funds.

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `STELLAR_RPC_URL` | Yes | Soroban RPC endpoint |
| `CONTRACT_ID` | Yes | Deployed contract address (hex) |
| `HARDWARE_GATEWAY_KEY` | Yes | Secret key for the admin account |
| `POLL_INTERVAL_SECS` | No | Daemon polling interval (default: 60) |
| `RUST_LOG` | No | Tracing filter (default: `energy_daemon=info`) |
