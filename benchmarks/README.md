# ReplayFix Benchmark Suite & Judge Verification Dataset

> Unified Multi-Repository Evaluation Dataset for Autonomous Screen-Recording-to-Playwright Reproduction and Dual-Worktree Git Auto-Patching.

This directory contains the complete empirical evaluation dataset used to test and grade **ReplayFix**. Every entry features:
1. **Recorded MP4 Video**: Recorded with realistic human cursor trajectories and visual click ripples, zero initial blank frames.
2. **Public Remote GitHub Repository**: Ready for instant cloning by hackathon judges.
3. **Structured Metadata (`BENCHMARK_METADATA.md`)**: Exact triggers, failing contract assertions, root-cause diffs, and 60-second local reproduction steps.

---

## 📊 Summary of Evaluation Repositories

| Repository Folder | Target Application | Category | Remote GitHub Repository | Video Artifacts | Verification Guide |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **[`metapulse/`](./metapulse)** | **MetaPulse** | Full-Stack Community App (Monolith) | [`https://github.com/aroucadi/MetaPulse.git`](https://github.com/aroucadi/MetaPulse.git) | **20 Videos** (`RF-01` to `RF-20`) | [`metapulse/BENCHMARK_METADATA.md`](./metapulse/BENCHMARK_METADATA.md) |
| **[`vyra/`](./vyra)** | **Vyra** | Full-Stack Fitness App (Next.js 16) | [`https://github.com/aroucadi/vyra.git`](https://github.com/aroucadi/vyra.git) | [`lower_body_duplicate_bug.mp4`](./vyra/lower_body_duplicate_bug.mp4) | [`vyra/BENCHMARK_METADATA.md`](./vyra/BENCHMARK_METADATA.md) |
| **[`react-date-picker/`](./react-date-picker)** | **`wojtekmaj/react-date-picker`** | Public OSS Component (1,400+ ⭐) | [`https://github.com/wojtekmaj/react-date-picker.git`](https://github.com/wojtekmaj/react-date-picker.git) | [`DEMO_react-date-picker_issue-361.mp4`](./react-date-picker/DEMO_react-date-picker_issue-361.mp4) | [`react-date-picker/BENCHMARK_METADATA.md`](./react-date-picker/BENCHMARK_METADATA.md) |
| **[`re-resizable/`](./re-resizable)** | **`bokuweb/re-resizable`** | Public OSS Component (3,000+ ⭐) | [`https://github.com/bokuweb/re-resizable.git`](https://github.com/bokuweb/re-resizable.git) | [`DEMO_re-resizable_issue-152.mp4`](./re-resizable/DEMO_re-resizable_issue-152.mp4) | [`re-resizable/BENCHMARK_METADATA.md`](./re-resizable/BENCHMARK_METADATA.md) |
| **[`react-resizable/`](./react-resizable)** | **`react-grid-layout/react-resizable`** | Public OSS Component (2,500+ ⭐) | [`https://github.com/react-grid-layout/react-resizable.git`](https://github.com/react-grid-layout/react-resizable.git) | [`DEMO_react-resizable_issue-136.mp4`](./react-resizable/DEMO_react-resizable_issue-136.mp4) | [`react-resizable/BENCHMARK_METADATA.md`](./react-resizable/BENCHMARK_METADATA.md) |

---

## 🚀 How ReplayFix Operates on this Dataset

For any benchmark directory, ReplayFix executes the end-to-end fail-to-pass pipeline:
1. **Ingestion**: Ingests the recorded `.mp4` video (e.g. `metapulse/RF-01_quote_repost_crash_with_cursor.mp4`).
2. **Vision Extraction**: SAM 3.1 & Muse Spark segment UI bounding boxes, track mouse cursor movement, and identify DOM action coordinates.
3. **Playwright Synthesis**: Synthesizes a deterministic `.replayfix_repro.spec.ts` test spec replicating the user flow.
4. **Dual-Worktree Oracle Verification**:
   - Runs synthesized test against buggy revision ➔ **FAIL** (verifying true reproduction).
   - Agent synthesizes surgical fix into clean worktree.
   - Runs test against patched revision ➔ **PASS** (verifying fix without regressions).
   - Generates minimal unified git patch.
