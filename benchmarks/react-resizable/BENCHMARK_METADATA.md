# Benchmark Metadata: react-resizable

## 📌 Overview
- **Repository Name**: `react-grid-layout/react-resizable`
- **Description**: Resizable box and handle component used in production grid layouts (Segment: 2,500+ stars)
- **Remote GitHub Repository**: [`https://github.com/react-grid-layout/react-resizable.git`](https://github.com/react-grid-layout/react-resizable.git)
- **Issue Reference**: [Issue #136 - Fix top-left resizing: account for new position when calling back onResize](https://github.com/react-grid-layout/react-resizable/pull/136) / Issue #10
- **Fix Commit**: [`6684a5b1`](https://github.com/react-grid-layout/react-resizable/commit/6684a5b1) (`lib/Resizable.tsx`)

---

## 🎬 Video Recording Artifact
- **File**: [`DEMO_react-resizable_issue-136.mp4`](./DEMO_react-resizable_issue-136.mp4)
- **Format**: H.264 MP4, 1280x720, 25fps
- **Human Cursor**: Cyan/White SVG pointer with visual drag indicator and click ripples
- **Blank Frames**: 0.00s initial blank duration (auto-trimmed with FFmpeg `freezedetect`)

---

## 🐛 Bug Specification: Top-Left (NW) Axis Delta Jitter
- **Trigger**: Click and drag from top-left (`nw`), top (`n`), or left (`w`) resize handles.
- **Contract Violation**:
  - **Expected**: Dragging outward should smoothly increase dimensions and update position proportionally.
  - **Actual**: Resizing from top or left handles physically moves the handle's bounding box during dragging. Without dynamic bounding-box delta compensation in `onDrag`, callback deltas compound, resulting in jitter, inverted dimensions, or sudden position snapping.

---

## ⚡ 60-Second Judge Verification & Reproduction

Judges can clone and run `react-resizable` locally:

```bash
# 1. Clone repository
git clone https://github.com/react-grid-layout/react-resizable.git
cd react-resizable

# 2. Check out pre-fix commit
git checkout 6684a5b1~1

# 3. Launch sample harness (using Vite)
npx vite sample --port 5176
```

1. Open `http://localhost:5176`.
2. Grab the top-left (`nw`) handle.
3. Drag diagonally outward.
4. Observe the compounding delta jitter and abrupt dimension jumps in the event telemetry stream.
