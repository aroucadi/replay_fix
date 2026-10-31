# Benchmark Metadata: MetaPulse

## 📌 Overview
- **Application Name**: MetaPulse
- **Description**: Full-Stack Threads/Discussions-style Community Feed Web Application
- **Remote GitHub Repository**: [`https://github.com/aroucadi/MetaPulse.git`](https://github.com/aroucadi/MetaPulse.git)
- **Live GitHub Issues**: [`https://github.com/aroucadi/MetaPulse/issues`](https://github.com/aroucadi/MetaPulse/issues)
- **Live Cloud Run Deployment**: [`https://metapulse-demo-365518648716.us-central1.run.app`](https://metapulse-demo-365518648716.us-central1.run.app)
- **Primary Tech Stack**: Node.js, Express, SQLite3 (`better-sqlite3` with WAL mode), Vanilla ES6+, CSS Glassmorphism Design System

---

## ⚡ 30-Second Local Reproduction for Judges (SWE-bench Standard)

MetaPulse follows the gold-standard **SWE-bench architecture**:
- **`main` branch**: Contains the complete baseline application with all 20 benchmark defects natively present together in the code (zero synthetic `if/else` conditionals or benchmark flag shortcuts).
- **20 Isolated Defect Branches (`bug/RF-01` to `bug/RF-20`)**: Each branch contains **strictly only that single defect** with the remaining 19 interactions completely clean and robust. This allows AI coding agents and human evaluators to run isolated reproduction tests and apply surgical code patches without side-effect pollution.

```bash
# 1. Clone the repository
git clone https://github.com/aroucadi/MetaPulse.git
cd MetaPulse

# 2. Install dependencies
npm install

# 3. Check out any target defect branch (e.g. RF-02)
git checkout bug/RF-02

# 4. Start the local server
npm start
```

1. Open `http://localhost:3000` in any browser.
2. The application boots with seeded production-like threads and reproduces the exact defect seen in the ground-truth benchmark recording.
3. To execute automated Playwright regression tests:
   ```bash
   npx playwright test
   ```

---

## 🎯 20-Bug Evaluation Benchmark Suite (`RF-01` to `RF-20`)

Every bug in this benchmark corresponds to a live public GitHub issue, an isolated Git branch, a Playwright contract assertion, and an `.mp4` video recording with an animated SVG human cursor and click ripples (zero initial blank frames):

| Bug ID | Isolated Branch | Live GitHub Issue | Video File | Bug Title & Root Cause |
| :--- | :--- | :--- | :--- | :--- |
| **RF-01** | [`bug/RF-01`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-01) | [#1](https://github.com/aroucadi/MetaPulse/issues/1) | [`RF-01_quote_repost_crash_with_cursor.mp4`](./RF-01_quote_repost_crash_with_cursor.mp4) | Quote Repost fails with TypeError: quoteContext is undefined |
| **RF-02** | [`bug/RF-02`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-02) | [#2](https://github.com/aroucadi/MetaPulse/issues/2) | [`RF-02_stale_follower_increment.mp4`](./RF-02_stale_follower_increment.mp4) | Follow button double-increments follower counter on rapid click |
| **RF-03** | [`bug/RF-03`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-03) | [#3](https://github.com/aroucadi/MetaPulse/issues/3) | [`RF-03_character_counter_underflow.mp4`](./RF-03_character_counter_underflow.mp4) | Character counter underflows into negative numbers and allows submission |
| **RF-04** | [`bug/RF-04`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-04) | [#4](https://github.com/aroucadi/MetaPulse/issues/4) | [`RF-04_draft_state_loss.mp4`](./RF-04_draft_state_loss.mp4) | Draft state lost on modal backdrop click without confirmation |
| **RF-05** | [`bug/RF-05`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-05) | [#5](https://github.com/aroucadi/MetaPulse/issues/5) | [`RF-05_like_race_condition_desync.mp4`](./RF-05_like_race_condition_desync.mp4) | Rapid like click causes optimistic counter race condition desync |
| **RF-06** | [`bug/RF-06`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-06) | [#6](https://github.com/aroucadi/MetaPulse/issues/6) | [`RF-06_whitespace_only_thread.mp4`](./RF-06_whitespace_only_thread.mp4) | Whitespace-only thread content passes validation and gets published |
| **RF-07** | [`bug/RF-07`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-07) | [#7](https://github.com/aroucadi/MetaPulse/issues/7) | [`RF-07_bio_length_raw_html_bypass.mp4`](./RF-07_bio_length_raw_html_bypass.mp4) | Profile bio length validation checks raw HTML string instead of text |
| **RF-08** | [`bug/RF-08`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-08) | [#8](https://github.com/aroucadi/MetaPulse/issues/8) | [`RF-08_disabled_post_button_stuck.mp4`](./RF-08_disabled_post_button_stuck.mp4) | Disabled Post button remains stuck disabled after correcting input |
| **RF-09** | [`bug/RF-09`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-09) | [#9](https://github.com/aroucadi/MetaPulse/issues/9) | [`RF-09_search_regex_break.mp4`](./RF-09_search_regex_break.mp4) | Search bar crashes client filter when entering regex special characters |
| **RF-10** | [`bug/RF-10`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-10) | [#10](https://github.com/aroucadi/MetaPulse/issues/10) | [`RF-10_category_dropdown_reset.mp4`](./RF-10_category_dropdown_reset.mp4) | Category filter dropdown resets to "All" upon pagination or submit error |
| **RF-11** | [`bug/RF-11`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-11) | [#11](https://github.com/aroucadi/MetaPulse/issues/11) | [`RF-11_stale_tab_switch_closure.mp4`](./RF-11_stale_tab_switch_closure.mp4) | Stale tab switch closure leaks old tab scroll listeners |
| **RF-12** | [`bug/RF-12`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-12) | [#12](https://github.com/aroucadi/MetaPulse/issues/12) | [`RF-12_missing_await_bookmark_desync.mp4`](./RF-12_missing_await_bookmark_desync.mp4) | Bookmark button missing await causes UI state reversion on refresh |
| **RF-13** | [`bug/RF-13`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-13) | [#13](https://github.com/aroucadi/MetaPulse/issues/13) | [`RF-13_unhandled_500_spinner_hang.mp4`](./RF-13_unhandled_500_spinner_hang.mp4) | Unhandled 500 error leaves loading skeleton spinner running indefinitely |
| **RF-14** | [`bug/RF-14`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-14) | [#14](https://github.com/aroucadi/MetaPulse/issues/14) | [`RF-14_search_race_condition.mp4`](./RF-14_search_race_condition.mp4) | Typeahead search race condition: slow response overwrites newer results |
| **RF-15** | [`bug/RF-15`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-15) | [#15](https://github.com/aroucadi/MetaPulse/issues/15) | [`RF-15_silent_session_expiry_close.mp4`](./RF-15_silent_session_expiry_close.mp4) | Session expiry during reply composition silently dismisses modal |
| **RF-16** | [`bug/RF-16`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-16) | [#16](https://github.com/aroucadi/MetaPulse/issues/16) | [`RF-16_backdrop_zindex_interception.mp4`](./RF-16_backdrop_zindex_interception.mp4) | Modal backdrop z-index intercepts clicks intended for header actions |
| **RF-17** | [`bug/RF-17`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-17) | [#17](https://github.com/aroucadi/MetaPulse/issues/17) | [`RF-17_dynamic_selector_drift.mp4`](./RF-17_dynamic_selector_drift.mp4) | Dynamic ID selector drift prevents interaction with newly appended replies |
| **RF-18** | [`bug/RF-18`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-18) | [#18](https://github.com/aroucadi/MetaPulse/issues/18) | [`RF-18_dropdown_clipped_overflow.mp4`](./RF-18_dropdown_clipped_overflow.mp4) | Thread card overflow:hidden clips action dropdown menu |
| **RF-19** | [`bug/RF-19`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-19) | [#19](https://github.com/aroucadi/MetaPulse/issues/19) | [`RF-19_focus_trap_escape.mp4`](./RF-19_focus_trap_escape.mp4) | Accessibility focus trap escapes into background page inside composer dialog |
| **RF-20** | [`bug/RF-20`](https://github.com/aroucadi/MetaPulse/tree/bug/RF-20) | [#20](https://github.com/aroucadi/MetaPulse/issues/20) | [`RF-20_mobile_hamburger_breakpoint_vanish.mp4`](./RF-20_mobile_hamburger_breakpoint_vanish.mp4) | Mobile navigation hamburger icon disappears at 768px breakpoint |
