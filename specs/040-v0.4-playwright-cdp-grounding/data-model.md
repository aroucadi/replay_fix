# Data Model: 040-v0.4-playwright-cdp-grounding

**Feature**: `040-v0.4-playwright-cdp-grounding`  
**Date**: 2026-10-27

---

## 1. `replayfix_core::locator` Entities

### `BoundingBox`
| Field | Type | Constraints | Description |
| :--- | :--- | :--- | :--- |
| `x` | `i32` | CSS logical px | Left coordinate of element bounding box |
| `y` | `i32` | CSS logical px | Top coordinate of element bounding box |
| `width` | `u32` | `> 0` for visible elements | Width in CSS logical pixels |
| `height` | `u32` | `> 0` for visible elements | Height in CSS logical pixels |

### `DomCandidate`
| Field | Type | Constraints | Description |
| :--- | :--- | :--- | :--- |
| `tag` | `String` | lowercase HTML tag | e.g., `"button"`, `"a"`, `"input"` |
| `role` | `String` | ARIA or implicit role | e.g., `"button"`, `"link"` |
| `accessible_name` | `String` | Trimmed label text | e.g., `"Refresh Cart"` |
| `test_id` | `Option<String>` | Optional `data-testid` | e.g., `Some("cart-refresh-btn")` |
| `bbox` | `BoundingBox` | CSS pixel box | Element geometry in viewport |
| `is_element_from_point` | `bool` | `true` if hit by `elementFromPoint(x, y)` | Direct reverse-probe hit indicator |

### `GroundingTelemetry`
| Field | Type | Constraints | Description |
| :--- | :--- | :--- | :--- |
| `click_x` | `i32` | Viewport X | Target click X coordinate |
| `click_y` | `i32` | Viewport Y | Target click Y coordinate |
| `viewport_normalized` | `bool` | `true` when inside viewport | Whether click fell inside viewport bounds |
| `element_from_point_match` | `bool` | `true` on direct hit | Whether `elementFromPoint` matched the winner |
| `accessible_name_match` | `bool` | `true` when verified in `ariaSnapshot` | Ground-truth `ariaSnapshot` verification |
| `candidate_score_bps` | `u32` | `0..=10_000` (`10000` = `1.0`) | Spatial-semantic score in basis points |
| `score_margin_bps` | `u32` | `>= 2_500` (`0.25`) to pass | Margin over runner-up candidate in basis points |
| `matched_selector` | `String` | Must not contain `.first()` | Synthesized Playwright selector |
| `aria_snapshot_excerpt` | `String` | Non-empty YAML excerpt | Captured `ariaSnapshot` from the live page |

---

## 2. `replayfix_core::browser` Protocol Entities

### `BridgeActionResult`
| Field | Type | Description |
| :--- | :--- | :--- |
| `navigated` | `bool` | True if page navigation succeeded |
| `aria_snapshot` | `String` | Accessibility YAML snapshot from `page.locator('body').ariaSnapshot()` |
| `candidates` | `Vec<DomCandidate>` | Interactive candidates discovered via `elementFromPoint` + DOM query |
| `clicked` | `bool` | True if grounded target click succeeded |
| `page_errors` | `Vec<String>` | Unhandled runtime exceptions captured via `page.on('pageerror')` |
| `result_text` | `String` | Post-click text content of `#cart-summary-output` |

---

## 3. `replayfix_core::receipt` Additions (Schema `"0.4"`)

- **`SCHEMA_VERSION`**: Updated to `"0.4"`.
- **`AuditReceipt`**: Adds optional `grounding_telemetry: Option<GroundingTelemetry>`.
- **`TriageStatus`**:
  - `AmbiguousGrounding` (`"AMBIGUOUS_GROUNDING"`, exit code `22`)
  - `BridgeExecutionFailed` (`"BRIDGE_EXECUTION_FAILED"`, exit code `20`)
