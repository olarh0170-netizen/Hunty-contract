#!/usr/bin/env bash
set -euo pipefail

ENVIRONMENT="${1:-}"
SOURCE_ACCOUNT="${2:-}"

case "$ENVIRONMENT" in
  testnet|staging)
    NETWORK="testnet"
    RPC_URL="${TESTNET_RPC_URL:-https://soroban-testnet.stellar.org}"
    PASSPHRASE="Test SDF Network ; September 2015"
    ;;
  mainnet)
    NETWORK="mainnet"
    RPC_URL="${MAINNET_RPC_URL:?Set MAINNET_RPC_URL}"
    PASSPHRASE="Public Global Stellar Network ; September 2015"
    ;;
  *)
    echo "Usage: $0 <testnet|staging|mainnet> <stellar-source-account>" >&2
    exit 1
esac

if [[ -z "$SOURCE_ACCOUNT" ]]; then
  echo "Missing Stellar source account name or secret key" >&2
  echo "Usage: $0 <testnet|staging|mainnet> <stellar-source-account>" >&2
  exit 1
fi

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || { echo "Missing required command: $1" >&2; exit 1; }
}

deploy_contract() {
  local name="$1"
  local wasm="target/wasm32v1-none/release/${name}.wasm"
  shift
  local constructor_args=("$@")

  if [[ ! -f "$wasm" ]]; then
    echo "Missing WASM: $wasm" >&2
    echo "Run 'stellar contract build' from the repository root first." >&2
    exit 1
  fi

  if [[ ${#constructor_args[@]} -gt 0 ]]; then
    stellar contract deploy \
      --wasm "$wasm" \
      --source "$SOURCE_ACCOUNT" \
      --rpc-url "$RPC_URL" \
      --network-passphrase "$PASSPHRASE" \
      -- "${constructor_args[@]}"
  else
    stellar contract deploy \
      --wasm "$wasm" \
      --source "$SOURCE_ACCOUNT" \
      --rpc-url "$RPC_URL" \
      --network-passphrase "$PASSPHRASE"
  fi
}

require_cmd stellar

echo "Deploying Hunty contracts to $ENVIRONMENT ($NETWORK)"

# Get deployer address for use as admin/minter
DEPLOYER_ADDRESS=$(stellar keys address "$SOURCE_ACCOUNT")

# Query native XLM token address for the network
XLM_TOKEN_ADDRESS=$(stellar contract id asset --asset native --network "$NETWORK")

# Deploy hunty-core with constructor arg: admin (set atomically at deploy
# time so initialize_admin cannot be front-run)
HUNTY_CORE_ID="$(deploy_contract hunty_core "$DEPLOYER_ADDRESS")"

# Deploy reward-manager with constructor args: admin, xlm_token, hunty_core
REWARD_MANAGER_ID="$(deploy_contract reward_manager \
  "$DEPLOYER_ADDRESS" \
  "$XLM_TOKEN_ADDRESS" \
  "$HUNTY_CORE_ID")"

# Deploy nft-reward with constructor args: admin, minter, max_supply, metadata
# max_supply is Option<u64>: pass null (None) for unlimited.
# Do NOT pass "0" — the CLI encodes it as Some(0), capping supply at zero so
# every mint fails with MaxSupplyReached.
# metadata: name, description, total_supply, creator
# The minter MUST be the reward-manager contract (not the deployer) so
# reward-manager can mint NFTs during distribution.
NFT_METADATA="{\"name\":\"Hunty NFT Reward\",\"description\":\"Reward NFTs for completed hunts\",\"total_supply\":0,\"creator\":\"$DEPLOYER_ADDRESS\"}"
NFT_REWARD_ID="$(deploy_contract nft_reward \
  "$DEPLOYER_ADDRESS" \
  "$REWARD_MANAGER_ID" \
  "0" \
  "$NFT_METADATA")"

invoke() {
  local id="$1"
  shift
  stellar contract invoke \
    --id "$id" \
    --source "$SOURCE_ACCOUNT" \
    --rpc-url "$RPC_URL" \
    --network-passphrase "$PASSPHRASE" \
    -- "$@"
}

echo "Linking contracts..."

# 1. Set the default NFT contract on reward-manager
invoke "$REWARD_MANAGER_ID" \
  -- set_nft_reward_contract \
  --admin "$DEPLOYER_ADDRESS" \
  --nft_contract "$NFT_REWARD_ID"
echo "  nft-reward linked to reward-manager."

# 2. Initialize hunty-core admin (constructor does not set one)
invoke "$HUNTY_CORE_ID" \
  -- initialize_admin \
  --admin "$DEPLOYER_ADDRESS"
echo "  hunty-core initialized."

# 3. Link reward-manager to hunty-core so complete_hunt can pay out
invoke "$HUNTY_CORE_ID" \
  -- set_reward_manager \
  --admin "$DEPLOYER_ADDRESS" \
  --reward_manager "$REWARD_MANAGER_ID"
echo "  reward-manager linked to hunty-core."

# 4. Register hunty-core as a distributor on reward-manager
invoke "$REWARD_MANAGER_ID" \
  -- add_authorized_contract \
  --admin "$DEPLOYER_ADDRESS" \
  --contract "$HUNTY_CORE_ID"
echo "  hunty-core registered as distributor on reward-manager."

cat > "config/contracts.${ENVIRONMENT}.json" <<JSON
{
  "environment": "${ENVIRONMENT}",
  "network": "${NETWORK}",
  "rpcUrl": "${RPC_URL}",
  "networkPassphrase": "${PASSPHRASE}",
  "contracts": {
    "huntyCore": "${HUNTY_CORE_ID}",
    "rewardManager": "${REWARD_MANAGER_ID}",
    "nftReward": "${NFT_REWARD_ID}"
  }
}
JSON

echo "HuntyCore      : $HUNTY_CORE_ID"
echo "RewardManager  : $REWARD_MANAGER_ID"
echo "NftReward      : $NFT_REWARD_ID"
echo "Wrote config/contracts.${ENVIRONMENT}.json"

# Validate the updated config does not contain placeholders
echo ""
echo "Validating config placeholders..."
bash scripts/validate_placeholders.sh

# Verify every cross-contract link before returning success
echo ""
echo "Running deployment verification..."
HUNTY_CORE_ID="$HUNTY_CORE_ID" \
REWARD_MANAGER_ID="$REWARD_MANAGER_ID" \
NFT_REWARD_ID="$NFT_REWARD_ID" \
SOURCE_ACCOUNT="$SOURCE_ACCOUNT" \
bash scripts/verify_deployment.sh "$NETWORK"
echo "Deployment verified."