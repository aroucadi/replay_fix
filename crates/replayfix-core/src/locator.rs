//! Spatial-semantic DOM locator grounding and fail-closed ambiguity guard.
//!
//! Given a click coordinate `(click_x, click_y)`, candidate interactive elements
//! collected via `document.elementFromPoint(x, y)` + `.closest(...)`, and the
//! live accessibility tree (`ariaSnapshot`), `LocatorResolver` scores each
//! candidate in integer basis points (`0..=10_000` bps, where `10_000` = `1.0000`)
//! and enforces:
//! 1. **DOM Grounding Invariant**: The target element must physically exist in
//!    the live `ariaSnapshot` (or carry an explicit `data-testid`) and synthesize
//!    a resilient Playwright locator (`getByRole` / `getByTestId`) free of blind
//!    `.first()`, `.last()`, or `:nth-match` indexing.
//! 2. **Fail-Closed Ambiguity Guard**: If no candidate reaches the minimum
//!    grounding score (`5_000` bps) or the top two distinct candidates differ by
//!    less than `2_500` bps ($|\Delta S| < 0.25$), resolution fails closed with
//!    `CoreError::Grounding`.

use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};

/// Basis-point scale representing a score of `1.0000`.
pub const SCORE_SCALE_BPS: u32 = 10_000;

/// Minimum required score margin between the top candidate and runner-up (`0.2500`).
pub const MIN_AMBIGUITY_MARGIN_BPS: u32 = 2_500;

/// Minimum score required for the winning candidate (`0.5000`).
pub const MIN_ACCEPTABLE_SCORE_BPS: u32 = 5_000;

/// Axis-aligned bounding box in CSS logical viewport pixels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct BoundingBox {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl BoundingBox {
    /// True when the box has non-zero area and `(px, py)` falls inside or on its edges.
    pub fn contains(&self, px: i32, py: i32) -> bool {
        if self.width == 0 || self.height == 0 {
            return false;
        }
        let max_x = self.x.saturating_add(self.width as i32);
        let max_y = self.y.saturating_add(self.height as i32);
        px >= self.x && px <= max_x && py >= self.y && py <= max_y
    }

    /// Center point `(cx, cy)` in CSS logical pixels.
    pub fn center(&self) -> (i32, i32) {
        (
            self.x.saturating_add((self.width / 2) as i32),
            self.y.saturating_add((self.height / 2) as i32),
        )
    }

    /// Rounded Euclidean distance in pixels from `(px, py)` to the box center.
    pub fn distance_to_center_px(&self, px: i32, py: i32) -> u32 {
        let (cx, cy) = self.center();
        let dx = i64::from(px) - i64::from(cx);
        let dy = i64::from(py) - i64::from(cy);
        let dist_sq = (dx * dx + dy * dy) as f64;
        dist_sq.sqrt().round() as u32
    }
}

/// Interactive DOM element candidate discovered in the live page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DomCandidate {
    pub tag: String,
    pub role: String,
    pub accessible_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_id: Option<String>,
    pub bbox: BoundingBox,
    pub is_element_from_point: bool,
}

/// Machine-readable telemetry recorded in `AuditReceipt` for grounded interactions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroundingTelemetry {
    pub click_x: i32,
    pub click_y: i32,
    pub viewport_normalized: bool,
    pub element_from_point_match: bool,
    pub accessible_name_match: bool,
    pub candidate_score_bps: u32,
    pub score_margin_bps: u32,
    pub matched_selector: String,
    pub aria_snapshot_excerpt: String,
}

impl GroundingTelemetry {
    /// Winning candidate score as a floating-point value in `[0.0, 1.0]` for display.
    pub fn candidate_score_f64(&self) -> f64 {
        f64::from(self.candidate_score_bps) / f64::from(SCORE_SCALE_BPS)
    }

    /// Margin over runner-up as a floating-point value in `[0.0, 1.0]` for display.
    pub fn score_margin_f64(&self) -> f64 {
        f64::from(self.score_margin_bps) / f64::from(SCORE_SCALE_BPS)
    }
}

