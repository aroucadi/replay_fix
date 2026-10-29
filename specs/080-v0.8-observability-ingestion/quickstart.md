# Quickstart: ReplayFix v0.8.0

## 1. Trace Viewer Auto-Launcher
To run a reproduction and automatically launch the Playwright Trace Viewer timeline scrubber upon completion:
```bash
cargo run -p replayfix-cli -- repro --bug RF-01 --trace
```
*Note: In headless CI or non-interactive terminals, trace collection proceeds into `.replayfix/artifacts/` but GUI launch is safely bypassed.*

## 2. Ingesting a Live GitHub Issue
To ingest a real GitHub issue and run reproduction against its parsed steps:
```bash
cargo run -p replayfix-cli -- repro --issue https://github.com/facebook/react/issues/12345
```

## 3. Human Approval Gate
When ReplayFix synthesizes a verified candidate patch that passes Gate 2 and Regression Parity, it presents a colored diff:
```
--- a/src/components/Counter.tsx
+++ b/src/components/Counter.tsx
@@ -10,1 +10,1 @@
-  const count = state + 1;
+  const count = state;

Apply patch to base repository? [y/N]: y
[INFO] Patch applied to base repository successfully.
```

To automatically confirm in unattended or test scripts:
```bash
cargo run -p replayfix-cli -- repro --bug RF-01 --yes
```
