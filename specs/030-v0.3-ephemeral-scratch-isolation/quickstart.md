# Quickstart: 030-v0.3-ephemeral-scratch-isolation

```bash
cargo build --release

# 1. Run mock-heal against a target repository directory
mkdir -p /tmp/demo-repo/src
printf 'export function cartItemCount(cart) {\n  return cart.items.length;\n}\n' > /tmp/demo-repo/src/cart-summary.js

replayfix repro \
  --task-id RF-01 \
  --bug-ref bug/RF-01 \
  --video recordings/RF-01.mp4 \
  --repo /tmp/demo-repo \
  --mock-heal

# Verify receipt includes base_tree_sha256, sandbox_tree_sha256, base_repo_unmodified_verified: true, and ephemeral_port
cat .replayfix/audit_receipt.json

# Verify the base repository file was NOT modified (still has buggy source)
cat /tmp/demo-repo/src/cart-summary.js
```

Automated verification:
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```