/// Verified grounding result pairing the winning candidate, resilient selector, and telemetry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroundedTarget {
    pub candidate: DomCandidate,
    pub selector: String,
    pub telemetry: GroundingTelemetry,
}

/// Deterministic spatial-semantic locator resolver.
#[derive(Debug, Default)]
pub struct LocatorResolver;

impl LocatorResolver {
    /// Rejects empty selectors or selectors relying on blind positional indexing.
    pub fn validate_selector(selector: &str) -> Result<()> {
        let trimmed = selector.trim();
        if trimmed.is_empty() {
            return Err(CoreError::Grounding("Selector cannot be empty".into()));
        }
        for forbidden in [".first()", ".last()", ".nth(", ":nth-match", ":nth-child"] {
            if trimmed.contains(forbidden) {
                return Err(CoreError::Grounding(format!(
                    "Selector '{}' uses forbidden positional indexing '{}'",
                    trimmed, forbidden
                )));
            }
        }
        Ok(())
    }

    /// Verifies that `(role, accessible_name)` physically appears in the captured `ariaSnapshot`.
    pub fn verify_in_aria_snapshot(role: &str, accessible_name: &str, aria_snapshot: &str) -> bool {
        let norm_role = normalize_whitespace(role).to_ascii_lowercase();
        let norm_name = normalize_whitespace(accessible_name);
        if norm_role.is_empty() || norm_name.is_empty() {
            return false;
        }
        let norm_snapshot = normalize_whitespace(aria_snapshot);
        let quoted_pattern = format!("{norm_role} \"{norm_name}\"");
        let unquoted_pattern = format!("{norm_role}: {norm_name}");
        let lower_snapshot = norm_snapshot.to_ascii_lowercase();
        lower_snapshot.contains(&quoted_pattern.to_ascii_lowercase())
            || lower_snapshot.contains(&unquoted_pattern.to_ascii_lowercase())
    }

