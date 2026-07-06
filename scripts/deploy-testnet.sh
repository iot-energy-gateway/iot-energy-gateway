#!/usr/bin/env bash
set -euo pipefail

###############################################################################
# deploy-testnet.sh
#
# Automates the build & deployment of the EnergyGateway Soroban contract
# to Stellar Testnet.
#
# Prerequisites:
#   - Rust (nightly) with wasm32v1-none target
#   - stellar CLI  (https://github.com/stellar/stellar-cli)
#   - A funded Testnet identity
#
# Usage:
#   chmod +x scripts/deploy-testnet.sh
#   ./scripts/deploy-testnet.sh
###############################################################################

NETWORK="testnet"
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
RPC_URL="https://soroban-testnet.stellar.org:443"
IDENTITY="deployer"
CONTRACT_ALIAS="energy_gateway"
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "=== EnergyGateway — Testnet Deployment ==="
echo "Network : ${NETWORK}"
echo "RPC URL : ${RPC_URL}"
echo "Identity: ${IDENTITY}"
echo ""

# ------------------------------------------------------------------
# 1. Ensure the deployer identity exists; create + fund if missing
# ------------------------------------------------------------------
if ! stellar keys ls | grep -q "${IDENTITY}"; then
    echo ">> Creating and funding identity '${IDENTITY}' on ${NETWORK}..."
    stellar keys generate "${IDENTITY}" --network "${NETWORK}" --fund
    echo "   Done."
else
    echo ">> Identity '${IDENTITY}' already exists, skipping creation."
fi

# ------------------------------------------------------------------
# 2. Install wasm target (if not already installed)
# ------------------------------------------------------------------
echo ">> Ensuring wasm target is installed..."
rustup target add wasm32v1-none 2>/dev/null || true

# ------------------------------------------------------------------
# 3. Build the contract
# ------------------------------------------------------------------
echo ">> Building contract (release profile)..."
cd "${PROJECT_ROOT}"
cargo build \
    --package energy_gateway \
    --target wasm32v1-none \
    --release \
    -Z build-std=std,panic_abort \
    -Z build-std-features=panic_immediate_abort

WASM_PATH="${PROJECT_ROOT}/target/wasm32v1-none/release/energy_gateway.wasm"
if [ ! -f "${WASM_PATH}" ]; then
    echo "ERROR: WASM not found at ${WASM_PATH}"
    exit 1
fi
echo "   WASM built: ${WASM_PATH}"

# ------------------------------------------------------------------
# 4. Deploy to Testnet
# ------------------------------------------------------------------
echo ">> Deploying contract to ${NETWORK}..."
stellar contract deploy \
    --wasm "${WASM_PATH}" \
    --source-account "${IDENTITY}" \
    --network "${NETWORK}" \
    --alias "${CONTRACT_ALIAS}"

# ------------------------------------------------------------------
# 5. Print deployment summary
# ------------------------------------------------------------------
CONTRACT_ID=$(stellar contract id --network "${NETWORK}" --alias "${CONTRACT_ALIAS}" 2>/dev/null || true)
echo ""
echo "=== Deployment Summary ==="
echo "Contract Alias : ${CONTRACT_ALIAS}"
echo "Contract ID    : ${CONTRACT_ID}"
echo "Network        : ${NETWORK}"
echo "Passphrase     : ${NETWORK_PASSPHRASE}"
echo "RPC URL        : ${RPC_URL}"
echo ""
echo "Update your .env file with:"
echo "  STELLAR_RPC_URL=${RPC_URL}"
echo "  CONTRACT_ID=${CONTRACT_ID}"
echo "  HARDWARE_GATEWAY_KEY=<deployer-secret-key>"
