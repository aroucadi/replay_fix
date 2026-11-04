# Benchmark Metadata: Vyra

## 📌 Overview
- **Application Name**: Vyra
- **Description**: Next.js 16 + React 19 Fitness, Workout Logging & Routine Planning Platform
- **Remote GitHub Repository**: [`https://github.com/aroucadi/vyra.git`](https://github.com/aroucadi/vyra.git)
- **Primary Tech Stack**: Next.js 16 (App Router), React 19, Tailwind CSS, Radix UI, Drizzle ORM

---

## 🎬 Video Recording Artifact
- **File**: [`lower_body_duplicate_bug.mp4`](./lower_body_duplicate_bug.mp4)
- **Format**: H.264 MP4, 1080p / 720p
- **Duration**: ~20 seconds
- **Interaction Recorded**: Real-world user journey inside the workout/routine builder selecting exercises from the Lower Body muscle category.

---

## 🐛 Bug Specification: Lower Body Duplicate Exercise Bug
- **Bug ID**: `VYRA-BUG-01`
- **Component**: Routine Builder / Exercise Selection Drawer (`src/components/routine-builder` or workout composer)
- **Trigger**: Selecting an exercise under the Lower Body category and clicking "Add to Routine".
- **Contract Violation**:
  - **Expected**: The selected exercise should be added once to the active routine draft; if already present, the quantity/sets counter should increment or an informative toast should indicate presence.
  - **Actual**: The list mutates with duplicated ID/key references, inserting duplicate exercise rows into the DOM and causing rendering inconsistencies.

---

## ⚡ 60-Second Judge Verification & Reproduction

Judges can clone and run Vyra locally with zero cloud dependencies:

```bash
# 1. Clone the repository
git clone https://github.com/aroucadi/vyra.git
cd vyra

# 2. Install dependencies
npm install

# 3. Start local development server
npm run dev
```

1. Navigate to `http://localhost:3000` (or the console output port).
2. Go to the Routine / Workout Creation view.
3. Filter by **Lower Body** exercises.
4. Add an exercise to the workout and observe the duplicate entry symptom shown in [`lower_body_duplicate_bug.mp4`](./lower_body_duplicate_bug.mp4).