    /// Synthesizes a resilient Playwright locator for `candidate` without `.first()`.
    pub fn synthesize_selector(candidate: &DomCandidate) -> Result<String> {
        let role = normalize_whitespace(&candidate.role);
        let name = normalize_whitespace(&candidate.accessible_name);
        let selector = if !role.is_empty() && !name.is_empty() {
            let escaped_role = escape_single_quotes(&role);
            let escaped_name = escape_single_quotes(&name);
            format!("page.getByRole('{escaped_role}', {{ name: '{escaped_name}', exact: true }})")
        } else if let Some(test_id) = candidate
            .test_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let escaped_id = escape_single_quotes(test_id);
            format!("page.getByTestId('{escaped_id}')")
        } else {
            return Err(CoreError::Grounding(
                "Candidate lacks both (role + accessible_name) and data-testid".into(),
            ));
        };
        Self::validate_selector(&selector)?;
        Ok(selector)
    }

    /// Computes `(score_bps, aria_verified)` for a single candidate relative to `(click_x, click_y)`.
    pub fn score_candidate(
        candidate: &DomCandidate,
        click_x: i32,
        click_y: i32,
        aria_snapshot: &str,
    ) -> (u32, bool) {
        if candidate.bbox.width == 0 || candidate.bbox.height == 0 {
            return (0, false);
        }
        let aria_verified = Self::verify_in_aria_snapshot(
            &candidate.role,
            &candidate.accessible_name,
            aria_snapshot,
        );
        let has_test_id = candidate
            .test_id
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty());

        // DOM Grounding Invariant: must be verified in ariaSnapshot or carry an explicit test_id.
        if !aria_verified && !has_test_id {
            return (0, false);
        }

        let containment_bps = if candidate.bbox.contains(click_x, click_y) {
            4_000
        } else {
            0
        };
        let efp_bps = if candidate.is_element_from_point {
            2_500
        } else {
            0
        };
        let dist_px = candidate.bbox.distance_to_center_px(click_x, click_y);
        let proximity_bps = 2_000u32.saturating_sub(dist_px.saturating_mul(10));
        let semantic_bps =
            (if aria_verified { 1_000 } else { 0 }) + (if has_test_id { 500 } else { 0 });

        let total = (containment_bps + efp_bps + proximity_bps + semantic_bps).min(SCORE_SCALE_BPS);
        (total, aria_verified)
    }

    /// Resolves `(click_x, click_y)` against `candidates` and `aria_snapshot`, failing closed on
    /// ungrounded coordinates or ambiguous candidate ties ($|\Delta S| < 2500$ bps).
    pub fn resolve(
        click_x: i32,
        click_y: i32,
        viewport_width: u32,
        viewport_height: u32,
        aria_snapshot: &str,
        candidates: &[DomCandidate],
    ) -> Result<GroundedTarget> {
        if viewport_width == 0
            || viewport_height == 0
            || click_x < 0
            || click_y < 0
            || (click_x as u32) >= viewport_width
            || (click_y as u32) >= viewport_height
        {
            return Err(CoreError::Grounding(format!(
                "Click coordinates ({click_x}, {click_y}) fall outside viewport ({viewport_width}x{viewport_height})"
            )));
        }

        let snapshot_trimmed = aria_snapshot.trim();
        if snapshot_trimmed.is_empty() {
            return Err(CoreError::Grounding(
                "Empty ariaSnapshot: cannot ground DOM elements".into(),
            ));
        }

        let mut scored: Vec<(u32, bool, String, &DomCandidate)> = Vec::new();
        for cand in candidates {
            let Ok(selector) = Self::synthesize_selector(cand) else {
                continue;
            };
            let (score_bps, aria_ok) =
                Self::score_candidate(cand, click_x, click_y, snapshot_trimmed);
            if score_bps > 0 {
                scored.push((score_bps, aria_ok, selector, cand));
            }
        }

        // Sort descending by score_bps, breaking ties deterministically by selector string.
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.2.cmp(&b.2)));

        let Some((top_score, top_aria_ok, top_selector, top_candidate)) = scored.first().cloned()
        else {
            return Err(CoreError::Grounding(format!(
                "No grounded interactive DOM candidate found at ({click_x}, {click_y})"
            )));
        };

        if top_score < MIN_ACCEPTABLE_SCORE_BPS {
            return Err(CoreError::Grounding(format!(
                "Top candidate '{top_selector}' scored {top_score} bps, below minimum required {MIN_ACCEPTABLE_SCORE_BPS} bps"
            )));
        }

        // Find the highest-scoring runner-up with a distinct selector.
        let runner_up = scored
            .iter()
            .skip(1)
            .find(|(_, _, sel, _)| sel != &top_selector);

        let score_margin_bps = if let Some((second_score, _, second_selector, _)) = runner_up {
            let margin = top_score.saturating_sub(*second_score);
            if margin < MIN_AMBIGUITY_MARGIN_BPS {
                return Err(CoreError::Grounding(format!(
                    "Ambiguous grounding at ({click_x}, {click_y}): top candidate '{top_selector}' ({top_score} bps) and '{second_selector}' ({second_score} bps) differ by {margin} bps (< {MIN_AMBIGUITY_MARGIN_BPS} bps threshold)"
                )));
            }
            margin
        } else {
            top_score
        };

        let excerpt: String = snapshot_trimmed
            .lines()
            .take(8)
            .collect::<Vec<_>>()
            .join("\n");

        let telemetry = GroundingTelemetry {
            click_x,
            click_y,
            viewport_normalized: true,
            element_from_point_match: top_candidate.is_element_from_point,
            accessible_name_match: top_aria_ok,
            candidate_score_bps: top_score,
            score_margin_bps,
            matched_selector: top_selector.clone(),
            aria_snapshot_excerpt: excerpt,
        };

        Ok(GroundedTarget {
            candidate: top_candidate.clone(),
            selector: top_selector,
            telemetry,
        })
    }
}

