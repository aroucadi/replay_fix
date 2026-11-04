# Benchmark Metadata: re-resizable

## 📌 Overview
- **Repository Name**: `bokuweb/re-resizable`
- **Description**: Atomic, high-performance resizable component for React (Segment: 3,000+ stars)
- **Remote GitHub Repository**: [`https://github.com/bokuweb/re-resizable.git`](https://github.com/bokuweb/re-resizable.git)
- **Issue Reference**: [Issue #152 - Fix using right click on resize](https://github.com/bokuweb/re-resizable/pull/152)
- **Fix Commit**: [`05853acc`](https://github.com/bokuweb/re-resizable/commit/05853acc) (7-line surgical guard in `src/index.js`)

---

## 🎬 Video Recording Artifact
- **File**: [`DEMO_re-resizable_issue-152.mp4`](./DEMO_re-resizable_issue-152.mp4)
- **Format**: H.264 MP4, 1280x720, 25fps
- **Human Cursor**: Cyan/White SVG pointer with cubic bezier curve smoothing, amber/rose right-click ripple
- **Blank Frames**: 0.00s initial blank duration (auto-trimmed with FFmpeg `freezedetect`)

---

## 🐛 Bug Specification: Right-Click Resize Anomaly
- **Trigger**: Move pointer to any resize handle (e.g. right handle or bottom-right handle) and perform a right-click (`button === 2` / `which === 3`).
- **Contract Violation**:
  - **Expected**: Right-clicking on an element should either open the browser context menu or be ignored; resizing MUST strictly be initiated by the primary left mouse button (`button === 0`).
  - **Actual**: `onResizeStart` triggers on right-click. Because right-clicks do not emit standard window `mouseup` events as expected by the resize listener, the component becomes trapped in resize mode—tracking and resizing with mouse movement without any button held down.

---

## ⚡ 60-Second Judge Verification & Reproduction

Judges can clone and run `re-resizable` with the reproduction host:

```bash
# 1. Clone repository
git clone https://github.com/bokuweb/re-resizable.git
cd re-resizable

# 2. Check out commit or examine unpatched onResizeStart logic
git checkout 05853acc~1

# 3. Run reproduction test page (using Vite)
npx vite sample --port 5175
```

1. Open `http://localhost:5175`.
2. Move pointer to the right resize handle.
3. Right-click the handle.
4. Move the mouse cursor without clicking—observe the box unexpectedly expanding and following the cursor.
