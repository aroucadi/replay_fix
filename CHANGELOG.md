# Changelog

All notable changes to the **ReplayFix** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [3.0.1] - 2026-11-07

**MMP Zero-Shot Engine & Responsive Breakpoint Release Fixes**: Completes universal zero-shot support for responsive CSS/media-query defects, pillarboxed mobile viewport recordings, vanished/detached DOM nodes (`DOM_NODE_DETACHED`), and dynamic benchmark defect branch hydration in isolated sandboxes.

### Fixed
- **Dynamic Sandbox Defect Branch Hydration (`engine.rs`, `environment.rs`, `platform.rs`)**:
  - Hydrates ephemeral scratch sandboxes from `MANIFEST.json` (`target_branch` / `repository_url`) when reproducing multi-branch benchmark suites while preserving strict $\Delta \text{repo} = 0$ on the host repository.
  - Populates a shared dependency cache with a pure `node:sqlite` (`DatabaseSync`) shim for `better-sqlite3` and wires `NODE_PATH` (stripping Windows verbatim UNC prefixes `\\?\`).
  - Wraps Windows `.cmd` tool invocations (`npm`, `npx`, `playwright`) via `cmd /C` in `Platform::create_command`.
- **Pillarboxed Mobile Viewport Auto-Detection (`video.rs`, `perception.rs`, `engine.rs`)**:
  - Added `FrameSet::detect_active_viewport_width` to detect uniform right-side pillarbox mattes when browser windows are resized to mobile/tablet widths inside a 1280x720 recording canvas, propagating `detected_viewport` into `GeometricCalibration` and `PlaywrightBridge::start`.
- **Responsive Hidden/Detached DOM Grounding & Gate 2 Re-Emergence (`browser.rs`, `engine.rs`)**:
  - Extended `PROBE_PAGE` candidate discovery to include `nav` / `[role="navigation"]` containers and CSS stylesheet interception.
  - Handled zero-sized (`display: none`) responsive containers in `execute_browser_gate` as a causal Gate 1 RED (`DOM_NODE_DETACHED`) and verified non-zero bounding-box restoration across 2 consecutive Gate 2 GREEN runs.
- **CSS/SCSS & HTML Source Discovery & Syntax Preflight (`discovery.rs`, `repair.rs`)**:
  - Added `.css` and `.scss` to `RepoSourceMapper::SOURCE_EXTENSIONS` with layout/media-breakpoint token ranking and dynamic post-Gate-1 target file refinement (`RepoSourceMapper::locate_target_file`).
  - Updated `BlockPatcher::apply` and `SyntaxPreflight::check_file` to validate balanced delimiters on `.css`/`.scss`/`.html` files instead of invoking `node --check`, and hardened `extract_search_replace_blocks` against `===` delimiter collisions inside source expressions.
- **Gateway Credential Rotation & Test Isolation (`gateway.rs`, `cli.rs`)**:
  - Prioritized `META_API_KEY` ahead of `REPLAYFIX_AI_KEY` with candidate key rotation across HTTP 401/403/429/5xx and inline upstream errors, plus `REPLAYFIX_TEST_NO_REG=1` test isolation to prevent `.env` or Windows Registry credential leakage during deterministic CLI test runs.

---

## [3.0.0] - 2026-11-05

**Spec 300 — Universal Zero-Shot Video-to-Code Repair Pipeline**: Delivers repository-agnostic, zero-shot video-driven debugging and repair powered by live Meta AI models (`muse-spark-1.3`, `muse-wonder-1.0`, `sam-3.1`) and genuine Playwright Chromium CDP browser execution. Full notes: [docs/release-notes/v3.0.0.md](docs/release-notes/v3.0.0.md).

### Added
- **Universal Zero-Shot Component Mount Harness (`discovery.rs`)**:
  - In-browser Babel Standalone transpilation configured with `['env', 'flow', 'react']` presets and recursive CommonJS `dynamicRequire` module loading for raw Flow/TypeScript/JSX component libraries (`react-resizable`, etc.).
  - `html_assets_exist` static bundle verification to skip unbuilt demo HTML templates.
  - `SelfContainedDraggableCore` with `offsetParent`-relative pointer coordinates and axis-specific ARIA slider labels (`Resize Handle NW`, `Resize Handle SE`) to faithfully reproduce DOM layout oscillation and drag-jitter defects in Chromium.
- **Multi-Frame Optical-Flow Trajectory & Physical Jitter Oracle (`perception.rs`, `browser.rs`)**:
  - Added `RoiDiffer::pair_change` frame-pair displacement calculation for multi-step drag vectors (`top_level_drag_end`) and graceful fallback from `sam-3.1` empty responses to `segment_from_frame_pair` (`local_diff_heuristic`).
  - Extended Playwright `PROBE_PAGE` and `DRAG_AND_OBSERVE` to measure step-by-step physical bounding-box acceleration and direction reversals (`jitter_score_bps`).
- **Execution Authenticity & Forensic Provenance Matrix (`receipt.rs`, `ui.rs`, `main.rs`)**:
  - Added 9-stage `StageExecutionMatrix` (`REAL`, `HEURISTIC`, `MOCKED`, `HARDCODED`, `SKIPPED`, `FAILED`, `UNVERIFIED`) to both CLI output and `audit_receipt.json`.
- **Flow/ES6 Method Scope Resolution & Live Wire Protocol (`repair.rs`, `gateway.rs`)**:
  - Updated `BlockPatcher::open_enclosing_declarations` to track parenthesis depth (`paren_depth`) and ES6/Flow class method declarations (`resizeHandler(...)`).
  - Fixed `"error": null` check on `https://api.meta.ai/v1/responses` and added `reasoning_tokens` extraction.

---

## [2.0.0] - 2026-11-03

**Spec 200 — Sovereign Enterprise Final MMP Release: Distributed Multi-Agent Swarm, Live Streaming Dashboard, APM & OpenTelemetry Ingestion, Cross-Browser Matrix & RFC 6962 Audit Ledger**: Delivers the final unified Sovereign Enterprise MMP release across all 5 architectural pillars and resolves every remaining partial-conformance item identified in the Master Specification Audit (`SPECIFICATION_AUDIT.md`, `ARCHITECTURE_AUDIT.md`, `TEST_QUALITY_AUDIT.md`, `COVERAGE_REPORT.md`, `FINAL_ASSESSMENT.md`). Full notes: [docs/release-notes/v2.0.0.md](docs/release-notes/v2.0.0.md).

### Added
- **Pillar 1 — Production APM, OpenTelemetry Span Alignment, SourceMap v3 & Secret Scrubbing (`replayfix ingest`, `replayfix-core::rrweb`)**:
  - Added `OtelSpan`, `AlignedInteractionSpan`, and `OtelIngestion` (`parse_json`, `align_with_rrweb`) to temporally correlate RRWeb user interactions with failing OpenTelemetry frontend/backend trace spans.
  - Added `SourceMapResolver` (`parse_json`, `resolve_candidate_sources`) to map minified bundle stack frames back to original TypeScript/JavaScript source paths (`src/...`), guarded by `TaskManifest::assert_safe_path`.
  - Added `sanitize_untrusted_telemetry_text` / `strip_ansi_and_redact` to automatically strip ANSI escape sequences and scrub credentials (`GITHUB_TOKEN`, `META_API_KEY`, `ghp_...`, `github_pat_...`, `sk-...`) from RRWeb inputs, Sentry exceptions, and APM breadcrumbs.
  - Extended `replayfix ingest` CLI with `--otel <path>` and `--sourcemap <path>`.
- **Pillar 2 — Distributed Work-Stealing Swarm Cluster (`replayfix swarm`, `replayfix-core::swarm`)**:
  - Extended `replayfix swarm` with `--tasks <task1.json> <task2.json>...` to ingest task manifest files directly alongside `--bugs`, plus `--notary-log <path>` for automatic receipt notarization.
  - Integrated live `DashboardBroadcaster` hooks (`with_broadcaster`) and automatic `TransparencyLog` appending (`with_notary_log`) into `SwarmCoordinator`.
- **Pillar 3 — Real-Time Live Streaming Dashboard (`replayfix dashboard`, `replayfix-core::dashboard`)**:
  - Extended `replayfix dashboard` with `--open` to launch the live observatory in interactive operator sessions.
  - Added `/api/notary` (serving `transparency_log.json` state) and `/api/matrix` (serving cross-browser inoculation matrix state) REST endpoints and `broadcast_dom_diff` / `broadcast_gate_verdict` SSE streaming methods.
- **Pillar 4 — Cross-Browser & Multi-Device Inoculation Matrix (`replayfix inoculate --matrix`, `replayfix-core::inoculate`)**:
  - Added `calibrate_viewport_with_letterbox` and `verify_matrix_parity` (`MatrixParityCell`, `MatrixParityReport`) across Chromium, Firefox, WebKit, Mobile Portrait, Pixel 7, Tablet, and Desktop presets.
  - Updated `replayfix inoculate` so `--matrix` works both as a standalone flag (`--matrix`) and with explicit values (`--matrix chromium,firefox,webkit`), and `--bug` defaults to `"RF-01"` when omitted.
- **Pillar 5 & Audit Convergence — Enterprise Multi-Party Attestation v2.0, 7-Leaf Canonical Merkle Receipts & Gateway Grounding (`crypto.rs`, `receipt.rs`, `gateway.rs`, `proof_viewer.html`)**:
  - Implemented `AttestationRole` (`Developer`, `Harness`, `CiJudge`), `PartySignature`, and `MultiPartyAttestation` (`sign_chain`, `verify_chain`) in `replayfix-core::crypto` (`FR-066`) and embedded the visual 3-party attestation chain inspector in `proof_viewer.html`.
  - Added `AuditReceipt::write_canonical_7leaf_artifacts` guaranteeing $\ge 7$ canonical RFC 6962 evidence leaves (`01_candidate.diff` through `07_worktree_manifest.json`) (`FR-026`, `NFR-009`).
  - Added `replayfix verify --receipt <path> --public-key <key>` alias and file/hex public key loader alongside `replayfix verify-receipt` (`FR-028`).
  - Added `MetaGatewayClient::ground_with_web_search` (`muse-spark-1.3`) and `synthesize_verification_badge` (`muse-image-1.0`) (`FR-036`).
  - Added `CoreError::Telemetry` and `CoreError::Security` domain error variants.

---

## [1.9.5] - 2026-11-03

**Spec 195 — Living Cybernetic Command Deck (24-Bit Truecolor, 60 FPS Motion Engine & Double-Buffered Bento TUI)**: Upgrades the ReplayFix Terminal UI (`crates/replayfix-cli/src/ui.rs` and `crates/replayfix-cli/src/tui/*`) into a 2026-era Living Cybernetic Command Deck inspired by Claude Code, Charm Lipgloss/Bubbles, and Lazygit while maintaining 100% BeautifulRust Law X zero-panic compliance and automatic non-TTY fallback. Full notes: [docs/release-notes/v1.9.5.md](docs/release-notes/v1.9.5.md).

### Added
- **ReplayFix Signature Brand Identity (`crates/replayfix-cli/src/tui/brand.rs`)**:
  - **Hero Splash Banner (`render_hero_splash`)**: Geometric Unicode block logotype combining playback triangle `▶` and causal bug target `󰒓 / ⬢`, rendered across an algorithmic 4-stop 24-bit Truecolor horizontal gradient: Electric Cyan (`#00d2ff`) -> Cyber Violet (`#7928ca`) -> Phosphor Amber (`#f59e0b`) -> Substrate Emerald (`#10b981`), paired with the `[ v1.9.0 ] [ Rust Substrate: 60 FPS ] [ Meta AI: Spark 1.3 + Wonder 1.0 ]` metadata ribbon.
  - **Compact HUD Emblem (`render_compact_hud_emblem`)**: Dual-segment micro-pill badge (`\x1b[48;2;18;24;38m\x1b[38;2;56;189;248m ▶⚡ REPLAYFIX \x1b[48;2;30;41;59m\x1b[38;2;148;163;184m v1.9.0 \x1b[0m`) with automatic `< 80` column collapse and `NO_COLOR`/non-TTY plain degradation.
- **Living Motion Engine (`crates/replayfix-cli/src/tui/motion.rs`)**:
  - 60 FPS sub-character tick engine (`MotionEngine`, `FRAME_DURATION = 16ms`).
  - 8-level Unicode fractional progress bars (`▏▎▍▌▋▊▉█` via `render_fractional_bar`).
  - Traveling 24-bit RGB sine-wave shimmer (`render_wave_shimmer`) during agent deliberation (`UNDERSTANDING` / `PLANNING`) and tool execution.
  - 1.5-second (90-frame) sinusoidal breathing prompt on `❯` (`render_breathing_prompt`).
  - Animated Gate 1 RED confirmation and Gate 2 GREEN verification flash transitions (`GateFlashAnimation`, `GateFlashKind`).
- **Asymmetric Bento Terminal Layout & Double-Buffered Differential Renderer (`crates/replayfix-cli/src/tui/layout.rs`)**:
  - `Cell`, `CellStyle`, `FrameBuffer`, and `DifferentialRenderer` emitting minimal cursor moves (`\x1b[{row};{col}H`) and Truecolor sequences only for mutated cells/rows, eliminating `Clear(ClearType::All)` redraw blink.
  - `BentoCommandDeck` (`80x24+` responsive 40%/60% asymmetric layout) with **Top Deck** (Branded HUD pill, active repo, `󰊢` branch badge, `:0` ephemeral port, and `tok/s` token rate laser meter), **Main Left Pane (40%)** (multi-agent thought stream with `effort: xhigh`, `[TOOL: ffmpeg]` invocation badges, and gate transition cards), **Main Right Pane (60%)** (Virtualized AST diff viewer with line-number gutters, hunk folding via `toggle_hunk_folding`, and 24-bit syntax coloring), and **Bottom Command Ribbon** (`Enter: Run`, `Tab: Switch`, `Space: Page`, `q: Quit`).

### Changed
- **Sovereign Web Portals (`proof_viewer.html` & `dashboard.html`)**:
  - **`proof_viewer.html` (Sovereign Cryptographic Notary Vault)**: Re-architected with `Instrument Serif` / `Fraunces` display typography, `Plus Jakarta Sans` structural copy, `JetBrains Mono` digest formatting, frosted obsidian glass (`backdrop-filter: blur(24px)`), `1px` structural hairline borders (`rgba(255,255,255,0.07)`), organic tactile SVG `feTurbulence` bond grain overlay, glowing Merkle inclusion audit paths, side-by-side surgical AST diff reviewer, and pulsing cryptographic seal badge while preserving 100% of the RFC 8785, RFC 6962, and RFC 8032 Ed25519 client-side verification engine and air-gapped CSP.
  - **`dashboard.html` (Autonomous Agent Swarm Observatory)**: Re-architected on a deep void canvas (`#08090c`) with `Syne` / `Cabinet Grotesk`, `Bricolage Grotesque`, and `Space Mono` typography, surgical contrast accents (`#f59e0b` phosphor amber, `#10b981` acid emerald, `#2563eb` international Klein blue), asymmetric Bento matrix, rate-pool token laser meters, parallel worker health tiles, subtle scanline overlay, and staggered `cubic-bezier(0.16, 1, 0.3, 1)` reveals while preserving all WebSocket (`/ws/screencast/:id`), SSE (`/events/reasoning/:id`), and REST telemetry hooks.
- **24-Bit Truecolor & Rounded Box Geometry (`ui.rs`, `tui/dialog.rs`, `tui/diff_view.rs`, `tui/spinner.rs`, `shell.rs`, `intent.rs`)**:
  - Replaced legacy 16-color ANSI escapes, ASCII square borders, and bracketed status strings with `CyberPalette` 24-bit Truecolor RGB escapes (`\x1b[38;2;...m`, `\x1b[48;2;...m`), rounded Unicode box geometry (`╭───╮`, `╰───╯`), and background-filled pill badges (`Theme::pill`, `Theme::status_pill`, `Theme::command_pill`).
  - Preserved strict RAII terminal restoration (`ScreenGuard` + panic hook) and automatic non-TTY plain-stream fallback when `!stdout().is_terminal()`.

---

## [1.9.1] - 2026-11-03

**Spec 190 Convergence & Hardening Patch — Benchmark Manifest Harmonization, Fast Playwright Resolution & Attached SPA Hydration Readiness**: Hardens the v1.9.0 Swarm Benchmark Engine and live Playwright CDP bridge across concurrent Windows test and sweep workloads, achieving a 100% pass rate across all 306 workspace tests with zero Clippy warnings and zero non-test panics. Full notes: [docs/release-notes/v1.9.1.md](docs/release-notes/v1.9.1.md).

### Fixed
- **Benchmark Sweep Manifest Resolution (`replayfix-core::sweep`, `v19_parallel_sweep.rs`)**:
  - Updated `ManifestParser::load_manifest` to resolve `suite_name` across `"benchmark_suite"`, `"suite"`, and `"suite_name"` keys while defaulting to `"MetaPulse-20"`.
  - Verified full `(suite_name, tasks)` tuple destructuring in `test_manifest_parser_loads_metapulse_manifest_safely` and harmonized `crates/replayfix-cli/tests/v19_parallel_sweep.rs` with the authoritative `MetaPulse-20` suite name.
- **Playwright Bridge Module Resolution & CDP Hydration Tolerance (`replayfix-core::browser`)**:
  - Prioritized fast synchronous filesystem/module candidates (`REPLAYFIX_PLAYWRIGHT_PATH`, `'playwright'`, `node_modules/playwright`, and local npx cache) in `loadPlaywright()` before falling back to `npm.cmd` subprocess invocations.
  - Added ancestor directory discovery from `current_dir()` and `current_exe()` in `PlaywrightBridge::start_with_script_and_timeouts` to propagate `NODE_PATH` and `REPLAYFIX_PLAYWRIGHT_PATH` into isolated scratch sandboxes.
  - Increased Windows CDP click hydration timeout to `20,000ms` and default bridge startup/command timeouts to `25s`.
- **Attached SPA DOM Hydration Readiness (`crates/replayfix-cli/tests/cli.rs`)**:
  - Added explicit HTTP readiness polling for both the root HTML shell (`/`) and client hydration bundle (`/public/app.js`) before dispatching attached browser reproduction runs in `test_attached_spa_mode_connects_to_external_url`.

---

## [1.9.0] - 2026-11-03

**Spec 190 — Pure Application-Agnostic Harness & High-Throughput Parallel Swarm Benchmark Engine**: Permanently decouples ReplayFix from hardcoded synthetic fixtures, eliminating `"has no built-in fixture for task"` across all workflows. Modern web applications across React, Vite, Next.js, and Express are dynamically grounded and evaluated based on generic observable signals (uncaught page exceptions, network 4xx/5xx responses, and DOM mutation activity). Adds the High-Throughput Parallel Swarm Benchmark Engine (`replayfix sweep`), processing the complete 20-defect MetaPulse benchmark suite (`RF-01` through `RF-20`) in 21.6s across concurrent workers with zero base repository mutation ($\Delta\text{repo}=0$), emitting schema-validated summaries (`benchmark_summary.json`, `telemetry_matrix.json`) and Ed25519-signed Merkle batch roots. Full notes: [docs/release-notes/v1.9.0.md](docs/release-notes/v1.9.0.md).

### Added
- **Pure Application-Agnostic Engine (`replayfix-core::manifest`, `replayfix-core::engine`)**:
  - `DynamicReproTask` enabling reproduction across arbitrary uninstrumented web repositories.
  - Universal dev server detection and ephemeral port binding in `ManagedSpaServer`.
  - Generic observable failure signals replacing all hardcoded selectors.
- **High-Throughput Parallel Swarm Benchmark Engine (`replayfix-core::sweep`, `replayfix sweep`)**:
  - Configurable concurrent worker swarm with dynamic work-stealing scheduling.
  - Pre/post worktree manifest verification guaranteeing strict $\Delta\text{repo}=0$ isolation.
  - Real-time fleet telemetry aggregator emitting `benchmark_summary.json` and `telemetry_matrix.json`.
  - Cryptographic batch notary with RFC 6962 Merkle tree chaining and Ed25519 attestation.

---

## [1.8.0] - 2026-11-02

**Spec 180 — Distributed Multi-Agent Swarm, Live Streaming Dashboard & Production APM Session Replay Ingestion**: Scales ReplayFix from an individual developer's CLI or bot into an enterprise distributed visual repair cluster and real-time observability center. Adds production APM telemetry ingestion (`replayfix ingest --sentry <url-or-file> --rrweb <file>`) transforming RRWeb DOM mutation recordings and Sentry/Datadog crash breadcrumbs into deterministic ReplayFix manifests (`TaskManifest`). Adds high-concurrency work-stealing swarm triage (`replayfix swarm --workers <N>`) with per-worker isolated scratch worktrees ($\Delta\text{repo}=0$), dynamic ephemeral ports, and rate-limited Meta Muse model pool concurrency. Adds an embedded async/threaded HTTP & WebSocket live streaming dashboard (`replayfix dashboard --port <port>`) broadcasting real-time CDP viewport screencasts, SSE reasoning token feeds, and side-by-side DOM diffs to maintainers. Extends permanent test inoculation (`replayfix inoculate --matrix`) to synthesize responsive Playwright test matrices across Desktop Chromium, Firefox, WebKit, and mobile viewports (Pixel 7, iPhone 14) with DPR coordinate calibration. Introduces the RFC 6962 Merkle Receipt Transparency Log and Notary (`replayfix notary`) chaining all triage receipts into an append-only verifiable cryptographic ledger (`transparency_log.json`). Full notes: [docs/release-notes/v1.8.0.md](docs/release-notes/v1.8.0.md).

### Added
- **Production APM & RRWeb Session Replay Ingestion (`replayfix-core::rrweb`, `replayfix ingest`)**:
  - `RrwebParser` parsing full DOM snapshots, incremental mutations, mouse clicks, scrolls, and text inputs from RRWeb v2 recordings.
  - `SentryIngestion` parsing unhandled exceptions, stack frames, candidate source files, and breadcrumbs, with remote crash fetching over HTTPS.
  - Automated reproduction manifest synthesis (`synthesize_manifest_from_telemetry`) with path traversal guards (`TaskManifest::assert_safe_path`).
- **Distributed Work-Stealing Triage Swarm (`replayfix-core::swarm`, `replayfix swarm`)**:
  - Multi-worker concurrent execution cluster with work-stealing task queues, per-worker isolated `ScratchSandbox` ($\Delta\text{repo}=0$), dynamic ephemeral ports, and rate-limited model pool governance.
  - Aggregated swarm execution summary serialization into `swarm_summary.json` matching `contracts/swarm_task.schema.json`.
- **Live Streaming Observability Dashboard (`replayfix-core::dashboard`, `replayfix dashboard`, `dashboard.html`)**:
  - Embedded zero-dependency HTTP & RFC 6455 WebSocket / SSE server broadcasting real-time screencast frames, turn-by-turn Muse Spark reasoning tokens, and REST status/receipt endpoints.
  - Standalone single-page `dashboard.html` with real-time viewport canvas, deliberation timeline, and strict Content Security Policy.
- **Cross-Browser & Multi-Device Inoculation Matrix (`replayfix-core::inoculate`, `replayfix inoculate --matrix`)**:
  - Multi-engine (`chromium,firefox,webkit`) and responsive viewport preset generation (`desktop`, `tablet`, `mobile-portrait`) with DPR geometric coordinate calibration.
- **RFC 6962 Merkle Receipt Transparency Log & Notary (`replayfix-core::notary`, `replayfix notary`)**:
  - Append-only cryptographic ledger (`transparency_log.json`) chaining triage receipts with Merkle consistency proofs between snapshot sizes and Ed25519 digital signatures.

---

## [1.7.0] - 2026-11-02

**Spec 170 — Autonomous GitHub Issue Ingestion Bot, Automated PR Dispatch & Continuous Triage Engine**: Evolves ReplayFix from a developer's local CLI tool into an autonomous continuous GitHub triage agent (`replayfix bot`). It connects directly to public and private GitHub issues and webhook event streams, extracts user-recorded bug videos (`.mp4`, `.webm`, `.mov`, and `user-attachments/assets`), executes the sub-60s Sovereign Healer (`v1.6` pipeline: SAM 3.1 + Muse Voice + Muse Spark Agent Loop + Muse Wonder AST Repair + Gate 2 GREEN + Gate 3 Stress Quorum + Inoculation) inside an ephemeral scratch sandbox ($\Delta\text{repo} = 0$), dispatches isolated branches (`replayfix/issue-<id>-<slug>`) with Ed25519-signed Merkle commit trailers, opens review-ready Pull Requests with comprehensive Evidence Matrix descriptions, posts verifiable visual comments and side-by-side DOM diffs to original issue threads, automates triage labels (`replayfix:verified-healed`, `replayfix:unreproduced`, `replayfix:ambiguous`), and provides a fail-closed HMAC-SHA256 authenticated webhook server (`replayfix bot --listen <addr>`). Full notes: [docs/release-notes/v1.7.0.md](docs/release-notes/v1.7.0.md).

### Added
- **Multi-Modal GitHub Issue Ingestion & Video Extraction (`replayfix-core::github`, `replayfix bot --issue <url>`)**:
  - `GitHubClient` and `GitHubTransport` trait supporting GitHub REST API v3 / `gh` CLI credentials (`GITHUB_TOKEN`, `GH_TOKEN`) and deterministic offline mock transport.
  - Markdown media extractor (`extract_video_urls_from_markdown`) targeting `user-attachments/assets` and `.mp4`, `.webm`, `.mov` URLs.
  - Media downloader and container validator (`validate_video_container`) enforcing MP4 (`ftyp`), WebM (EBML), and MOV magic byte checks before caching into `.replayfix/scratch/`.
- **Autonomous Ephemeral Scratch Triage & Inoculation (`replayfix-cli::bot`)**:
  - `IssueTriageRunner` executing autonomous reproduction and repair inside an ephemeral `ScratchSandbox`, enforcing strict host repository invariance ($\Delta\text{repo} = 0$).
  - Issue-specific permanent Playwright regression spec synthesis (`tests/repro/rf_issue_<id>.spec.ts`) via `InoculationGenerator::synthesize_for_issue`.
- **Autonomous Git PR Dispatch Engine (`replayfix-cli::git_packager`)**:
  - Issue-aware branch isolation (`replayfix/issue-<id>-<slug>`) and commit authoring embedding `Fixes #<id>` and Ed25519-signed RFC 6962 commit trailers (`ReplayFix-Issue`, `ReplayFix-Merkle-Root`, `ReplayFix-Signature`, `ReplayFix-Gate1-Status`, `ReplayFix-Gate2-Status`, `ReplayFix-Gate3-Status`).
- **Issue Evidence Commenter & Automated Triage Labels (`replayfix-core::github`)**:
  - `IssueCommenter` producing structured GitHub comments with executive verification summaries, keyframe bounding box previews, side-by-side DOM `ariaSnapshot` diffs, PR links, and offline `replayfix verify-receipt` commands.
  - Triage label automation (`TriageLabel::VerifiedHealed`, `TriageLabel::Unreproduced`, `TriageLabel::Ambiguous`).
- **Authenticated Webhook Server & Composite CI Action (`replayfix bot --listen`, `action.yml`)**:
  - Bounded HTTP daemon implementing RFC 2104 constant-time HMAC-SHA256 signature verification (`X-Hub-Signature-256`) returning HTTP 401 on authentication failures.
  - Anti-loop bot filter (`filter_webhook_event`, `<!-- replayfix-bot-comment -->`) ignoring bot accounts and self-authored comments.
  - Updated composite GitHub Action (`action.yml`) adding `issue-url`, `create-pr`, `comment-issue`, and `apply-labels` inputs.

---

## [1.6.0] - 2026-11-02

**Spec 160 — Sovereign Autonomous Healer: Multi-Turn Muse Spark Agent Loop & Sub-60s Macro Pipeline**: Fulfills Meta's core hackathon mandate ("Build Agent Loops with Muse Spark") by establishing **Meta Muse Spark 1.3** as the sovereign multi-turn agent loop cognitive orchestrator (`305 tok/s`, `reasoning: { effort: "xhigh" }`, `prompt_cache_key: "replayfix-v1.6-spark-agent-loop"`). Introduces the unified autonomous macro pipeline (`replayfix heal --video <path> --repo <path>`) executing the complete chain (Video Ingestion ➔ SAM 3.1 ➔ Voice Lock ➔ Ephemeral Server ➔ Spark Gate 1 RED ➔ Wonder AST Repair ➔ Spark Gate 2 GREEN ➔ Gate 3 Stress Quorum ➔ Permanent Inoculation ➔ RFC 6962 Signed Merkle Receipt) in **under 60 seconds**. Implements multi-turn encrypted deliberation caching (`reasoning.encrypted_content`) and `previous_response_id` backtracking for Meta Muse Wonder 1.0, preserving deep AST deliberation graphs across repair attempts upon compiler or assertion feedback. Upgrades `proof_viewer.html` with an interactive Turn-by-Turn Muse Spark Deliberation Timeline, side-by-side accessibility tree DOM diff inspector, and Merkle leaf verification. Full notes: [docs/release-notes/v1.6.0.md](docs/release-notes/v1.6.0.md).

### Added
- **Meta Muse Spark 1.3 Multi-Turn Agent Loop (`replayfix-core::deliberation`)**:
  - `DeliberationEngine`, `SparkDeliberationSession`, `SparkDeliberationTurn`, `DeliberationToolCall`, and `DeliberationToolObservation` managing multi-turn agent execution with sub-second turn latency and `prompt_cache_key: "replayfix-v1.6-spark-agent-loop"`.
  - Autonomous tool dispatch across 4 canonical tools: `replayfix_reproduce`, `replayfix_inspect_trace`, `replayfix_verify_patch`, and `replayfix_inoculate`.
- **Meta Muse Wonder 1.0 Encrypted Deliberation Backtracking (`replayfix-core::repair`, `replayfix-core::gateway`)**:
  - Threading of `previous_response_id` and cached `reasoning.encrypted_content` across iterative repair turns, preserving AST deliberation state across compiler diagnostics and flakiness feedback.
- **Sub-60s Sovereign End-to-End Macro Pipeline (`replayfix-cli::heal`, `replayfix heal`)**:
  - Single-command autonomous pipeline taking raw bug recording video through visual perception, dual-gate verification, stress quorum, permanent test inoculation, and signed Merkle receipt export in `< 60s`.
- **Hardened Offline Proof Portal Updates (`proof_viewer.html`, `replayfix-core::portal`)**:
  - Interactive Turn-by-Turn Deliberation Timeline tab rendering Muse Spark's thoughts, tool calls, and observations.
  - Side-by-side DOM diff inspector comparing pre-repair (Gate 1 RED) and post-repair (Gate 2 GREEN) Playwright `ariaSnapshot` accessibility trees.

---

## [1.5.1] - 2026-11-01

**Spec 151 — Live Meta Multimodal Wire Protocol & Full Muse Model Utilization**: Aligns ReplayFix's `--mode live-ai` cloud gateway (`crates/replayfix-core/src/gateway.rs`) and perception engine (`crates/replayfix-core/src/perception.rs`) with the production wire schemas of `https://api.meta.ai/v1/` across all 5 Meta models (`sam-3.1` `input_image` + `input_video` segmentation, `muse-voice-transcribe-1.0` RFC 7578 `multipart/form-data` WAV transcription, `muse-spark-1.3` `web_search` tool calling, `muse-wonder-1.0` / `muse-spark-1.3` autonomous repair, and `muse-image-1.0` `/v1/images/generations` badge synthesis). Full notes: [docs/release-notes/v1.5.1.md](docs/release-notes/v1.5.1.md).

### Changed
- **`MetaGatewayClient` & `SamSegmenter` (`replayfix-core::gateway`, `replayfix-core::perception`)**:
  - Formatted multimodal image and video segmentation payloads as `input_image` (`data:image/png;base64,...`) and `input_video` (`data:video/mp4;base64,...`) content blocks on `POST /v1/responses` (`sam-3.1`), and updated `SamSegmenter::parse_wire_tokens` to support 6-attribute `<|box;x1=...;y1=...;x2=...;y2=...;w=...;h=...|>` tokens.
  - Updated `transcribe_audio_wav` to dispatch RFC 7578 `multipart/form-data` (`request={"model":"muse-voice-transcribe-1.0","audioEncoding":"WAV"}` + `audio` WAV binary part) to `POST /v1/asr/transcribe` and parse `audioDurationMs`.
  - Added `query_web_search_grounding` (`tools: [{"type": "web_search"}]` on `muse-spark-1.3`) and `generate_verification_badge` (`muse-image-1.0` on `POST /v1/images/generations`).

---

## [1.5.0] - 2026-11-01

**Spec 150 — Permanent Inoculation, Agentic Protocol (MCP) & Continuous CI/CD Automation**: Evolves ReplayFix into a universal verification, permanent codebase inoculation ("Dreaming"), and Model Context Protocol (MCP) platform across six architectural pillars. Introduces standalone `@playwright/test` spec synthesis (`tests/repro/rf_<id>.spec.ts`) and a tamper-evident `.replayfix/inoculations.json` registry (`contracts/inoculation.json`), a native JSON-RPC 2.0 MCP server (`replayfix mcp --transport stdio|sse`, `contracts/mcp_tools.json`), autonomous Git branch and Evidence Matrix PR packaging (`--package-pr` with Ed25519 + RFC 6962 commit trailers), Gate 3 Concurrency Perturbation & Stress Quorum (`--stress-quorum` with 3G latency/500ms jitter and 3-pass concurrency quorum, quarantining flaky patches as `FLAKY_NON_DETERMINISTIC` Exit 21), Multi-Modal Copilot ASR keyframe alignment and interactive ambiguity tie-breaker ($|\Delta\text{score}| < 0.05$), and headless SARIF v2.1.0 (`results.sarif`) + JUnit XML (`junit.xml`) CI/CD reporters with a composite GitHub Actions `action.yml`. Full notes: [docs/release-notes/v1.5.0.md](docs/release-notes/v1.5.0.md).

### Added
- **Permanent Codebase Inoculation ("Dreaming") (`replayfix-core::inoculate`, `replayfix inoculate`)**:
  - `InoculationGenerator`, `InoculationSpec`, `InoculationEntry`, and `InoculationRegistry` synthesizing standalone `@playwright/test` TypeScript regression specs (`tests/repro/rf_<id>.spec.ts`) using grounded accessibility locators (`getByRole`, `getByText`, `getByTestId`) with zero positional selectors.
  - `InoculationGenerator::verify_inoculation_parity` ensuring every synthesized spec fails on the unpatched baseline and passes on the repaired state before recording in `.replayfix/inoculations.json`.
- **Model Context Protocol (MCP) Universal Server (`replayfix-cli::mcp`, `contracts/mcp_tools.json`)**:
  - `replayfix mcp --transport stdio|sse [--port <port>]` implementing JSON-RPC 2.0 over bounded `stdio` and Streamable HTTP / `SSE`.
  - Exposes 5 tools (`replayfix_reproduce`, `replayfix_verify_patch`, `replayfix_inoculate`, `replayfix_inspect_trace`, `replayfix_doctor`), 3 resources (`replayfix://receipts/{task_id}`, `replayfix://sessions/{session_id}`, `replayfix://benchmarks`), and the `triage_bug` prompt.
- **Autonomous Git Branch & PR Packaging Engine (`replayfix-cli::git_packager`)**:
  - `GitPrPackager` and `CommitTrailers` isolating repairs onto `replayfix/fix-<bug_id>` branches, signing commits with Ed25519 + RFC 6962 Merkle trailers (`ReplayFix-Verified`, `ReplayFix-Merkle-Root`, `ReplayFix-Signature`), and emitting `pr_description.md` with keyframe bounding boxes and side-by-side DOM `ariaSnapshot` diffs.
- **Gate 3 Concurrency Perturbation & Stress Quorum (`replayfix-core::stress`)**:
  - `StressQuorumRunner`, `PerturbationProfile`, and `StressQuorumReport` executing network latency/jitter perturbation (`SET_NETWORK_PERTURBATION`) and a 3-pass randomized-interval concurrency quorum, quarantining flaky patches with `TriageStatus::FlakyNonDeterministic` (Exit Code 21).
- **Multi-Modal Copilot & Ambiguity Picker (`replayfix-core::locator`, `replayfix-cli::shell`)**:
  - `LocatorResolver::detect_ambiguous_tie` and `resolve_with_tie_breaker` prompting an interactive terminal candidate picker when $|\Delta\text{score}| < 0.05$ (`500` bps) in interactive sessions while failing closed (`AMBIGUOUS_GROUNDING`, Exit 22) in headless runs.
  - ASR narration + keyframe timestamp alignment (`/trace`) and live `SubstrateEvent::CdpTelemetry` streaming.
- **CI/CD Action & Headless Reporters (`replayfix-core::reporters`, `action.yml`)**:
  - `SarifReporter` (SARIF v2.1.0 `results.sarif`), `JunitReporter` (`junit.xml`), `--sarif` / `--junit` flags on `repro` and `sweep`, and composite GitHub Action `action.yml`.

---

## [1.4.0] - 2026-11-01

**Spec 140 — Agent Experience Revolution**: Major transformation from a traditional flags-only CLI utility into a conversational, human-centered **Agentic Terminal Workspace**. Introduces an interactive conversational shell (`replayfix chat` / default interactive `replayfix`), natural language intent parser, 13-state observable agent lifecycle, context-aware tiered permissions (Low, Medium, High risk gates with session-level persistence), real-time reactive execution feedback with live spinners and duration timers, an interactive virtualized hunk diff reviewer (`j/k`, `Space`, `Tab` navigation), and durable multi-turn session persistence with checkpoint resumption. All existing CLI flags and 241+ tests maintain 100% backward compatibility. Full notes: [docs/release-notes/v1.4.0.md](docs/release-notes/v1.4.0.md).

### Added
- **Interactive Conversational Shell (`replayfix-cli::shell`, `replayfix-cli::intent`)**:
  - `AgentShell` REPL launched when `replayfix` is executed without arguments in an interactive TTY, or via `replayfix chat` / `replayfix agent`.
  - `IntentParser` translating conversational English (e.g. *"Fix the cart quantity bug in metapulse"*, *"Inspect RF-03 with live browser"*, *"Run doctor check"*) and slash commands (`/repro`, `/doctor`, `/sweep`, `/verify`, `/proof`, `/status`, `/history`, `/resume`, `/exit`) into structured `AgentIntent`s.
  - `ExecutionPlan` card generator formulating explicit, step-by-step verification plans with risk ratings prior to execution.
  - Contextual clarification dialogues when natural language input is ambiguous.
- **Real-Time Substrate Event Pipeline (`replayfix-core::observer`, `replayfix-cli::tui::spinner`)**:
  - `SubstrateObserver` trait and `SubstrateEvent` stream capturing phase transitions, tool invocations, grounding candidate evaluations, and gate outcomes in real-time.
  - `StatusSpinner` in-place reactive terminal status line with animated Braille frames, phase indicators, and millisecond timers that reflect substrate events without screen flicker.
- **Context-Aware Tiered Permission Engine (`replayfix-cli::permissions`, `replayfix-cli::tui::dialog`)**:
  - `PermissionManager` categorizing actions into `Low` (auto-granted), `Medium` (ephemeral dev server / external AI API; `Allow Once` or `Allow for Session`), and `High` (base repo patch mutation requiring explicit affirmative review).
  - High-risk security boundary dialogs displaying affected files, insertions/deletions, and SHA-256 digests.
- **Interactive In-Terminal Diff Reviewer (`replayfix-cli::tui::diff_view`, `replayfix-cli::tui::screen`)**:
  - `DiffReviewer` providing a keyboard-navigable (`j/k`, `Space`, `d/u`, `Home/End`, `a`, `r`, `q`) virtualized diff viewer with ANSI syntax highlighting and viewport bounding.
  - `ScreenGuard` RAII raw mode and alternate screen buffer manager with guaranteed cursor restoration on drop or cancellation.
- **Session Lifecycle & Resumption (`replayfix-cli::session`)**:
  - `AgentSession` tracking multi-turn conversation logs, active task context, and linking with `SessionStateCheckpoint` in `.replayfix/session_state.json`.
  - Resumption of interrupted workflows via `/resume` or `AgentIntent::ResumeSession`.

---

## [1.3.0] - 2026-11-01

**Spec 130 — Substrate Truth & Universal SPA Grounding**: Enforced execution-derived provenance and billing integrity in `audit_receipt.json`, added Universal SPA **Attached Mode** (`--url <endpoint> --repo <path>`) and **Managed Mode** (`--repo <path>` via `ManagedSpaServer` with dynamic `:0` port capture, 30s health polling, and RAII process-tree teardown), and decoupled real workspaces (`benchmarks/metapulse/`) from synthetic HTML fixture fallbacks. Full notes: [docs/release-notes/v1.3.0.md](docs/release-notes/v1.3.0.md).

### Added
- **Universal SPA Environment Manager (`replayfix-core::environment`, `replayfix-cli`)**:
  - `ManagedSpaServer` and `SpaToolchain` auto-detecting `vite`, `next`, `react-scripts`, and Node dev scripts from `package.json`, spawning child dev servers on dynamic port `:0`, capturing bound ports from stdout/stderr, polling HTTP readiness with a 30s timeout, and terminating child process trees via `Drop` (`Platform::terminate_process_tree`).
  - `AttachedSpaEndpoint` and `--url <endpoint>` CLI option connecting Playwright CDP directly to externally running application servers, synchronizing candidate patches to `--repo` during Gate 2, and rolling back on failure.
- **Execution-Derived Provenance & Billing Integrity (`replayfix-cli`, `replayfix-core::engine`)**:
  - Rejects any combination of `--mode live-ai` with mock flags (`--mock-syntax-retry`, `--mock-failing-repair`, `--mock-pkg-test-mutation`, or `--mock-patch != "builtin"`) with `ERR_USAGE` (Exit Code `2`).
  - Initializes `AuditReceipt.provenance` to `Some("deterministic-fixture")` and guarantees `model_provenance: null` and `token_accounting: null` on all offline/fixture executions.
- **Runnable MetaPulse SPA Workspace (`benchmarks/metapulse/`, `replayfix-core::fixtures`)**:
  - Added runnable `package.json`, `server.js`, `public/index.html`, and `public/app.js` under `benchmarks/metapulse/` and removed synthetic `#btn-publish-quote` fallback elements from `RF01_INDEX_HTML`.

---

## [1.2.0] - 2026-10-31

**Spec 120 — Realtime Multimodal Perception (Live Meta SAM 3.1, Muse Voice 24 kHz ASR & Multi-Action Trajectory)**: Wired live outbound HTTPS inference for `sam-3.1` (`SamSegmenter::segment_live`, `MetaGatewayClient::send_sam_segmentation`), `muse-voice-transcribe-1.0` (`AsrTranscriber::transcribe_live`, `MetaGatewayClient::send_asr_transcription` with 24 kHz mono PCM WAV validation), and `BoundedRepairClient` / `ModelPool`, added multi-step `ActionTrajectory` extraction (`RoiDiffer::detect_trajectory`), enforced honest offline provenance (`local_diff_heuristic` / `simulated_fixture`), and added `.env` + Windows Registry (`HKCU\Environment`) credential resolution. Full notes: [docs/release-notes/v1.2.0.md](docs/release-notes/v1.2.0.md).

### Added
- **Live Meta SAM 3.1 Segmentation (`replayfix-core::gateway`, `replayfix-core::perception`)**:
  - `SamWirePayload`, `MetaGatewayClient::send_sam_segmentation`, and `SamSegmenter::segment_live` parsing `<|box;x1=...;y1=...;x2=...;y2=...|><|mask;...|>` tokens and snapping to calibrated CSS viewport centroids verified via CDP `document.elementFromPoint`.
- **Live Muse Voice 24 kHz PCM ASR (`replayfix-core::gateway`, `replayfix-core::perception`)**:
  - `AsrPayload` (`PCM_24KHZ`, `ENDPOINTING`, `DELTA`), 44-byte RIFF/WAVE header verifier, `MetaGatewayClient::send_asr_transcription`, and SSE/JSON `AsrTranscriber::parse_asr_events`.
- **Multi-Action Trajectory Engine (`replayfix-core::video`, `replayfix-core::perception`)**:
  - `RoiDiffer::detect_trajectory`, `ActionTrajectory`, and `UserAction` (`click`, `input`, `navigate`) across multi-step defect recordings (`RF-01_quote_repost_crash_with_cursor.mp4`).
- **Global & `.env` Credential Resolution (`replayfix-core::gateway`)**:
  - Automatic `META_API_KEY` / `REPLAYFIX_AI_KEY` lookup across process env, `.env` files (`KEY="val"`, `$env:KEY = "val"`, `export KEY="val"`), and Windows Registry (`HKCU\Environment` and `HKLM`).

---

## [1.1.0] - 2026-10-30

**Spec 110 — Real MetaPulse 20-Bug Benchmark Suite, Authenticated Meta AI HTTPS Gateway & Dual Execution Modes**: Migrated all 20 real MetaPulse `.mp4` cursor recordings (`RF-01` through `RF-20`) with immutable `benchmarks/metapulse/MANIFEST.json`, added `MetaGatewayClient` (`reqwest` + `rustls-tls`, 30s connect / 90s read timeouts, `<UNTRUSTED_DOM_CONTEXT>` escaping, Llama-3.2/3.3 fallbacks), exposed `--mode <live-ai | deterministic-fixture>`, `--model-intent`, and `--model-repair` CLI flags, and generated the Option B real MetaPulse demonstration video in `release/v1.1.0/`. Full notes: [docs/release-notes/v1.1.0.md](docs/release-notes/v1.1.0.md).

### Added
- **20-Bug MetaPulse Benchmark Suite (`benchmarks/metapulse/`, `replayfix-core::fixtures`)**:
  - 20 real `.mp4` cursor defect recordings (`RF-01`..`RF-20`), `BENCHMARK_METADATA.md`, `MANIFEST.json` with SHA-256 verification (`MetaPulseManifest::verify_all_videos`), and `MetaPulseRepoRunner` (`https://github.com/aroucadi/MetaPulse.git`).
- **Production HTTPS `MetaGatewayClient` (`replayfix-core::gateway`)**:
  - Pure-Rust TLS (`reqwest` + `rustls-tls`) client with 30s connect / 90s read timeouts, `META_API_KEY` / `REPLAYFIX_AI_KEY` Bearer auth, HTTP 429/503 exponential backoff, `<UNTRUSTED_DOM_CONTEXT>` XML escaping, and fallback routing to `meta-llama/Llama-3.2-11B-Vision-Instruct` and `meta-llama/Llama-3.3-70B-Instruct`.
- **Dual Execution Modes (`replayfix-cli`, `replayfix-core::receipt`)**:
  - Added `--mode <live-ai | deterministic-fixture>`, `--model-intent`, and `--model-repair`, failing closed with Exit Code `30` (`ERR_GATEWAY_AUTH_MISSING`) when `live-ai` is invoked without credentials, and recording `provenance` and `model_provenance` in `audit_receipt.json`.

---

## [1.0.0] - 2026-10-30

**Grand Finale — Final Production Release & Hackathon Submission (`Spec 100`)**: Optimized LTO production Rust binary (`replayfix.exe` v1.0.0), Windows 11 Authenticode code signing and verification, synchronized 180-second Three-Act demonstration video (`replayfix-v1.0.0-demo.mp4`) with `demo-evidence-manifest.json` and WebVTT captions, updated comprehensive documentation (`README.md`), and complete `release/v1.0.0/` GitHub & Meta Hackathon submission package. Full notes: [docs/release-notes/v1.0.0.md](docs/release-notes/v1.0.0.md).

### Added
- **Production Distribution & Authenticode Signing (`release/v1.0.0/binaries/windows-x64/`)**:
  - Compiled `replayfix.exe` (`v1.0.0`) with `[profile.release]` (`opt-level = 3`, `lto = true`, `codegen-units = 1`, `panic = "abort"`), signed via `scripts/sign-binary.ps1`, and verified with `Get-AuthenticodeSignature` (`SIGNATURE-VERIFICATION.txt` & `SHA256SUMS.txt`).
- **Synchronized Three-Act Judge Demonstration (`release/v1.0.0/demo/`)**:
  - Generated 180.0-second (`1920x1080`, `30fps`, H.264/AAC) 3-act video `replayfix-v1.0.0-demo.mp4`, WebVTT captions (`captions/replayfix-v1.0.0-demo.vtt`), and `demo-evidence-manifest.json` grounded in real `v1.0.0` executions of Act 1 (Grounded Defect `RF-01`), Act 2 (No-Op `Exit 30` & Cheating `Exit 11` rejection), and Act 3 (Verified Repair, 5-Bug Sweep & `proof_viewer.html`).
- **Complete Submission & Documentation Suite (`release/v1.0.0/`, `README.md`)**:
  - Authored `INSTALLATION.md`, `QUICKSTART.md`, `SECURITY.md`, `VERIFICATION.md`, `HACKATHON_SUBMISSION.md`, `RELEASE_NOTES.md`, and `QUALIFICATION_REPORT.md`, and updated root `README.md` for `v1.0.0`.

---

## [0.9.5] - 2026-10-30

Spec 095 — Offline Judge Proof Portal (`proof_viewer.html`), Client-Side RFC 8785 / RFC 6962 / RFC 8032 Cryptographic Verifier, Bounded Live Meta Multimodal Cloud Gateway (`replayfix-core::gateway`), Token-Bucket Rate Limiter, 3-State Circuit Breaker with Baseten Open-Weight Fallback, Zero-Audio Rule Enforcement, and `replayfix export-proof` CLI. Full notes: [docs/release-notes/v0.9.5.md](docs/release-notes/v0.9.5.md).

### Added
- **Self-Contained Offline Judge Proof Portal (`proof_viewer.html`, `replayfix-core::portal`)**:
  - Single-file zero-dependency HTML5/CSS3/ES2022 proof inspector with strict air-gapped CSP (`default-src 'none'; connect-src 'none'`).
  - Client-side RFC 8785 UTF-16 canonical JSON serializer, RFC 6962 SHA-256 leaf (`0x00`) and internal node (`0x01`) Merkle verifier, and pure RFC 8032 Ed25519 signature verifier.
  - Six honest verification states (`VERIFIED GENUINE`, `SIGNATURE VALID — SIGNER UNTRUSTED`, `INTEGRITY CHECK FAILED`, `UNSIGNED RECEIPT`, `UNSUPPORTED PROTOCOL VERSION`, `MALFORMED RECEIPT`).
  - Interactive inspector tabs for Merkle Proof Exploration, Ed25519 Attestation & Pinned Trust Store, Multimodal Video/DOM/SAM 3.1 Spatial Grounding, Unified Diff & Active Regression Parity ($F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$), and 5-Bug Benchmark Sweep (`RF-01`..`RF-05`).
- **Bounded Live Multimodal Cloud Gateway (`replayfix-core::gateway`)**:
  - `MultimodalGateway` supporting Meta Model API (`muse-wonder-1.0`, `muse-spark-1.3`, `sam-3-1`, `muse-voice-transcribe-1.0`) and Baseten fallback (`meta-models/muse-spark-open`).
  - Token-bucket `RatePool` (RPM/TPM/in-flight), HTTP 429 `Retry-After` + exponential backoff with jitter, 3-state `CircuitBreaker` (`Closed`, `Open`, `HalfOpen`), Zero-Audio Rule enforcement, 24 kHz mono WAV validation, and credential redaction (`[REDACTED]`).
- **CLI Proof Exporter (`replayfix-cli`)**: Added `replayfix export-proof [--output <dir>] [--json]`.

---

## [0.9.1] - 2026-10-29

Independent Zero-Trust Qualification remediation for Spec 090: 4-Layer Cryptographic Trust Verification (Layer 1 on-disk artifact integrity, Layer 2 strict RFC 6962 tree-size/audit-path invariants, Layer 3 pure Ed25519 signature verification, Layer 4 public-key trust policy), RFC 8785 UTF-16 code-unit canonical JSON sorting, complete `audit_receipt.json` & `sweep_receipt.json` attestation binding, atomic key & sweep persistence, and bounded concurrent `ModelPool` execution. Full notes: [docs/release-notes/v0.9.1.md](docs/release-notes/v0.9.1.md).

### Added
- **4-Layer Zero-Trust Receipt & Sweep Verifier (`replayfix-core::receipt`, `replayfix-cli`)**: `AuditReceipt::verify_receipt_and_attestation` now performs end-to-end verification across all 4 trust layers:
  - **Layer 1 (On-Disk Evidence Integrity)**: Re-reads every referenced artifact file in `<dir>/artifacts/<file_name>` on disk, verifies exact `size_bytes` and `content_sha256`, cross-checks top-level `audit_receipt.json` core fields against the committed `04_audit_receipt.json` leaf, and fails closed (`MERKLE_PROOF_INVALID`, exit code 40) if any file is tampered with or if `merkle_receipt.json` is deleted.
  - **Layer 2 (Strict RFC 6962 Merkle Proofs)**: Enforces `tree_size == leaves.len()`, `proof.tree_size == receipt.tree_size`, `proof.leaf_index == idx`, unique leaf `file_name` entries, and empty remaining `audit_path` at `size == 1` (`verify_node`).
  - **Layer 3 & Layer 4 (Ed25519 & Signer Trust Policy)**: Verifies pure Ed25519 signatures over `REPLAYFIX-ATTESTATION:v0.9:{schema}:{root}:{size}:{timestamp}` and supports direct cryptographic verification of `.replayfix/sweep_receipt.json` against per-bug receipts on disk.
- **Persistent & Environment-Driven Ed25519 Key Management (`replayfix-core::crypto`)**: Added `KeypairManager::load_or_generate` following `ADR-03` priority (`REPLAYFIX_SIGNING_KEY` environment variable $\rightarrow$ `<output_dir>/keys/ed25519_signing.key` $\rightarrow$ atomic generation via `atomic_write_file`).
- **RFC 8785 Surrogate-Pair UTF-16 Key Ordering Test (`replayfix-core::digest`)**: Enforced UTF-16 code-unit key sorting (`a.encode_utf16().cmp(b.encode_utf16())`) in `canonical_json` with regression coverage for supplementary-plane surrogate pairs (`\u{10000}` vs. `\u{E000}`).

### Fixed
- **Top-Level `audit_receipt.json` Attestation Completeness (`replayfix-core::receipt`, `replayfix-cli`)**: `AuditReceipt::write_disaggregated_artifacts` now populates `self.merkle_root` and `self.attestation` and atomically updates `<output_dir>/audit_receipt.json` across both single-bug and 5-bug sweep runs.
- **Sweep Receipt Schema & Isolation Hardening (`replayfix-core::sweep`, `contracts/sweep_receipt.schema.json`)**: Aligned `contracts/sweep_receipt.schema.json` with `SweepReceipt` (removing non-canonical float fields), added pre/post `WorktreeManifest` $\Delta\text{repo}=0$ verification across the entire sweep, wired `options.live_browser`, hashed final on-disk per-bug receipts into the sweep Merkle root, and persisted `sweep_receipt.json` via `atomic_write_file`.
- **Bounded Concurrent `ModelPool` Execution (`replayfix-core::repair`)**: Upgraded `ModelPool::race_candidates` to dispatch candidate generation across bounded scoped worker threads (`max_concurrent_tasks`) with per-task timeout enforcement (`timeout_per_task`).

---

## [0.9.0] - 2026-10-29

RFC 6962 Merkle receipts, pure Ed25519 digital attestation, receipt verifier CLI (`verify-receipt`), authoritative 5-bug MetaPulse benchmark suite (`RF-01` to `RF-05`), tri-model competitive orchestration, and high-performance benchmark sweep CLI (`sweep`). Full notes: [docs/release-notes/v0.9.0.md](docs/release-notes/v0.9.0.md).

### Added
- **RFC 6962 Domain-Separated Merkle Trees (`replayfix-core::merkle`)**: Implements RFC 6962 compliant leaf ($0x00$) and internal node ($0x01$) hashing, deterministic power-of-2 binary tree construction, and reproducible audit path inclusion proofs for all disaggregated fact artifacts.
- **Pure Ed25519 Digital Attestation (`replayfix-core::crypto`)**: Hardware-accelerated Ed25519 signing and verification via `ed25519-dalek` with versioned canonical payload binding (`REPLAYFIX-ATTESTATION:v0.9:...`), public-key trust policies (`AllowAny`, `PinnedKeys`), and fail-closed exit code 40 (`RECEIPT_VERIFICATION_FAILED`).
- **Independent Receipt Verification CLI (`replayfix verify-receipt`)**: Universal CLI subcommand that verifies Merkle inclusion proofs, tree roots, and Ed25519 signatures across file and directory targets with optional `--trusted-key` enforcement.
- **Five-Bug MetaPulse Benchmark Suite (`replayfix-core::fixtures`)**: Implemented complete test fixtures for `RF-01` through `RF-05` covering null dereferences, state arithmetic bugs, inventory underflow, lifecycle reset on navigation, and double-submit idempotency races with unified diff fixes and behavioral contracts.
- **Tri-Model Orchestration & Pooling (`replayfix-core::repair`)**: `ModelPool` concurrent candidate dispatch across `muse-spark-1.3`, `muse-wonder-1.0`, and `muse-spark-1.3-contributor` with atomic cancellation upon first-verified-green qualification.
- **High-Performance Benchmark Sweep CLI (`replayfix sweep`)**: Automated execution of `RF-01` through `RF-05` across isolated ephemeral scratch sandboxes, emitting `.replayfix/sweep_receipt.json` with aggregate pass/fail accounting, aggregate Merkle root, and digital signature in $< 2$ seconds ($< 60$s ceiling).

### Changed
- **Schema Version Harmonization (`0.9`)**: Bumped canonical `SCHEMA_VERSION` to `"0.9"` across receipts, triage reports, differential matrices, regression evaluators, and CLI spec generators.
- **Disaggregated Artifact Pipeline (`replayfix-core::receipt`)**: Emits `merkle_receipt.json` and `attestation.json` alongside disaggregated stage artifacts.

---

## [0.8.2] - 2026-10-29

Comprehensive remediation of identified architectural gaps, schema version harmonization, and write atomicity hardening across the observability and ingestion pipeline. Full notes: [docs/release-notes/v0.8.2.md](docs/release-notes/v0.8.2.md).

### Added
- **Unified Atomic Artifact Pipeline (`replayfix-core::artifacts`)**: Integrated `ArtifactStore` and `atomic_write_file` across all artifact and spec writers, providing two-phase atomic (`.tmp` -> `rename`) writes with unique PID/counter suffixes and defensive error cleanup.
- **Artifact Traversal Protection (`replayfix-core::artifacts`)**: Enforced strict `validate_file_name` in `ArtifactStore`, blocking path traversal and directory separators.
- **Worktree Mutation Atomicity (`replayfix-core::patch`)**: Hardened `UnifiedPatcher::apply_to_worktree` to apply patches atomically via `atomic_write_file`, eliminating partial or truncated file writes on disk interruption.
- **Custom Output Directory Consistency (`replayfix-cli::approval`)**: Fixed `write_approval_audit` to consistently place `approval_audit.json` in `<output_dir>/artifacts` across custom and default directories.
- **Dynamic Harness User-Agent (`replayfix-core::ingestion`)**: Dynamically resolves harness package version via `CARGO_PKG_VERSION` to eliminate version drift in GitHub API interactions.

### Changed
- **Schema Version Harmonization (`0.8`)**: Bumped `SCHEMA_VERSION` to `"0.8"` across `receipt.rs`, `matrix.rs`, `regression.rs`, and `freeze-spec`, resolving specification alignment gaps.

---

## [0.8.1] - 2026-10-29

Security hardening, terminal injection defense, authenticated issue ingestion, and specification harmonization for v0.8.x baseline qualification freeze. Full notes: [docs/release-notes/v0.8.1.md](docs/release-notes/v0.8.1.md).

### Added
- **Adversarial Prompt Injection Defense (`replayfix-core::ingestion`)**: Entity-escapes embedded `<UNTRUSTED_ISSUE_METADATA>` and `</UNTRUSTED_ISSUE_METADATA>` delimiters within issue titles, reproduction steps, and raw markdown bodies.
- **Authenticated GitHub Issue Retrieval (`replayfix-core::ingestion`)**: Supports `GH_TOKEN` / `GITHUB_TOKEN` environment variables to prevent rate limiting (60/hr) and access private issues without credential leakage.
- **Terminal Injection Sanitization (`replayfix-cli::approval`)**: Neutralizes raw ANSI control characters (`\x1b` -> `^[`) in diff previews, preventing terminal hijack vulnerabilities.
- **Interactive EOF Fail-Closed Handling (`replayfix-cli::approval`)**: Ensures non-affirmative EOF triggers fail-closed `ManualNegative` rejection.

### Changed
- **Spec 080 Harmonization**: Synchronized acceptance criteria exit codes in `specs/080-v0.8-observability-ingestion/spec.md` with Section 4 mapping (12, 13, 23, 24).

---

## [0.8.0] - 2026-10-29

Observability suite, live GitHub issue ingestion, interactive human confirmation gate, and disaggregated evidence artifacts for Spec 080. Full notes: [docs/release-notes/v0.8.0.md](docs/release-notes/v0.8.0.md).

### Added
- **Playwright Trace Auto-Launcher (`--trace`)**: Captures complete Playwright execution traces and launches the interactive visual timeline scrubber (`npx playwright show-trace`) upon run completion, automatically skipping GUI spawn in headless CI and non-interactive sessions.
- **Live GitHub Issue Ingestion (`--issue <url>`)**: Secure retrieval of public GitHub issues via HTTPS curl or gh CLI, extraction of reproduction steps into task manifests, and enclosing untrusted descriptions in `<UNTRUSTED_ISSUE_METADATA>` blocks.
- **Interactive Human Approval Gate (`replayfix_cli::approval`)**: Syntax-highlighted ANSI candidate diff preview requiring affirmative operator confirmation (`[y/N]`) before modifying base working trees, failing closed with exit code 13 (`NON_INTERACTIVE_REFUSAL`) in non-TTY environments unless `--yes` is provided.
- **Worktree Patch Application (`replayfix_core::patch`)**: Added `UnifiedPatcher::apply_to_worktree` to apply approved candidate patches directly to repository working trees.
- **Disaggregated Fact Artifacts**: Persists `candidate.patch`, `approval_audit.json`, and `issue_ingestion.json` alongside `differential_diagnosis.json` and `regression_parity.json` in `.replayfix/artifacts/`.

---

## [0.7.1] - 2026-10-29

Security hardening, regression wiring, and spec alignment for Spec 070. Full notes: [docs/release-notes/v0.7.1.md](docs/release-notes/v0.7.1.md).

### Added
- **Dedicated DiffGuard Module (replayfix_core::diffguard)**: Formalized DiffGuard with structural AST package.json test script inspection and test runner configuration protection (CONFIG_MUTATION_PROHIBITED, exit code 11).
- **Active Regression Parity Engine (replayfix_core::regression)**: Wired RegressionEvaluator::evaluate and capture_suite_snapshot into the repair engine, enforcing F_patched \setminus F_baseline = \emptyset and rejecting regressions with exit code 31 (REGRESSION_DETECTED).
- **Resumable State CheckpointManager (replayfix_core::checkpoint)**: Implemented CheckpointManager orchestrating stage progression, disk persistence of session_state.json, and staleness invalidation.
- **Codebase Inoculation Generation (replayfix-cli freeze-spec)**: Generates permanent Playwright regression tests at tests/repro/rf_{id}_regression.spec.ts with cryptographic aggregate SHA-256 headers.
- **Disaggregated Fact Artifact Suite (replayfix_core::receipt)**: Emits numbered fact artifacts (01_candidate.diff, 02_preflight.json, 03_differential_diagnosis.json, 04_regression_parity.json, 04_audit_receipt.json).
- **New CLI Flag --mock-regression**: Simulates candidate patch introducing regressions and asserts exit code 31.

---

## [0.7.0] - 2026-10-28

Spec-driven release for Spec 070 (`070-v0.7-differential-diagnosis-anticheat`). Full notes: [docs/release-notes/v0.7.0.md](docs/release-notes/v0.7.0.md).

### Added
- **Formal 6-State Differential Diagnosis Matrix (`replayfix_core::fsm::matrix`)**:
  - Implemented `DifferentialMatrix` evaluating baseline, clean reference, and 2-run patch stability.
  - Distinguishes `CAUSAL_DEFECT_CONFIRMED` (exit 0), `INVALID_TEST_OVERFIT` (exit 10), `QUARANTINE_INVERTED_BASELINE` (exit 32), `UNATTRIBUTED_SYMPTOM` (exit 21), `CLEAN_PASS_UNREPRODUCED` (exit 10), and `FLAKY_NON_DETERMINISTIC` (exit 21).
- **Cryptographic Anti-Cheat & Surgical `package.json` DiffGuard (`replayfix_core::repair`)**:
  - SHA-256 pre/post freeze on all test files and runner configs.
  - Byte-level surgical immutability on `package.json` `"scripts.test*"` keys with `CoreError::SecurityViolation("TEST_MUTATION_PROHIBITED")` (exit 11) while allowing safe non-test dependency/script edits.
- **Structured Regression Parity Engine (`replayfix_core::regression`)**:
  - `RegressionEvaluator` enforcing $F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$, $T_{\text{baseline}} \setminus T_{\text{patched}} = \emptyset$, and non-increasing test skip counts with `REGRESSION_DETECTED` (exit 31).
- **Resumable Session Checkpointing (`replayfix_core::checkpoint`)**:
  - Checkpoint state model conforming to `session_state.schema.json` with git HEAD, manifest hash, and TTL staleness rejection.
- **Receipt Schema 0.7 & Disaggregated Artifacts (`replayfix_core::receipt`)**:
  - Bumped schema version to `"0.7"` incorporating `differential_diagnosis` and `regression_parity`.
  - Automatically writes `candidate.patch`, `repro_spec.json`, `differential_diagnosis.json`, and `regression_parity.json` to `<output_dir>/artifacts/`.
- **CLI Subcommands & Verification Flags (`replayfix-cli`)**:
  - Added `replayfix freeze-spec --bug <ID>` subcommand.
  - Added `--clean-ref <REF>`, `--verify-matrix`, and `--mock-pkg-test-mutation` flags.

---

## [0.6.2] - 2026-10-28

Surgical packaging, test runner resiliency, and sandbox seeding patch for Spec 060 (`060-v0.6-meta-model-bounded-repair`). Full notes: [docs/release-notes/v0.6.2.md](docs/release-notes/v0.6.2.md).

### Packaging & Hardening
- **Git Packaging & CI Tracking (`.gitignore`, `scripts/check-panics.ps1`)**:
  - Un-ignored `.specify/scripts/` in `.gitignore` and added root `scripts/check-panics.ps1` for clean CI reproducibility.
  - Dynamically detects the workspace root by walking parent directories looking for `Cargo.toml`.
- **Windows Test Runner Resiliency (`crates/replayfix-cli/tests/cli.rs`)**:
  - Replaced `.expect()` in test process exit code helper with `out.status.code().unwrap_or(-1)` to eliminate false runner panics on Windows.
- **NTFS Directory Junction Exclusion in Sandbox Seeding (`replayfix_core::worktree`)**:
  - In `collect_entries` and `copy_tree`, evaluated `is_symlink_or_reparse_point` to guarantee directory junctions (`mklink /J`) are excluded during initial workspace cloning.
- **Comment & String Literal Stripping in Scope Parser (`replayfix_core::repair::BlockPatcher`)**:
  - Implemented `strip_comments_and_strings` in `BlockPatcher::open_enclosing_declarations` to blank out comments and string literals, preventing false enclosing scope detection.

---

## [0.6.1] - 2026-10-28

Adversarial security hardening and preflight reliability patch for Spec 060 (`060-v0.6-meta-model-bounded-repair`). Full notes: [docs/release-notes/v0.6.1.md](docs/release-notes/v0.6.1.md).

### Security & Hardening
- **Windows Reparse Points & NTFS Junction Defense (`replayfix_core::worktree`)**:
  - Implemented `is_symlink_or_reparse_point` via `FILE_ATTRIBUTE_REPARSE_POINT` (`0x0400`) on Windows to block sandbox escapes via directory junctions (`mklink /J`).
  - Added canonical sandbox root containment verification in `write_file`.
- **DOS Device & Trailing Dot/Space Path Sanitization (`replayfix_core::manifest`)**:
  - Hardened `assert_safe_path` against multi-dot DOS reserved device patterns (e.g., `CON.txt.bak`) and stripped trailing dots/spaces from path components.
- **Enclosing Declaration Verification & Overlapping Matches (`replayfix_core::repair::BlockPatcher`)**:
  - Validated that `touched_symbols` are either defined in `replace_block` or belong to the active enclosing declaration scope.
  - Added overlapping match detection to guarantee `search_block` matches uniquely without ambiguous multi-match overlaps.
- **Fail-Closed Fallback Token Accounting (`replayfix_core::repair::TokenAccounting`)**:
  - Corrected nano-dollar cost accumulation across mixed model fallback attempts, preserving micro-USD integrity without integer rounding error.
- **Preflight & Engine Fail-Closed Error Handling (`replayfix_core::engine`)**:
  - Dynamic `syntax_preflight_passed` state reflection in telemetry.
  - Replaced partial `if let` blocks with exhaustive `match` arms for security violation reporting.
- **CLI & Zero-Panic Invariants (`replayfix-cli`)**:
  - Added CLI argument validation rejecting `--reasoning-effort max` with exit code `2`.
  - Enforced `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented, clippy::unreachable))]` across crates.
  - Built lexical tokenizer into `.specify/scripts/check-panics.ps1` with `-SelfTest` suite.

---

## [0.6.0] - 2026-10-28

Delivers **Spec 060 (`060-v0.6-meta-model-bounded-repair`)**: Bounded autonomous Meta Muse repair (`muse-spark-1.3`, `muse-wonder-1.0`, `muse-spark-1.3-contributor`, and open-weights fallback `meta-models/muse-spark-open-1026`), anti-cheating SHA-256 test freeze (`TestFreezeGuard`), `<UNTRUSTED_CONTEXT>` prompt-injection data fencing with 24-hour static prefix caching (`replayfix-v0.6-harness-prefix-v1`), Meta Superpower #3 encrypted deliberation continuity (`reasoning.encrypted_content` + `previous_response_id`), strict 5-field JSON Schema output validation (`StructuredRepairPatch`), ambiguity-safe source-block patching (`BlockPatcher`), `node --input-type=module --check` syntax preflight (`SyntaxPreflight`), 2-run consecutive Gate 2 stability verification, bounded 3-attempt quarantine (`QUARANTINE_REPAIR_FAILED`, exit code `21`), and integer micro-dollar token accounting (`TokenAccounting`). Full notes: [docs/release-notes/v0.6.0.md](docs/release-notes/v0.6.0.md).

### Added
- **Anti-Cheating Test Freeze Guard (`replayfix_core::repair::TestFreezeGuard`)**:
  - Pre-repair SHA-256 capture of all protected test and runner config files (`tests/**`, `__tests__/**`, `*.spec.*`, `*.test.*`, `playwright.config.*`, `jest.config.*`, `vitest.config.*`, `tsconfig*.json`, `package.json`).
  - Rejects any patch targeting protected paths across case variants (`TESTS\Foo.SPEC.ts`) or path traversal (`src/../../tests/foo.spec.ts`) with `CoreError::SecurityViolation("TEST_MUTATION_PROHIBITED")` (exit code `11`), and re-verifies SHA-256 parity on disk after every patch.
- **Meta Muse Model Routing, Prompt Isolation & Deliberation Continuity (`replayfix_core::repair`)**:
  - `MuseModelSlug`, `ReasoningEffort`, and `MuseModelConfig` supporting `muse-spark-1.3` (`effort: "xhigh"`), `muse-wonder-1.0` (`effort: "high"`), `muse-spark-1.3-contributor` (`effort: "xhigh"`), and `meta-models/muse-spark-open-1026`, while explicitly rejecting `muse-wonder-full-1.0` and `effort: "max"`.
  - `PromptBuilder`: Static system prefix (`replayfix-v0.6-harness-prefix-v1`, `24h` retention) and `<UNTRUSTED_CONTEXT>` escaping.
  - `RepairRequest` & `BoundedRepairClient`: Requests `include: ["reasoning.encrypted_content"]` and threads `previous_response_id` + `encrypted_content` across attempts 1..=3.
- **Strict JSON Schema, Ambiguity-Safe Block Patcher & Syntax Preflight**:
  - `StructuredRepairPatch::decode_strict` (`#[serde(deny_unknown_fields)]`, rejecting markdown code fences).
  - `BlockPatcher::apply`: Exact single-match uniqueness check (`0` matches -> `PATCH_TARGET_NOT_FOUND`, $\ge 2$ matches -> `AMBIGUOUS_PATCH`) and `touched_symbols` verification.
  - `SyntaxPreflight::check_file`: Pipes patched target source to `node --input-type=module --check` via `run_bounded` inside `ScratchSandbox`.
- **2-Run Gate 2 Stability, Quarantine Exit Code `21` & Receipt Schema `"0.6"`**:
  - Bumped `SCHEMA_VERSION` to `"0.6"`, added `TriageStatus::QuarantineRepairFailed` (`"QUARANTINE_REPAIR_FAILED"`, exit code `21`), and added `repair_telemetry: Option<RepairTelemetry>` and `token_accounting: Option<TokenAccounting>` to `AuditReceipt` and `TriageReport`.
  - Added `--model <MODEL>`, `--mock-failing-repair`, and `--mock-syntax-retry` flags to `replayfix repro` / `replayfix run`.
  - Remediated SpecKit workflow artifacts for `050` and generated full SpecKit suite for `060` (`spec.md`, `plan.md`, `tasks.md`, `research.md`, `data-model.md`, `quickstart.md`, `checklists/requirements.md`, `contracts/repair_response.schema.json`).
  - Expanded automated test suite to 151 tests (125 core unit tests + 24 CLI integration tests + 2 CLI UI unit tests).

---

## [0.5.0] - 2026-10-27

Delivers **Spec 050 (`050-v0.5-video-telemetry-ingestion`)**: Multimodal video telemetry ingestion, bounded FFmpeg/FFprobe subprocess execution, cursor-bounded $120 \times 120\text{px}$ ROI $L_1$ optical-flow frame differencing, geometric calibration across resolution/letterbox/DPR, Meta Superpower #1 (`sam-3.1`) wire-token centroid snapping, Meta Superpower #2 (`muse-voice-transcribe-1.0`) 24 kHz mono PCM `audioProcessedMs` narration alignment with strict Zero-Audio enforcement on `muse-spark-1.3`, and end-to-end CDP grounding integration. Full notes: [docs/release-notes/v0.5.0.md](docs/release-notes/v0.5.0.md).

### Added
- **Bounded Subprocess Supervisor (`replayfix_core::process`)**:
  - `run_bounded` and `BoundedOutput` enforcing wall-clock deadlines via `mpsc::Receiver::recv_timeout`, stdout size caps, stderr tail ring drains, and automatic process-tree termination (`Platform::terminate_process_tree`) on timeout or overflow.
- **FFmpeg Video Prober, Geometric Calibration & Cursor-Bounded ROI Differencer (`replayfix_core::video`)**:
  - `VideoPreprocessor`: Container safety validation (`.mp4`, `.webm`, `.mov`, $\le 120\text{s}$), `ffprobe` JSON metadata inspection (`VideoMetadata`), bounded grayscale frame extraction, keyframe PNG writer (`MAX_KEYFRAMES = 10` to `.replayfix/keyframes/`), 24 kHz mono 16-bit PCM WAV extraction (`pcm_s16le`), and deterministic demo MP4 synthesis (`synthesize_demo_video`).
  - `GeometricCalibration`: Integer-math coordinate transformation from video pixel space to CSS logical pixel space accounting for resolution scaling, letterbox/pillarbox padding, and device-pixel-ratio (`dpr_milli`).
  - `RoiDiffer`: $120 \times 120\text{px}$ cursor-bounded ROI $L_1$ frame differencer (`InteractionEvidence`) isolating the click timestamp `t_ms` and rough hotspot `(x_video, y_video)` while failing closed (`AMBIGUOUS_GROUNDING`, exit code `22`) on static/noisy frames or competing peaks within `< 2_500` bps ($|\Delta S| < 0.25$).
- **Meta Multimodal Perception Engine (`replayfix_core::perception`)**:
  - **Meta Superpower #1 (`sam-3.1` - `SamSegmenter`)**: Parses `<|box;x1=...;y1=...;x2=...;y2=...|><|mask;...|>` wire tokens, computes contour centroids $\left(\frac{x_1+x_2}{2}, \frac{y_1+y_2}{2}\right)$, enforces the `< 2_500` bps fail-closed ambiguity guard, and projects centroids into CSS logical pixels (`SamSnapResult`).
  - **Meta Superpower #2 (`muse-voice-transcribe-1.0` - `AsrTranscriber`)**: Enforces the strict **Zero-Audio Rule** (blocking `muse-spark-1.3` from receiving audio with `CoreError::SecurityViolation`), validates 24 kHz mono PCM streaming config (`PCM_24KHZ`, `ENDPOINTING`, `DELTA`, `emit_audio_progress: true`), and aligns developer narration using cumulative `audioProcessedMs` (`NarrationCue`).
  - `VideoTelemetry::ingest`: End-to-end multimodal ingestion pipeline wired into `MockHealer::run_live_browser_in_repo` and CLI `replayfix run --video assets/fixtures/rf01_demo.mp4 --repo .`.
- **Receipt Schema `"0.5"` & Demo Video Fixture**:
  - Bumped `SCHEMA_VERSION` to `"0.5"` and added `video_telemetry: Option<VideoTelemetry>` to `AuditReceipt` (100% float-free for RFC 8785 canonical JSON).
  - Added deterministic `assets/fixtures/rf01_demo.mp4` (`1280x720`, 24 kHz mono audio, UI click transition at `(100, 100)..(220, 140)`).
  - Expanded automated test suite to 138 tests (115 core unit tests + 21 CLI integration tests + 2 CLI UI unit tests).

---

## [0.4.1] - 2026-10-27

High-Risk Systems Gate adversarial hardening release for Playwright CDP grounding, child process timeout enforcement, protocol correlation, and DOM TOCTOU safety. Full notes: [docs/release-notes/v0.4.1.md](docs/release-notes/v0.4.1.md).

### Fixed & Hardened
- **Deadline-Bounded Non-Blocking Stdout Reader (`PlaywrightBridge`)**: Replaced blocking `BufReader::read_line` on the child's OS pipe with a dedicated reader thread (`spawn_stdout_reader`), `read_bounded_line` (`4 MiB` cap), and `mpsc::Receiver::recv_timeout` (`DEFAULT_BRIDGE_STARTUP_TIMEOUT = 20s`, `DEFAULT_BRIDGE_COMMAND_TIMEOUT = 15s`, `SHUTDOWN_COMMAND_TIMEOUT = 1.5s`), immediately poisoning and terminating the child process tree on timeout.
- **Continuous Background `stderr` Drain (`spawn_stderr_drainer`)**: Eliminated OS pipe buffer backpressure deadlocks when Node.js/Chromium writes >64 KiB to `stderr` and surfaced captured `stderr` context in all timeout, EOF, and protocol errors.
- **Monotonic Request Correlation (`request_id`)**: Added `request_id: u64` envelope correlation to `PlaywrightBridge` and `PLAYWRIGHT_BRIDGE_JS`, discarding late responses from prior requests (`resp_id < req_id`) and failing closed on mismatched future IDs (`resp_id > req_id`) or malformed JSON.
- **Browser WebSocket Disconnect, Target Crash & CDP Timeouts (`PLAYWRIGHT_BRIDGE_JS`)**: Added `browser.on('disconnected')`, `page.on('crash')`, `page.on('close')` tracking, `ensureActivePage()` health assertions, and `withTimeout` promise race guards across all Playwright/CDP calls.
- **DOM Node TOCTOU & Duplicate-Selector Ambiguity Guard (`LocatorResolver` & `CLICK_AND_OBSERVE`)**:
  - `LocatorResolver::resolve` now rejects selectors that match `> 1` grounded DOM candidates on the page (`Ambiguous grounding ... matches N distinct DOM elements`).
  - `LocatorResolver::synthesize_selector_for_grounding` and `execute_browser_gate` only synthesize/use `page.getByRole(...)` when `aria_verified == true` and fall back to `page.getByTestId(...)` otherwise.
  - `CLICK_AND_OBSERVE` verifies `locator.count() === 1` and non-zero `locator.boundingBox()` before clicking, surfacing `DOM_NODE_DETACHED` or `AMBIGUOUS_GROUNDING` immediately.
- **Idempotent Single-Termination Shutdown (`PlaywrightBridge::shutdown`)**: Added `shut_down` and `terminated` state guards and bounded `try_wait` reaping (`500ms`) so `Platform::terminate_process_tree` is called at most once per child instance.
- **Concurrent Per-Connection HTTP Handling (`SandboxServer`)**: Spawned short-lived per-connection threads with a 250ms loopback read timeout and 32 MiB file size cap (`MAX_HTTP_FILE_BYTES`) so idle TCP sockets cannot stall concurrent asset requests or server shutdown.

---

## [0.4.0] - 2026-10-27

Delivers **Spec 040 (`040-v0.4-playwright-cdp-grounding`)**: Playwright CDP spatial-semantic grounding, accessibility tree (`ariaSnapshot`) DOM Grounding Invariant verification, fail-closed ambiguity abstention (`AMBIGUOUS_GROUNDING`, exit code `22`), line-delimited JSON-lines stdio IPC (`PlaywrightBridge`), and live Chromium two-gate verification (`--live-browser` and `--headed`). Full notes: [docs/release-notes/v0.4.0.md](docs/release-notes/v0.4.0.md).

### Added
- **Spatial-Semantic Locator Grounding (`replayfix_core::locator`)**:
  - `BoundingBox`, `DomCandidate`, `GroundedTarget`, `GroundingTelemetry`, and `LocatorResolver`.
  - Integer basis-point composite scoring (`0..=10_000` bps) combining spatial containment (`4000` bps), semantic accessibility identity (`4000` bps), and DOM hierarchy specificity (`0..=2000` bps) for full RFC 8785 canonical JSON compatibility.
  - **DOM Grounding Invariant**: Cross-checks every synthesized semantic locator (`getByRole` / `getByTestId`) against Playwright's live `ariaSnapshot` YAML before execution.
  - **Strict `.first()` Ban**: Rejects positional disambiguators (`.first()`, `.nth()`, `:nth-match`).
  - **Fail-Closed Ambiguity Guard**: Abstains when the top two candidates score within $|\Delta S| < 0.25$ (`< 2500` bps), emitting `triage_report.json` with `status: "AMBIGUOUS_GROUNDING"` (exit code `22`).
- **Ephemeral Sandbox HTTP Server & Playwright Bridge (`replayfix_core::browser`)**:
  - `SandboxServer`: Serves isolated `ScratchSandbox` files over `127.0.0.1:<ephemeral_port>` with path-traversal, Windows drive-letter prefix, and symlink defenses (`403 Forbidden`).
  - `PlaywrightBridge`: Spawns a Node.js child process speaking flushed line-delimited JSON (`\n`) over `stdin`/`stdout` (`START_BROWSER`, `PROBE_PAGE`, `CLICK_AND_OBSERVE`, `SHUTDOWN`), automatically resolving Playwright from local `node_modules` or npm `_npx` caches and registering its process tree for deterministic cleanup.
- **Live Browser Two-Gate Verification (`--live-browser` & `--headed`)**:
  - `MockHealer::run_live_browser_in_repo` and `LiveBrowserOptions` in `replayfix_core::engine` driving Gate 1 (RED runtime `TypeError` in Chromium) -> disk patch in `ScratchSandbox` -> Gate 2 (GREEN DOM assertion in Chromium) under identical SHA-256 behavioral contracts and $\Delta \text{repo} = 0$ verification.
  - Added `--live-browser` and `--mock-ambiguous` flags to `replayfix repro` (with `--headed` launching visible Chromium).
- **Receipt Schema `"0.4"`**:
  - Bumped `SCHEMA_VERSION` to `"0.4"`, added `SUBSTRATE_LIVE_BROWSER` (`"playwright-cdp"`), `TriageStatus::AmbiguousGrounding` (exit code `22`), `TriageStatus::BridgeExecutionFailed` (exit code `20`), and `grounding_telemetry: Option<GroundingTelemetry>` on `AuditReceipt`.
- **Spec Kit Artifacts & Expanded Test Suite**:
  - Complete specification, plan, research, data model, quickstart, checklist, and task breakdown under `specs/040-v0.4-playwright-cdp-grounding/`.
  - Expanded automated test suite to 107 tests (86 core unit tests + 19 CLI integration tests + 2 CLI UI unit tests).

### Changed
- Bumped workspace crate versions in `replayfix-core` and `replayfix-cli` from `0.3.3` to `0.4.0`.
- Extended `Fixture::rf01()` with `index_html`, `click_x: 160`, `click_y: 120`, `viewport_width: 1280`, `viewport_height: 720`, and `expected_output_text: "Items: 0"`.

### Security
- `SandboxServer` binds strictly to `127.0.0.1` loopback and rejects path traversal (`../`), Windows drive-letter prefixes (`GET /C:/...`), and symlinks with `403 Forbidden`.
- `LocatorResolver` enforces the DOM Grounding Invariant against `ariaSnapshot` and bans `.first()` / `:nth-match` selectors.

---

## [0.3.3] - 2026-10-27

v0.3 Release Quality Gate qualification, workspace version harmonization, and BeautifulRust verification. Full notes: [docs/release-notes/v0.3.3.md](docs/release-notes/v0.3.3.md).

### Changed
- **Workspace Version Harmonization**: Synchronized crate versions to `v0.3.3` across `replayfix-core` and `replayfix-cli`.
- **Windows Command Resolution in `doctor`**: Resolved Windows batch wrapper probes (`.cmd`) for npm/npx-managed tools (`npx`, `playwright`), ensuring 100% accurate environment compatibility reporting.

### Verified
- **Full Test Parity**: 96 passing automated tests (77 core unit tests, 17 CLI integration tests, 2 UI painter tests) with 0 failures.
- **BeautifulRust Invariants**: Zero `.unwrap()` and zero `.expect()` across all production source files; `unsafe_code = "forbid"` and `missing_debug_implementations = "warn"` enforced.
- **Strict Clippy Cleanliness**: `cargo clippy --workspace --all-targets --all-features -- -D warnings` passing with zero warnings.
- **Interactive & Non-Interactive UX**: Verified `--dry-run`, deterministic color precedence (`--color auto|always|never`), structured diagnostic cards, and quiet mode.

---

## [0.3.2] - 2026-10-27

Production-grade terminal UX and interaction engineering milestone.

### Added
- **Output Modes & Capability Precedence**: Added --color <auto|always|never> and --quiet (OutputMode::Human, OutputMode::Quiet, OutputMode::Machine).
- **Security & Trust Boundaries**: User-visible verification boundary cards declaring repository immutability ($\\Delta\\text{repo} = 0$), scratch isolation status, and SHA-256 test freeze.
- **Actionable Diagnostic Architecture**: Overhauled replayfix doctor with structured findings, impacts, concrete remediation, and evidence.
- **Dry-Run & Preview**: Added --dry-run flag to replayfix repro.
- **Deterministic Operational Steps**: Labeled phase indicators ([RUN], [PASS], [FAIL], [SKIP]) with contextual metadata and audit summaries.

---

## [0.3.1] - 2026-10-27

Terminal UX and Interaction Engineering patch. Full notes: [docs/release-notes/v0.3.1.md](docs/release-notes/v0.3.1.md).

### Added
- Dedicated Terminal UI Module (replayfix_cli::ui).
- Semantic styling painter (Theme) and accessible symbols (Symbols: check, cross, warn, arrow).
- Standard color policy detection honoring NO_COLOR, CLICOLOR_FORCE, TERM=dumb, and std::io::IsTerminal.
- Doctor diagnostic badges and repro step progress indicators.

---

## [0.3.0] - 2026-10-27

Delivers **Spec 030 (`030-v0.3-ephemeral-scratch-isolation`)**: Ephemeral scratch workspace isolation, dual-layer repository integrity ($\Delta \text{repo} = 0$), dynamic ephemeral port allocation (`127.0.0.1:0`), and cross-platform process tree supervision with Windows file-lock defense. Full notes: [docs/release-notes/v0.3.0.md](docs/release-notes/v0.3.0.md).

### Added
- **Dual-Layer Workspace Integrity (`replayfix_core::worktree::WorktreeManifest`)**:
  - **Layer 1**: Recursive filesystem walk computing SHA-256 over every non-excluded regular file (`FileEntry { path, size, sha256 }`) and hashing the RFC 8785 canonical JSON array into `tree_sha256`.
  - **Layer 2**: Automatic capture and verification of `git status --porcelain=v1 -uall` when `.git` is present in the target repository.
  - Built-in exclusion of `.git`, `.replayfix`, `node_modules`, `target`, and custom `--output-dir` directories.
- **Ephemeral Scratch Sandbox (`replayfix_core::worktree::ScratchSandbox`)**:
  - Clones target repository working files into a unique OS temporary directory (`replayfix-sandbox-<task>-<pid>-<seq>`).
  - Executes Gate 1, disk-backed patch application, and Gate 2 strictly inside the scratch sandbox while keeping the user's base repository untouched ($\Delta \text{repo} = 0$).
  - Automatic `Drop` and `teardown()` lifecycle that terminates registered child process trees, clears read-only file attributes, and retries `remove_dir_all` with bounded backoff (`10ms`â€“`200ms`) to defeat Windows file locks (`os error 5` / `os error 32`).
- **Dynamic Ephemeral Port Allocation (`replayfix_core::platform::EphemeralPort`)**:
  - Binds `TcpListener::bind("127.0.0.1:0")` to allocate a guaranteed non-zero OS ephemeral port (`NonZeroU16`) per run, preventing `EADDRINUSE` collisions across concurrent sessions.
- **Cross-Platform Process Tree Termination (`Platform::terminate_process_tree`)**:
  - Terminates spawned child process trees via `taskkill /F /T /PID <pid>` on Windows and `pkill -KILL -P <pid>` / `kill -9 <pid>` on Unix, with self/zero PID safety guards and idempotent handling of already-exited processes.
- **Receipt Schema `"0.3"` & Base-Repo Mutation Triage**:
  - Added `TriageStatus::BaseRepoDirtyMutationDetected` (`"BASE_REPO_DIRTY_MUTATION_DETECTED"`, exit code `11`).
  - Added `base_tree_sha256`, `sandbox_tree_sha256`, `base_repo_unmodified_verified`, and `ephemeral_port` to `AuditReceipt`.
- **Engineering Principles Codification**:
  - Codified the 10 Rust Engineering Principles in [docs/engineering-principles-rust.md](docs/engineering-principles-rust.md) and enforced workspace-wide `unsafe_code = "forbid"` and `missing_debug_implementations = "warn"` lints.
- **Spec Kit Artifacts & Expanded Test Suite**:
  - Complete specification, plan, research, data model, quickstart, checklist, and task breakdown under `specs/030-v0.3-ephemeral-scratch-isolation/`.
  - Expanded automated test suite to 94 tests (77 core unit tests + 17 CLI integration tests).

### Changed
- `MockHealer` now executes Gate 1, patch application, and Gate 2 against real files on disk inside `ScratchSandbox` rather than purely in-memory strings, and verifies pre-run vs. post-run `WorktreeManifest` parity on `--repo`.
- `replayfix repro` now validates that `--repo` exists and is a directory (exiting with code `2` if missing) and prints scratch sandbox, ephemeral port, and tree digest telemetry during `--mock-heal`.
- Bumped `SCHEMA_VERSION` in `replayfix_core::receipt` from `"0.2"` to `"0.3"`.

### Security
- Added symlink escape detection and relative path enforcement inside `ScratchSandbox` (`resolve_safe_path` and `reject_symlink_chain`).
- Added a 32 MiB per-file safety limit (`MAX_WORKSPACE_FILE_BYTES`) during workspace snapshot and sandbox cloning to prevent memory exhaustion DoS.
- Unauthorized mutation of the base repository during a run fails closed with `BASE_REPO_DIRTY_MUTATION_DETECTED` (exit code `11`).

---

## [0.2.1] - 2026-10-27

Maintenance and clean-architecture refactoring release following the comprehensive engineering confidence audit. Full notes: [docs/release-notes/v0.2.1.md](docs/release-notes/v0.2.1.md).

### Added
- **SSOT String Representation for `TriageStatus`**: Added `as_str()` and `Display` implementation directly on `TriageStatus` in `replayfix_core::receipt`, eliminating runtime serde string extraction overhead.
- **SSOT CLI Exit Codes**: Introduced `replayfix_cli::exit_codes` (`SUCCESS = 0`, `DOCTOR_FAILED = 1`, `USAGE_OR_INPUT_ERROR = 2`, `INTERNAL_OR_SYSTEM_ERROR = 10`, `SECURITY_VIOLATION = 11`, `REPAIR_REJECTED = 30`) centralizing all CLI termination pathways.
- **Root Crate Re-exports**: Re-exported `TriageStatus`, `AUDIT_RECEIPT_FILE`, and `TRIAGE_REPORT_FILE` from `replayfix_core` crate root.
- **Automated Property Tests**: Added `test_triage_status_as_str_and_display` and `test_triage_status_serde_parity` ensuring exact equivalence between serde serialization and string representation.

### Changed
- Refactored CLI triage report output to use native `Display` formatting directly on `report.status`.
- Replaced magic exit code numbers across CLI handlers with named `exit_codes` constants.

### Removed
- Debloated redundant `status_name` helper in `replayfix-cli`.

---

## [0.2.0] - 2026-10-27

First release of the dual-gate loop. It runs against a **simulated oracle**: no browser, video, model or repository access is involved yet. Full notes: [docs/release-notes/v0.2.0.md](docs/release-notes/v0.2.0.md).

### Added
- **Dual-gate state machine** (`replayfix_core::fsm`): `Init` -> `Gate1Red` -> `Repair` -> `Gate2Green`, plus a terminal `Rejected`. Illegal transitions fail closed.
- **Mock healer engine** (`replayfix_core::engine`): Gate 1 must be RED before repair; Gate 2 re-evaluates the same behavioral contract, compared by SHA-256 of its canonical JSON. RED is valid only when setup, target and action pass before a failed assertion.
- **In-memory unified-diff patcher** (`replayfix_core::patch`): single file, exact offsets and context, no fuzz; rejects bad headers, mismatched hunk counts, overlaps, multi-file patches, creation and rename headers, and unsafe header paths.
- **Anti-no-op guard**: empty, whitespace-only and net-zero patches end as `REPAIR_NOOP_REJECTED`.
- **Receipts**: `audit_receipt.json` (`VERIFIED_REPAIRED`) or `triage_report.json`. A rejected run removes any stale receipt in the same folder.
- **CLI**: `replayfix repro --mock-heal`, with `--mock-patch <builtin|noop|FILE>` and `--output-dir <DIR>`. Exit codes: `0` verified, `2` usage or input error, `10` Gate 1 not RED or contract mismatch, `11` security violation, `30` repair rejected.
- Built-in scenario for task `RF-01` (unguarded `cart.items.length`).
- `scripts/sign-build-outputs.ps1` and a troubleshooting section for Smart App Control blocks on proc-macro DLLs and test executables.
- Spec Kit artifacts for Spec 020 (`specs/020-v0.2-mock-healer-two-gates/`).
- 66 core unit tests and 15 CLI integration tests.

### Changed
- `replayfix --version` and the banner now read the crate version instead of a hardcoded string.
- The `repro` success line without `--mock-heal` now says what to do next instead of promising Gate 1 execution.
- `doctor` result lines were reworded in plain language (exit codes unchanged).
- README rewritten around what is implemented and verified, with the simulated-oracle limit stated up front.

### Removed
- `scripts/cargo-runner.ps1`: orphaned since v0.1.1 removed the Cargo runner.
- Unused dependencies: `tokio`, `tracing`, `walkdir` and `async-trait` from `replayfix-core`; `serde` and `tracing` from `replayfix-cli`; `tokio` now enables only the features the CLI uses.
- Mojibake in `docs/windows-code-signing.md`.

### Security
- Patch header paths, `--mock-patch` and `--output-dir` go through the same path checks as manifests (traversal, UNC, DOS device, ADS), exit code 11.
- Patch errors truncate echoed content so a wrong `--mock-patch` file cannot leak its contents into logs or reports.
- `--mock-patch` files are limited to 1 MiB.

### Breaking changes
- None. Without `--mock-heal`, `repro` behaves as in v0.1.1 apart from the reworded status line. Library users who match `CoreError` exhaustively must handle the new `CoreError::Patch` variant.

### Known limitations
- Only `RF-01` has a built-in scenario; the oracle inspects a bundled snippet, so receipts prove the state machine and receipt format, not real browser behavior (real gates: v0.4).
- No repository isolation yet (v0.3), no rejection of patches that touch test files and no two-run stability pass (v0.7).
- Receipts are not signed or Merkle-bound (v0.9).
- On Windows with Smart App Control, new builds may need `scripts/sign-build-outputs.ps1` before tests can launch the CLI.

---

## [0.1.1] - 2026-10-26

### Security Hardening (CSO Comprehensive Audit)
- **UNC Network Injection Defense**:
  - Rejection of UNC network paths (`\\host\share` and `//host/share`) across repository, recording, and manifest file paths (`TaskManifest::from_file`) to prevent SMB credential coercion and NTLM reflection attacks.
- **Multi-Component DOS Device Sanitization**:
  - Rejection of Windows reserved device filenames (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`) across all path components (including subdirectories like `projects/CON/repo`), eliminating descriptor hijacking and process hangs.
- **Alternate Data Stream (ADS) Defense**:
  - Rejection of unauthorized NTFS stream delimiters (`:`) in paths outside of valid Windows drive prefixes, blocking hidden payloads and stream smuggling.
- **Environment Adapter Sanitization**:
  - Strict regex enforcement on environment variable names (`port_env_var` matching `^[a-zA-Z_][a-zA-Z0-9_]*$`).
  - Validation of `healthcheck_path` against CRLF injection, control characters, and whitespace to prevent HTTP header injection.
  - Bounded verification of `startup_timeout_ms` (100ms to 600,000ms).
- **Regex DoS & Churn Elimination**:
  - Transitioned all regex patterns in `doctor` and `manifest` parsing to static `std::sync::OnceLock` singletons, eliminating runtime regex recompilation overhead.
- **Cross-Platform Path Normalization & Portability**:
  - Normalized Windows backslashes (`\`) to forward slashes (`/`) during path decomposition in `assert_safe_path`, ensuring path traversal and DOS device checks execute identically across Linux, macOS, and Windows.
  - Refined `OsFamily` taxonomy with discrete `MacOS`, `Linux`, `OtherUnix`, and `Windows` variants, along with `is_windows()` and `is_unix()` helpers.
  - Implemented platform-adaptive installation suggestions in `replayfix doctor` (`brew` for macOS, `apt`/`dnf` for Linux, `winget` for Windows).
- **Windows Code Signing & SAC Architecture**:
  - Replaced machine-specific Cargo runner overrides with native execution and standalone developer scripts (`scripts/setup-dev-cert.ps1` and `scripts/sign-binary.ps1`).
  - Eliminated PowerShell execution wrapper overhead and associated Smart App Control toast warnings during local testing.
  - Added comprehensive architectural and troubleshooting documentation in `docs/windows-code-signing.md`.
- **Automated Regression Tests**:
  - Added comprehensive automated security and cross-platform tests (including `test_security_violation_on_unc_path`, `test_security_violation_on_dos_device`, `test_security_violation_on_intermediate_dos_device`, `test_security_violation_on_ads_colon`, `test_from_file_rejects_unc_manifest_path`, `test_environment_adapter_security_validation`, `test_windows_backslash_path_normalization`, `test_security_violation_on_backslash_path_traversal`, `test_security_violation_on_backslash_dos_device`, `test_platform_os_family`, and `test_platform_remediation_suggestions`). All 17 workspace tests pass cleanly.

---

## [0.1.0] - 2026-10-26

### Added
- **Multi-Dependency Environment Diagnostics (`replayfix doctor`)**:
  - Deterministic probing and semantic version validation for Git ($\ge 2.30.0$), Node.js ($\ge 18.0.0$), Playwright ($\ge 1.40.0$), and FFmpeg ($\ge 4.4.0$).
  - Actionable remediation advice and installation commands on missing or incompatible dependencies.
  - Machine-readable structured JSON output (`--json`) for CI pipelines.
- **Canonical Task Manifest Ingestion (`replayfix repro`)**:
  - Strongly typed domain entities (`TaskManifest`, `EnvironmentAdapter`) strictly conforming to `task_manifest.schema.json`.
  - Dual ingestion modes: direct command-line arguments (`--task-id`, `--title`, `--repo`, `--bug-ref`, `--clean-ref`, `--video`, `--server-cmd`, etc.) or manifest file ingestion via `--manifest <path>`.
  - Visible browser replay flag (`--headed`) and trace capture flag (`--trace`).
- **Security Boundary Enforcement**:
  - Path canonicalization and path traversal detection on `--repo`, `--video`, and manifest paths, terminating attempts with `SECURITY_VIOLATION` (exit code 11).
- **Core Library Expansion**:
  - `replayfix_core::doctor` module with version comparison and dependency checkers.
  - `replayfix_core::manifest` module with serde schemas and validation rules.
  - Extended error taxonomy in `replayfix_core::error::CoreError`.

### Known Boundaries & Limitations
- Gate 1 browser reproduction and dual-worktree isolation are scheduled for upcoming vertical micro-slices (`020` / `030`). In this release, `replayfix repro` validates and hydrates the canonical task manifest, confirming readiness for baseline reproduction.

---

## [0.0.0] - 2026-10-26

### Added
- Pristine 2-crate root Cargo workspace: `crates/replayfix-core` (library) and `crates/replayfix-cli` (binary).
- Cross-platform native toolchain lock-in (`replayfix_core::Platform`) supporting Windows MSVC, macOS, and Linux.
- Structured error handling with `thiserror` (`CoreError`).
- Spec Kit governance configuration and baseline state journal.