fn normalize_whitespace(input: &str) -> String {
    input
        .replace(['\u{00a0}', '\u{2018}', '\u{2019}'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn escape_single_quotes(input: &str) -> String {
    input.replace('\\', "\\\\").replace('\'', "\\'")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_button_candidate() -> DomCandidate {
        DomCandidate {
            tag: "button".into(),
            role: "button".into(),
            accessible_name: "Refresh Cart".into(),
            test_id: Some("cart-refresh-btn".into()),
            bbox: BoundingBox {
                x: 100,
                y: 100,
                width: 120,
                height: 40,
            },
            is_element_from_point: true,
        }
    }

    #[test]
    fn test_resolves_clear_single_candidate() {
        let snapshot = "- main:\n  - heading \"Shopping Cart\"\n  - button \"Refresh Cart\"";
        let candidates = vec![sample_button_candidate()];
        let grounded =
            LocatorResolver::resolve(160, 120, 1280, 720, snapshot, &candidates).unwrap();

        assert_eq!(
            grounded.selector,
            "page.getByRole('button', { name: 'Refresh Cart', exact: true })"
        );
        assert!(grounded.telemetry.element_from_point_match);
        assert!(grounded.telemetry.accessible_name_match);
        assert_eq!(grounded.telemetry.candidate_score_bps, 10_000);
        assert!(grounded.telemetry.score_margin_bps >= MIN_AMBIGUITY_MARGIN_BPS);
    }

    #[test]
    fn test_rejects_ambiguous_candidate_tie_below_threshold() {
        let snapshot = "- button \"Save Draft\"\n- button \"Save Order\"";
        let cand_a = DomCandidate {
            tag: "button".into(),
            role: "button".into(),
            accessible_name: "Save Draft".into(),
            test_id: None,
            bbox: BoundingBox {
                x: 100,
                y: 100,
                width: 100,
                height: 40,
            },
            is_element_from_point: true,
        };
        let cand_b = DomCandidate {
            tag: "button".into(),
            role: "button".into(),
            accessible_name: "Save Order".into(),
            test_id: Some("save-order".into()),
            bbox: BoundingBox {
                x: 105,
                y: 100,
                width: 100,
                height: 40,
            },
            is_element_from_point: false,
        };

        let err =
            LocatorResolver::resolve(150, 120, 1280, 720, snapshot, &[cand_a, cand_b]).unwrap_err();
        assert!(matches!(err, CoreError::Grounding(_)));
        assert!(err.to_string().contains("Ambiguous grounding"));
    }

    #[test]
    fn test_rejects_ungrounded_candidate_missing_from_aria_snapshot() {
        let snapshot = "- heading \"Empty Page\"";
        let mut cand = sample_button_candidate();
        cand.test_id = None; // Neither in ariaSnapshot nor carrying a test_id
        let err = LocatorResolver::resolve(160, 120, 1280, 720, snapshot, &[cand]).unwrap_err();
        assert!(matches!(err, CoreError::Grounding(_)));
    }

    #[test]
    fn test_rejects_out_of_viewport_coordinates() {
        let snapshot = "- button \"Refresh Cart\"";
        let candidates = vec![sample_button_candidate()];
        assert!(LocatorResolver::resolve(-5, 120, 1280, 720, snapshot, &candidates).is_err());
        assert!(LocatorResolver::resolve(160, 900, 1280, 720, snapshot, &candidates).is_err());
    }

    #[test]
    fn test_bans_first_and_nth_positional_selectors() {
        assert!(LocatorResolver::validate_selector("page.getByRole('button').first()").is_err());
        assert!(LocatorResolver::validate_selector("page.locator('button').nth(0)").is_err());
        assert!(LocatorResolver::validate_selector("button:nth-match(1)").is_err());
        assert!(LocatorResolver::validate_selector(
            "page.getByRole('button', { name: 'Refresh Cart', exact: true })"
        )
        .is_ok());
    }
}
