#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck > /dev/null && shellcheck "$0"

# This is imported by cw3-fixed-multisig, which is imported by cw3-flex-multisig
# need to make a separate category to remove race conditions
ALL_CONTRACTS="cw20-atomic-swap cw20-bonding cw20-escrow cw20-staking cw20-merkle-airdrop"

for cont in $ALL_CONTRACTS; do
  (
    cd "contracts/$cont"
    echo "Publishing $cont"
    cargo publish
  )
done

echo "Everything is published!"
