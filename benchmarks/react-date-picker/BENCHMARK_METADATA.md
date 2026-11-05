# Benchmark Metadata: react-date-picker

## 📌 Overview
- **Repository Name**: `wojtekmaj/react-date-picker`
- **Description**: Atomic, dependency-free React date picker component (Segment: 1,400+ stars)
- **Remote GitHub Repository**: [`https://github.com/wojtekmaj/react-date-picker.git`](https://github.com/wojtekmaj/react-date-picker.git)
- **Issue Reference**: [Issue #361 - Calendar closes on date selection even when `closeCalendar={false}`](https://github.com/wojtekmaj/react-date-picker/issues/361)
- **Fix Commit**: [`d6d9b688`](https://github.com/wojtekmaj/react-date-picker/commit/d6d9b688) (2-line surgical patch)

---

## 🎬 Video Recording Artifact
- **File**: [`DEMO_react-date-picker_issue-361.mp4`](./DEMO_react-date-picker_issue-361.mp4)
- **Format**: H.264 MP4, 1280x720, 25fps
- **Human Cursor**: Cyan/White SVG pointer with cubic bezier spline interpolation and visual click ripples
- **Blank Frames**: 0.00s initial blank duration (clean start directly on the UI)

---

## 🐛 Bug Specification: Calendar Premature Dismissal
- **Trigger**: Click the calendar button to expand the date grid, then click any active date tile.
- **Contract Violation**:
  - **Expected**: With explicit prop `<DatePicker closeCalendar={false} />`, the calendar dropdown MUST remain open and visible so the user can see their selection in context or pick additional parameters.
  - **Actual**: In unpatched versions, the calendar abruptly closes upon date selection despite `closeCalendar={false}` being passed.

---

## ⚡ 60-Second Judge Verification & Reproduction

Judges can clone and test this external component locally:

```bash
# 1. Clone repository
git clone https://github.com/wojtekmaj/react-date-picker.git
cd react-date-picker

# 2. Check out pre-fix commit or tag v8.x / v9.0.0-beta
git checkout v9.0.0~1

# 3. Navigate to sample host harness
cd sample
npm install
npm run dev
```

1. Open `http://localhost:5173` (or sample dev port).
2. Ensure `closeCalendar={false}` is set on the component.
3. Click the calendar toggle icon, then click any calendar date tile.
4. Observe the bug: the calendar immediately snaps shut.
