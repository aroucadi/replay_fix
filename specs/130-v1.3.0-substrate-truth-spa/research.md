# Research: Spec 130 — Substrate Truth & Universal SPA Grounding (v1.3.0)

## 1. Universal Modern Web SPA Typology & Framework Detection

ReplayFix v1.3.0 introduces native support for real-world Single Page Applications (SPAs) rather than relying on synthetic static HTML mocks. Modern SPAs exhibit diverse build tools, development servers, and port-binding conventions that must be reliably detected from `package.json`.

### Toolchain Heuristics & Commands
| Framework / Toolchain | Detection Invariants (`package.json`) | Non-Interactive Dynamic Port Dev Command | Listening Port Log Regex |
|:---|:---|:---|:---|
| **Vite** | `dependencies.vite` or `devDependencies.vite` or `scripts.dev` contains `"vite"` | `npx vite --port 0 --strictPort false --no-open` | `Local:\s+http:\/\/(?:localhost\|127\.0\.0\.1):(\d+)\/` |
| **Next.js** | `dependencies.next` or `scripts.dev` contains `"next dev"` | `npx next dev -p 0` | `- Local:\s+http:\/\/(?:localhost\|127\.0\.0\.1):(\d+)` |
| **Create-React-App** | `dependencies.react-scripts` or `scripts.start` contains `"react-scripts start"` | `npx cross-env BROWSER=none PORT=0 CI=true react-scripts start` | `Local:\s+http:\/\/(?:localhost\|127\.0\.0\.1):(\d+)` |
| **Custom Node Script** | Fallback if `scripts.dev` or `scripts.start` is present | `npm run <script> -- --port 0` (with env `PORT=0`) | `(?:listening on\|server started at\|http:\/\/)(?:localhost\|127\.0\.0\.1):(\d+)` |

---

## 2. Dynamic Port Binding & Readiness Health Polling

### Port 0 Ephemeral Allocation
Hardcoding or guessing port numbers (e.g. `3000`, `5173`, `8080`) is strictly prohibited because it causes race conditions and collisions in concurrent verification runs and CI pipelines.
- **Child-Assigned Port 0**: When `PORT=0` or `--port 0` is passed, the underlying operating system kernel assigns an available ephemeral port.
- **Stdout/Stderr Parsing**: The dev server prints the resolved port to stdout/stderr. A bounded background reader captures this URL, rejecting literal `:0`, and extracts the real integer port.
- **Fallback Allocation**: If a toolchain does not support dynamic port flags, ReplayFix allocates an ephemeral port via `replayfix_core::Platform::allocate_ephemeral_port()`, passes that explicit port, and verifies socket binding.

### HTTP Readiness Polling Contract
Dev servers require a warmup compilation phase (transpiling TypeScript, building bundles, starting HMR websockets).
- **Polling Loop**: ReplayFix polls `GET /` and `GET /api/health` with a 200ms backoff interval.
- **Timeout Bound**: A strict 30-second deadline is enforced.
- **Early Exit Detection**: On each poll iteration, ReplayFix invokes `child.try_wait()`. If the child process exited prematurely (e.g., syntax error, missing dependency), ReplayFix captures stderr, terminates any dangling child sub-processes, and fails closed with `CoreError::Environment(String)` (Exit Code `25`).

---

## 3. Process Tree Supervision & RAII Teardown

Modern dev servers (especially Vite and Next.js) spawn nested worker threads and child node processes. Simply calling `child.kill()` on the root PID leaves orphaned background processes holding open TCP sockets on Windows and POSIX systems.

### Process Cleanup Strategy
- **Windows**: Use `Platform::terminate_process_tree(pid)`, which invokes `taskkill /F /T /PID <pid>` (or Windows Job Objects with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`).
- **POSIX**: Send `SIGKILL` to the process group (`kill(-pid, SIGKILL)`).
- **RAII Enforcement**: `ManagedSpaServer` implements `Drop`. If ReplayFix panics, encounters a timeout, or terminates early, the `Drop` implementation guarantees that the process tree is destroyed immediately.

---

## 4. Attached SPA Mode & CDP Synchronization

Attached Mode (`--url <endpoint> --repo <path>`) enables developers to point ReplayFix directly to a running application server without restarting it.

### The Hot-Reload Race Condition
When ReplayFix applies a repair patch to `--repo`, file watchers in Vite or Webpack detect changes and trigger Hot Module Replacement (HMR). If Gate 2 executes immediately, assertions may evaluate before the client bundle finishes updating in the browser.

### Synchronization Protocol
1. **CDP Connection**: Playwright CDP connects directly to `<endpoint>`.
2. **Gate 1 Baseline**: Record initial failure state on the live page.
3. **Patch Application**: Apply AST repair in the scratch workspace.
4. **Synchronous Reload**: Instead of relying on passive HMR websockets, ReplayFix commands Playwright CDP to execute `Page.reload({ waitUntil: "domcontentloaded" })`.
5. **Selector Stabilization**: Await target selector availability (`waitForSelector`) before dispatching Gate 2 assertions.
6. **Substrate Attribution**: `audit_receipt.json` explicitly records `substrate: "attached-spa-remote"`.

---

## 5. Inviolable Base Workspace Isolation ($\Delta\text{repo} = 0$)

### Constitution Law III Invariant
Constitution Law III dictates that the base repository is strictly read-only ($\Delta\text{repo} = 0$).
- In **Managed Mode**: The entire dev server runs inside an ephemeral scratch worktree in `std::env::temp_dir()`. The base repository is never touched.
- In **Attached Mode**: Trial patches are applied in a temporary shadow directory. The host repository is NEVER mutated during speculative trials. Only after Gate 2 achieves 2-run stability does the existing, human-approved `--apply` / `--yes` gate allow modifying the target repository.

---

## 6. Execution-Derived Provenance & Token Billing Rules

1. **Anti-Spoofing Mutual Exclusion**:
   - `build_manifest` checks `extras.execution_mode == Some(ExecutionMode::LiveAi)`.
   - If any mock flag (`mock_syntax_retry`, `mock_failing_repair`, `mock_pkg_test_mutation`, `mock_patch != "builtin"`) is present, reject execution with `ERR_USAGE` (Exit Code `2`).
2. **Telemetry Verification**:
   - Stamping `"provenance": "live-ai"` requires `receipt.repair_telemetry.provider_mode == "live-meta-muse-api"` or `receipt.video_telemetry.sam_snap.model == "sam-3.1"`.
   - All other runs default to `"deterministic-fixture"`.
3. **Billing Zeroing**:
   - When `provenance == "deterministic-fixture"`, `receipt.token_accounting` is set to `None`.
   - Guarantees 100% agreement with the disclaimer: *"Zero live external AI tokens billed"*.
