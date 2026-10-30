# Data Model: Spec 100 — Release Manifest & Demo Evidence Schema

## 1. `DemoEvidenceManifest` (`demo-evidence-manifest.json`)
- `schema_version`: `"1.0"`
- `release_version`: `"1.0.0"`
- `git_commit`: Commit SHA of the baseline/release
- `video_file`: `"replayfix-v1.0.0-demo.mp4"`
- `video_sha256`: 64-char hex SHA-256 of the MP4 file
- `duration_seconds`: `180.0`
- `resolution`: `"1920x1080"`
- `fps`: `30`
- `acts`: Array of 3 `DemoActRecord` items:
  - `act_number`: `1 | 2 | 3`
  - `title`: Act title (`ACT 1 — GROUNDED DEFECT`, `ACT 2 — CHEATING AND NO-OP TRAP REJECTION`, `ACT 3 — AUTONOMOUS CAUSAL REPAIR AND CRYPTOGRAPHIC PROOF`)
  - `start_sec` / `end_sec`: Time window (`0..45`, `45..90`, `90..180`)
  - `commands_executed`: Exact CLI commands executed
  - `observed_exit_codes`: Array of integer exit codes (`[0]`, `[30, 11]`, `[0, 0]`)
  - `evidence_artifacts`: Map of relative artifact paths to SHA-256 digests

## 2. `ReleasePackageManifest` (`SHA256SUMS.txt` & `SIGNATURE-VERIFICATION.txt`)
- Standard GNU coreutils / `sha256sum`-compatible `<sha256_hex>  <relative_path>` lines covering all distributed files in `release/v1.0.0/`.
