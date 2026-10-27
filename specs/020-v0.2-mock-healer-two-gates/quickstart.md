# Quickstart: 020-v0.2-mock-healer-two-gates

```bash
cargo build --release
cd /some/scratch/dir

# Happy path: Gate 1 RED -> repair -> Gate 2 GREEN, exit 0
replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video recordings/RF-01.mp4 --mock-heal
cat .replayfix/audit_receipt.json     # status: VERIFIED_REPAIRED

# Adversarial: empty patch is rejected, exit 30
replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video recordings/RF-01.mp4 --mock-heal --mock-patch noop
cat .replayfix/triage_report.json     # status: REPAIR_NOOP_REJECTED

# Unsafe output directory, exit 11
replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video recordings/RF-01.mp4 --mock-heal --output-dir ../escape
```

Automated checks: `cargo test --workspace`.
