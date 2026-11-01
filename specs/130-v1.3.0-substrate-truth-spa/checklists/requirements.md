# Specification Quality Checklist: Spec 130 — Substrate Truth & Universal SPA Grounding (v1.3.0)

**Purpose**: Validate specification completeness, clarity, and consistency before and during implementation  
**Created**: 2026-11-01  
**Feature**: [spec.md](../spec.md)  

## Requirement Completeness
- [x] CHK001 - Are the mutual exclusion rules between `--mode live-ai` and offline mock flags explicitly specified with exit code `2`? [Completeness, Spec §REQ-FR-001]
- [x] CHK002 - Are the telemetry fields required to establish authentic live AI provenance (`repair_telemetry.provider_mode` and `video_telemetry.sam_snap.model`) documented? [Completeness, Spec §REQ-FR-002]
- [x] CHK003 - Is the token accounting zeroing rule defined whenever provenance is `"deterministic-fixture"`? [Completeness, Spec §REQ-FR-003]
- [x] CHK004 - Are the SPA framework detection heuristics (Vite, Next.js, CRA, Node scripts) specified? [Completeness, Spec §REQ-FR-004]
- [x] CHK005 - Are the dynamic port binding mechanisms (`PORT=0`, `--port 0`) and readiness polling timeouts (30s) defined? [Completeness, Spec §REQ-FR-004]
- [x] CHK006 - Is the Attached Mode CDP synchronization protocol (`Page.reload` and DOM ready wait) specified to prevent HMR race conditions? [Completeness, Spec §REQ-FR-005]
- [x] CHK007 - Is the removal of synthetic `#btn-publish-quote` and decoupling of `RF01_INDEX_HTML` documented? [Completeness, Spec §REQ-FR-006]

## Requirement Clarity & Precision
- [x] CHK008 - Is dynamic port capture defined with exact URL extraction patterns (`http://(?:127\.0\.0\.1|localhost):(\d+)`)? [Clarity, Research §2]
- [x] CHK009 - Is child dev-server process tree termination unambiguously defined across Windows (`Platform::terminate_process_tree`) and POSIX (`SIGKILL` to process group)? [Clarity, Research §3]
- [x] CHK010 - Are the audit receipt substrate tags (`"managed-spa-supervisor"` vs `"attached-spa-remote"` vs `"live-browser-cdp"`) uniquely defined? [Clarity, Plan §5.2]
- [x] CHK011 - Is the behavior on dev-server readiness timeout or crash defined with Exit Code `25` (`ERR_ENVIRONMENT_SETUP`)? [Clarity, Spec §3]

## Requirement Consistency & Constitutional Alignment
- [x] CHK012 - Is Law III ($\Delta\text{repo} = 0$) preserved in Attached Mode by isolating candidate patch trials in ephemeral scratch worktrees? [Consistency, Plan §3]
- [x] CHK013 - Is Law IV (Ephemeral Provenance) respected by binding managed servers to OS dynamic port 0 and honestly labeling remote attached endpoints? [Consistency, Plan §3]
- [x] CHK014 - Is Law VII (Model Honesty) upheld by omitting `model_provenance` on deterministic runs and stamping `"deterministic-fixture"` by default? [Consistency, Spec §REQ-FR-002]
- [x] CHK015 - Is Law X (BeautifulRust Discipline) enforced with `#![forbid(unsafe_code)]`, zero unhandled panics, and warning-free Clippy? [Consistency, Plan §2]

## Scenario & Edge Case Coverage
- [x] CHK016 - Is the system behavior specified when an attached dev-server endpoint is unreachable or returns 5xx? [Edge Case, Spec §3]
- [x] CHK017 - Is the handling of missing `node_modules` in scratch clones resolved via `NODE_PATH` injection? [Edge Case, Spec §3]
- [x] CHK018 - Is the behavior of default `replayfix run --bug RF-01` defined without requiring manual `--mock-heal` flags? [Scenario Coverage, Spec §3]

## Traceability & Acceptance
- [x] CHK019 - Do all functional requirements (`FR-001` through `FR-006`) map directly to implementation tasks in `tasks.md`? [Traceability]
- [x] CHK020 - Are JSON schema contracts (`contracts/managed_spa_config.schema.json` and `contracts/attached_spa_endpoint.schema.json`) aligned with the Rust structs? [Traceability]
