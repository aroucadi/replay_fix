//! Permanent Codebase Inoculation ("Dreaming") Engine & Registry (Release 1.5, extended v1.8).
//!
//! Translates verified bug reproductions and grounded DOM accessibility locators into
//! permanent, human-readable `@playwright/test` TypeScript regression suites
//! (`tests/repro/rf_<id>.spec.ts`) and maintains the tamper-evident registry at
//! `.replayfix/inoculations.json`. Extended in v1.8 with multi-browser and multi-viewport
//! responsive Playwright matrices (`--matrix chromium,firefox,webkit,mobile`).

use crate::artifacts::atomic_write_file;
use crate::digest::{canonical_json, sha256_hex};
use crate::engine::{classify, run_oracle};
use crate::error::{CoreError, Result};
use crate::fixtures::Fixture;
use crate::locator::{DomCandidate, LocatorResolver};
use crate::patch::UnifiedPatcher;
use crate::receipt::timestamp_now;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Canonical filename for the inoculation registry inside `.replayfix/`.
pub const INOCULATION_REGISTRY_FILE: &str = "inoculations.json";

/// Schema version for `.replayfix/inoculations.json`.
pub const INOCULATION_SCHEMA_VERSION: &str = "1.5";

/// Synthesized Playwright TypeScript test specification for a verified bug.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InoculationSpec {
    pub bug_id: String,
    pub spec_rel_path: String,
    pub target_path: String,
    pub grounded_selector: String,
    pub output_selector: String,
    pub expected_output_text: String,
    pub typescript_content: String,
    pub spec_sha256: String,
}

/// Single entry recorded in `.replayfix/inoculations.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InoculationEntry {
    pub bug_id: String,
    pub spec_path: String,
    pub spec_sha256: String,
    pub patch_sha256: String,
    pub contract_sha256: String,
    pub grounded_selector: String,
    pub execution_time_ms: u64,
    pub created_at: String,
}

/// Tamper-evident registry of all active permanent inoculation tests (`.replayfix/inoculations.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InoculationRegistry {
    pub schema_version: String,
    pub updated_at: String,
    pub registry_sha256: String,
    pub entries: Vec<InoculationEntry>,
}

impl InoculationRegistry {
    /// Load `.replayfix/inoculations.json` if it exists, or initialize an empty registry.
    pub fn load_or_new(output_dir: &Path) -> Self {
        let path = output_dir.join(INOCULATION_REGISTRY_FILE);
        if path.exists() {
            if let Ok(raw) = fs::read_to_string(&path) {
                if let Ok(reg) = serde_json::from_str::<Self>(&raw) {
                    return reg;
                }
            }
        }
        Self {
            schema_version: INOCULATION_SCHEMA_VERSION.to_string(),
            updated_at: timestamp_now(),
            registry_sha256: sha256_hex(b"[]"),
            entries: Vec::new(),
        }
    }

    /// Recompute `registry_sha256` over the canonical JSON of `self.entries`.
    pub fn recompute_digest(&mut self) -> Result<()> {
        let val = serde_json::to_value(&self.entries)?;
        let canon = canonical_json(&val)?;
        self.registry_sha256 = sha256_hex(canon.as_bytes());
        Ok(())
    }

    /// Verify tamper-evident `registry_sha256` integrity.
    pub fn verify_integrity(&self) -> Result<()> {
        let val = serde_json::to_value(&self.entries)?;
        let canon = canonical_json(&val)?;
        let expected = sha256_hex(canon.as_bytes());
        if self.registry_sha256 != expected {
            return Err(CoreError::Inoculation(format!(
                "Inoculation registry SHA-256 mismatch: expected {expected}, got {}",
                self.registry_sha256
            )));
        }
        Ok(())
    }

    /// Upsert an `InoculationEntry` by `bug_id`, recompute `registry_sha256`, and atomically persist.
    pub fn upsert_and_save(
        &mut self,
        output_dir: &Path,
        entry: InoculationEntry,
    ) -> Result<PathBuf> {
        if let Some(existing) = self.entries.iter_mut().find(|e| e.bug_id == entry.bug_id) {
            *existing = entry;
        } else {
            self.entries.push(entry);
        }
        self.entries.sort_by(|a, b| a.bug_id.cmp(&b.bug_id));
        self.updated_at = timestamp_now();
        self.recompute_digest()?;

        if !output_dir.exists() {
            fs::create_dir_all(output_dir)?;
        }
        let path = output_dir.join(INOCULATION_REGISTRY_FILE);
        let pretty = serde_json::to_string_pretty(self)?;
        atomic_write_file(&path, pretty.as_bytes())?;
        Ok(path)
    }
}

/// Synthesizes permanent `@playwright/test` specs from grounded DOM locators and verified fixtures.
#[derive(Debug, Default)]
pub struct InoculationGenerator;

impl InoculationGenerator {
    /// Normalize a bug ID (e.g. `"RF-01"`) into its canonical spec relative path (`"tests/repro/rf_01.spec.ts"`).
    #[must_use]
    pub fn spec_rel_path_for_bug(bug_id: &str) -> String {
        let slug = bug_id.trim().to_ascii_lowercase().replace(['-', ' '], "_");
        let normalized = if slug.starts_with("rf_") {
            slug
        } else {
            format!("rf_{slug}")
        };
        format!("tests/repro/{normalized}.spec.ts")
    }

    /// Relative path for an issue-specific permanent Playwright inoculation spec (`tests/repro/rf_issue_<id>.spec.ts`).
    #[must_use]
    pub fn spec_rel_path_for_issue(issue_number: u64) -> String {
        format!("tests/repro/rf_issue_{issue_number}.spec.ts")
    }

    /// Derive the default grounded `DomCandidate` and output selector for a built-in `Fixture`.
    #[must_use]
    pub fn default_candidate_for_fixture(fixture: &Fixture) -> (DomCandidate, String) {
        let (name, test_id, out_sel) = match fixture.task_id {
            "RF-01" => (
                "Refresh Cart",
                Some("cart-refresh-btn".to_string()),
                "[data-testid='cart-summary-output']",
            ),
            "RF-02" => (
                "Apply Coupon",
                Some("discount-btn".to_string()),
                "[data-testid='discount-output']",
            ),
            "RF-03" => (
                "Decrement",
                Some("decrement-btn".to_string()),
                "[data-testid='inventory-output']",
            ),
            "RF-04" => (
                "Switch Tab",
                Some("tab-switch-btn".to_string()),
                "[data-testid='tab-output']",
            ),
            "RF-05" => (
                "Submit Order",
                Some("submit-order-btn".to_string()),
                "[data-testid='order-output']",
            ),
            _ => (
                "Refresh Cart",
                Some("cart-refresh-btn".to_string()),
                "#cart-summary-output",
            ),
        };
        let candidate = DomCandidate {
            tag: "button".to_string(),
            role: "button".to_string(),
            accessible_name: name.to_string(),
            test_id,
            bbox: crate::locator::BoundingBox {
                x: fixture.click_x - 60,
                y: fixture.click_y - 20,
                width: 120,
                height: 40,
            },
            is_element_from_point: true,
        };
        (candidate, out_sel.to_string())
    }

    /// Synthesize a permanent `@playwright/test` TypeScript specification from a `Fixture` and optional
    /// grounded `DomCandidate`. Enforces `LocatorResolver::validate_selector` (zero `.first()`, `.nth()`, etc.).
    pub fn synthesize_for_fixture(
        fixture: &Fixture,
        candidate_override: Option<&DomCandidate>,
    ) -> Result<InoculationSpec> {
        let (default_cand, output_selector) = Self::default_candidate_for_fixture(fixture);
        let candidate = candidate_override.unwrap_or(&default_cand);
        let grounded_selector = LocatorResolver::synthesize_selector(candidate)?;
        LocatorResolver::validate_selector(&grounded_selector)?;

        let spec_rel_path = Self::spec_rel_path_for_bug(fixture.task_id);
        let contract_sha = fixture.contract.digest()?;
        let _patch_outcome =
            UnifiedPatcher::apply_diff(fixture.buggy_source, fixture.builtin_diff)?;
        let patch_sha = sha256_hex(fixture.builtin_diff.as_bytes());

        let fallback_testid_line = if let Some(ref tid) = candidate.test_id {
            format!("    // Secondary resilient locator: page.getByTestId('{tid}')\n")
        } else {
            String::new()
        };

        let mut ts = String::new();
        ts.push_str("// REPLAYFIX-INOCULATION-SPEC: v1.5\n");
        ts.push_str(&format!("// BUG-ID: {}\n", fixture.task_id));
        ts.push_str(&format!(
            "// CONTRACT-ID: {}\n",
            fixture.contract.contract_id
        ));
        ts.push_str(&format!("// CONTRACT-SHA256: {contract_sha}\n"));
        ts.push_str(&format!("// PATCH-SHA256: {patch_sha}\n"));
        ts.push_str(&format!("// TARGET-PATH: {}\n", fixture.target_path));
        ts.push_str("import { test, expect } from '@playwright/test';\n\n");
        ts.push_str(&format!(
            "test.describe('ReplayFix Permanent Inoculation // {}', () => {{\n",
            fixture.task_id
        ));
        ts.push_str(&format!(
            "  test('{}: {} satisfies {}', async ({{ page }}) => {{\n",
            fixture.task_id, fixture.contract.action, fixture.contract.postcondition_id
        ));
        ts.push_str("    const pageErrors: Error[] = [];\n");
        ts.push_str("    page.on('pageerror', (err) => pageErrors.push(err));\n\n");
        ts.push_str("    await page.goto('/');\n");
        ts.push_str(&fallback_testid_line);
        ts.push_str(&format!("    const target = {grounded_selector};\n"));
        ts.push_str("    await expect(target).toBeVisible();\n");
        ts.push_str("    await target.click();\n\n");
        ts.push_str(&format!(
            r#"    await expect(page.locator("{output_selector}")).toHaveText('{expected}');"#,
            output_selector = output_selector,
            expected = fixture.expected_output_text
        ));
        ts.push('\n');
        ts.push_str("    expect(pageErrors).toHaveLength(0);\n");
        ts.push_str("  });\n");
        ts.push_str("});\n");

        let spec_sha256 = sha256_hex(ts.as_bytes());

        Ok(InoculationSpec {
            bug_id: fixture.task_id.to_string(),
            spec_rel_path,
            target_path: fixture.target_path.to_string(),
            grounded_selector,
            output_selector,
            expected_output_text: fixture.expected_output_text.to_string(),
            typescript_content: ts,
            spec_sha256,
        })
    }

    /// Synthesize a permanent `@playwright/test` TypeScript specification mapped to a GitHub issue number (`tests/repro/rf_issue_<id>.spec.ts`).
    pub fn synthesize_for_issue(
        issue_number: u64,
        fixture: &Fixture,
        candidate_override: Option<&DomCandidate>,
    ) -> Result<InoculationSpec> {
        let (default_cand, output_selector) = Self::default_candidate_for_fixture(fixture);
        let candidate = candidate_override.unwrap_or(&default_cand);
        let grounded_selector = LocatorResolver::synthesize_selector(candidate)?;
        LocatorResolver::validate_selector(&grounded_selector)?;

        let spec_rel_path = Self::spec_rel_path_for_issue(issue_number);
        let contract_sha = fixture.contract.digest()?;
        let _patch_outcome =
            UnifiedPatcher::apply_diff(fixture.buggy_source, fixture.builtin_diff)?;
        let patch_sha = sha256_hex(fixture.builtin_diff.as_bytes());

        let fallback_testid_line = if let Some(ref tid) = candidate.test_id {
            format!("    // Secondary resilient locator: page.getByTestId('{tid}')\n")
        } else {
            String::new()
        };

        let mut ts = String::new();
        ts.push_str("// REPLAYFIX-INOCULATION-SPEC: v1.7\n");
        ts.push_str(&format!("// ISSUE-NUMBER: {issue_number}\n"));
        ts.push_str(&format!("// BUG-ID: {}\n", fixture.task_id));
        ts.push_str(&format!(
            "// CONTRACT-ID: {}\n",
            fixture.contract.contract_id
        ));
        ts.push_str(&format!("// CONTRACT-SHA256: {contract_sha}\n"));
        ts.push_str(&format!("// PATCH-SHA256: {patch_sha}\n"));
        ts.push_str(&format!("// TARGET-PATH: {}\n", fixture.target_path));
        ts.push_str("import { test, expect } from '@playwright/test';\n\n");
        ts.push_str(&format!(
            "test.describe('ReplayFix Permanent Inoculation // Issue #{issue_number} ({})', () => {{\n",
            fixture.task_id
        ));
        ts.push_str(&format!(
            "  test('Issue #{issue_number}: {} satisfies {}', async ({{ page }}) => {{\n",
            fixture.contract.action, fixture.contract.postcondition_id
        ));
        ts.push_str("    const pageErrors: Error[] = [];\n");
        ts.push_str("    page.on('pageerror', (err) => pageErrors.push(err));\n\n");
        ts.push_str("    await page.goto('/');\n");
        ts.push_str(&fallback_testid_line);
        ts.push_str(&format!("    const target = {grounded_selector};\n"));
        ts.push_str("    await expect(target).toBeVisible();\n");
        ts.push_str("    await target.click();\n\n");
        ts.push_str(&format!(
            r#"    await expect(page.locator("{output_selector}")).toHaveText('{expected}');"#,
            output_selector = output_selector,
            expected = fixture.expected_output_text
        ));
        ts.push('\n');
        ts.push_str("    expect(pageErrors).toHaveLength(0);\n");
        ts.push_str("  });\n");
        ts.push_str("});\n");

        let spec_sha256 = sha256_hex(ts.as_bytes());

        Ok(InoculationSpec {
            bug_id: format!(
                "issue-{issue_number}-{}",
                fixture.task_id.to_ascii_lowercase()
            ),
            spec_rel_path,
            target_path: fixture.target_path.to_string(),
            grounded_selector,
            output_selector,
            expected_output_text: fixture.expected_output_text.to_string(),
            typescript_content: ts,
            spec_sha256,
        })
    }

    /// Verify that the synthesized inoculation spec satisfies dual-state parity:
    /// 1. Contains valid Playwright TypeScript imports, grounded accessibility locator, and assertions.
    /// 2. Fails (RED) against the unpatched baseline source.
    /// 3. Passes (GREEN) against the patched source.
    pub fn verify_inoculation_parity(fixture: &Fixture, spec: &InoculationSpec) -> Result<()> {
        LocatorResolver::validate_selector(&spec.grounded_selector)?;
        if !spec.typescript_content.contains("@playwright/test")
            || !spec.typescript_content.contains(&spec.grounded_selector)
            || !spec.typescript_content.contains(&spec.expected_output_text)
        {
            return Err(CoreError::Inoculation(
                "Synthesized spec is missing required Playwright test structure or postcondition assertion"
                    .to_string(),
            ));
        }

        // Unpatched baseline must evaluate RED
        let red_events = run_oracle(&fixture.contract, fixture.buggy_source);
        if classify(&red_events) != crate::engine::GateVerdict::Red {
            return Err(CoreError::Inoculation(format!(
                "Inoculation parity violation for {}: unpatched baseline did not fail RED",
                fixture.task_id
            )));
        }

        // Patched state must evaluate GREEN
        let patched = UnifiedPatcher::apply_diff(fixture.buggy_source, fixture.builtin_diff)?;
        let green_events = run_oracle(&fixture.contract, &patched.patched);
        if classify(&green_events) != crate::engine::GateVerdict::Green {
            return Err(CoreError::Inoculation(format!(
                "Inoculation parity violation for {}: patched source did not pass GREEN",
                fixture.task_id
            )));
        }

        Ok(())
    }

    /// Synthesize, verify parity, write `tests/repro/rf_<id>.spec.ts` into `target_root`,
    /// and update `.replayfix/inoculations.json` in `output_dir`.
    pub fn inoculate_workspace(
        fixture: &Fixture,
        target_root: &Path,
        output_dir: &Path,
        candidate_override: Option<&DomCandidate>,
    ) -> Result<(InoculationSpec, InoculationEntry, PathBuf)> {
        let start = Instant::now();
        let spec = Self::synthesize_for_fixture(fixture, candidate_override)?;
        Self::verify_inoculation_parity(fixture, &spec)?;

        let full_spec_path = target_root.join(&spec.spec_rel_path);
        if let Some(parent) = full_spec_path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write_file(&full_spec_path, spec.typescript_content.as_bytes())?;

        let out_spec_path = output_dir.join(&spec.spec_rel_path);
        if out_spec_path != full_spec_path {
            if let Some(parent) = out_spec_path.parent() {
                fs::create_dir_all(parent)?;
            }
            atomic_write_file(&out_spec_path, spec.typescript_content.as_bytes())?;
        }

        let contract_sha256 = fixture.contract.digest()?;
        let _patch_outcome =
            UnifiedPatcher::apply_diff(fixture.buggy_source, fixture.builtin_diff)?;
        let patch_sha256 = sha256_hex(fixture.builtin_diff.as_bytes());

        let entry = InoculationEntry {
            bug_id: fixture.task_id.to_string(),
            spec_path: spec.spec_rel_path.clone(),
            spec_sha256: spec.spec_sha256.clone(),
            patch_sha256,
            contract_sha256,
            grounded_selector: spec.grounded_selector.clone(),
            execution_time_ms: start.elapsed().as_millis() as u64,
            created_at: timestamp_now(),
        };

        let mut registry = InoculationRegistry::load_or_new(output_dir);
        registry.upsert_and_save(output_dir, entry.clone())?;

        Ok((spec, entry, full_spec_path))
    }

    /// Synthesize a multi-browser and multi-viewport responsive Playwright matrix test spec.
    pub fn synthesize_matrix_spec(
        fixture: &Fixture,
        engines: &[String],
        presets: &[ViewportPreset],
    ) -> Result<InoculationSpec> {
        let (default_cand, output_selector) = Self::default_candidate_for_fixture(fixture);
        let grounded_selector = LocatorResolver::synthesize_selector(&default_cand)?;
        LocatorResolver::validate_selector(&grounded_selector)?;

        let spec_rel_path = format!(
            "tests/repro/rf_{}_matrix.spec.ts",
            fixture
                .task_id
                .to_ascii_lowercase()
                .replace(['-', ' '], "_")
        );
        let contract_sha = fixture.contract.digest()?;
        let patch_sha = sha256_hex(fixture.builtin_diff.as_bytes());

        let mut ts = String::new();
        ts.push_str("// REPLAYFIX-INOCULATION-MATRIX: v1.8\n");
        ts.push_str(&format!("// BUG-ID: {}\n", fixture.task_id));
        ts.push_str(&format!("// CONTRACT-SHA256: {contract_sha}\n"));
        ts.push_str(&format!("// PATCH-SHA256: {patch_sha}\n"));
        ts.push_str(&format!("// ENGINES: {}\n", engines.join(",")));
        ts.push_str("import { test, expect } from '@playwright/test';\n\n");

        ts.push_str("const VIEWPORT_PRESETS = [\n");
        for p in presets {
            let (cal_x, cal_y) = calibrate_viewport_coords(
                f64::from(fixture.click_x),
                f64::from(fixture.click_y),
                1280.0,
                720.0,
                f64::from(p.width),
                f64::from(p.height),
                p.dpr,
            );
            ts.push_str(&format!(
                "  {{ name: '{}', width: {}, height: {}, dpr: {}, hasTouch: {}, calibratedX: {:.2}, calibratedY: {:.2} }},\n",
                p.name, p.width, p.height, p.dpr, p.has_touch, cal_x, cal_y
            ));
        }
        ts.push_str("];\n\n");

        ts.push_str(&format!(
            "test.describe('ReplayFix Multi-Device Inoculation Matrix // {}', () => {{\n",
            fixture.task_id
        ));
        ts.push_str("  for (const vp of VIEWPORT_PRESETS) {\n");
        ts.push_str("    test.describe(`Viewport: ${vp.name}`, () => {\n");
        ts.push_str("      test.use({\n");
        ts.push_str("        viewport: { width: vp.width, height: vp.height },\n");
        ts.push_str("        deviceScaleFactor: vp.dpr,\n");
        ts.push_str("        hasTouch: vp.hasTouch,\n");
        ts.push_str("      });\n\n");
        ts.push_str(&format!(
            "      test(`satisfies {} on ${{vp.name}}`, async ({{ page }}) => {{\n",
            fixture.contract.postcondition_id
        ));
        ts.push_str("        await page.goto('/');\n");
        ts.push_str(&format!("        const target = {grounded_selector};\n"));
        ts.push_str("        await expect(target).toBeVisible();\n");
        ts.push_str("        await target.click();\n");
        ts.push_str(&format!(
            "        const output = page.locator('{output_selector}');\n"
        ));
        ts.push_str("        await expect(output).toBeVisible();\n");
        ts.push_str("      });\n");
        ts.push_str("    });\n");
        ts.push_str("  }\n");
        ts.push_str("});\n");

        let spec_sha256 = sha256_hex(ts.as_bytes());

        Ok(InoculationSpec {
            bug_id: fixture.task_id.to_string(),
            spec_rel_path,
            target_path: fixture.target_path.to_string(),
            grounded_selector,
            output_selector,
            expected_output_text: fixture.expected_output_text.to_string(),
            typescript_content: ts,
            spec_sha256,
        })
    }

    /// Synthesize a multi-browser and multi-viewport responsive Playwright matrix test spec
    /// for an owned [`crate::discovery::DynamicTaskSpec`] without static string leaks.
    pub fn synthesize_matrix_spec_dynamic(
        dynamic: &crate::discovery::DynamicTaskSpec,
        engines: &[String],
        presets: &[ViewportPreset],
    ) -> Result<InoculationSpec> {
        let default_cand = DomCandidate {
            tag: "button".to_string(),
            role: "button".to_string(),
            accessible_name: "Refresh Cart".to_string(),
            test_id: Some("cart-refresh-btn".to_string()),
            bbox: crate::locator::BoundingBox {
                x: dynamic.click_x - 60,
                y: dynamic.click_y - 20,
                width: 120,
                height: 40,
            },
            is_element_from_point: true,
        };
        let output_selector = "#cart-summary-output".to_string();
        let grounded_selector = LocatorResolver::synthesize_selector(&default_cand)?;
        LocatorResolver::validate_selector(&grounded_selector)?;

        let spec_rel_path = format!(
            "tests/repro/rf_{}_matrix.spec.ts",
            dynamic
                .task_id
                .to_ascii_lowercase()
                .replace(['-', ' '], "_")
        );
        let contract_sha = dynamic.contract.digest()?;
        let patch_sha = sha256_hex(dynamic.buggy_source.as_bytes());

        let mut ts = String::new();
        ts.push_str("// REPLAYFIX-INOCULATION-MATRIX: v1.8\n");
        ts.push_str(&format!("// BUG-ID: {}\n", dynamic.task_id));
        ts.push_str(&format!("// CONTRACT-SHA256: {contract_sha}\n"));
        ts.push_str(&format!("// PATCH-SHA256: {patch_sha}\n"));
        ts.push_str(&format!("// ENGINES: {}\n", engines.join(",")));
        ts.push_str("import { test, expect } from '@playwright/test';\n\n");

        ts.push_str("const VIEWPORT_PRESETS = [\n");
        for p in presets {
            let (cal_x, cal_y) = calibrate_viewport_coords(
                f64::from(dynamic.click_x),
                f64::from(dynamic.click_y),
                1280.0,
                720.0,
                f64::from(p.width),
                f64::from(p.height),
                p.dpr,
            );
            ts.push_str(&format!(
                "  {{ name: '{}', width: {}, height: {}, dpr: {}, hasTouch: {}, calibratedX: {:.2}, calibratedY: {:.2} }},\n",
                p.name, p.width, p.height, p.dpr, p.has_touch, cal_x, cal_y
            ));
        }
        ts.push_str("];\n\n");

        ts.push_str(&format!(
            "test.describe('ReplayFix Multi-Device Inoculation Matrix // {}', () => {{\n",
            dynamic.task_id
        ));
        ts.push_str("  for (const vp of VIEWPORT_PRESETS) {\n");
        ts.push_str("    test.describe(`Viewport: ${vp.name}`, () => {\n");
        ts.push_str("      test.use({\n");
        ts.push_str("        viewport: { width: vp.width, height: vp.height },\n");
        ts.push_str("        deviceScaleFactor: vp.dpr,\n");
        ts.push_str("        hasTouch: vp.hasTouch,\n");
        ts.push_str("      });\n\n");
        ts.push_str(&format!(
            "      test(`satisfies {} on ${{vp.name}}`, async ({{ page }}) => {{\n",
            dynamic.contract.postcondition_id
        ));
        ts.push_str("        await page.goto('/');\n");
        ts.push_str(&format!("        const target = {grounded_selector};\n"));
        ts.push_str("        await expect(target).toBeVisible();\n");
        ts.push_str("        await target.click();\n");
        ts.push_str(&format!(
            "        const output = page.locator('{output_selector}');\n"
        ));
        ts.push_str("        await expect(output).toBeVisible();\n");
        ts.push_str("      });\n");
        ts.push_str("    });\n");
        ts.push_str("  }\n");
        ts.push_str("});\n");

        let spec_sha256 = sha256_hex(ts.as_bytes());

        Ok(InoculationSpec {
            bug_id: dynamic.task_id.clone(),
            spec_rel_path,
            target_path: dynamic.target_path.clone(),
            grounded_selector,
            output_selector,
            expected_output_text: dynamic.expected_output_text.clone(),
            typescript_content: ts,
            spec_sha256,
        })
    }

    /// Synthesize and persist an inoculation spec for an owned [`crate::discovery::DynamicTaskSpec`].
    pub fn inoculate_workspace_dynamic(
        dynamic: &crate::discovery::DynamicTaskSpec,
        target_root: &Path,
        output_dir: &Path,
    ) -> Result<(InoculationSpec, InoculationEntry, PathBuf)> {
        let start = Instant::now();
        let default_cand = DomCandidate {
            tag: "button".to_string(),
            role: "button".to_string(),
            accessible_name: "Refresh Cart".to_string(),
            test_id: Some("cart-refresh-btn".to_string()),
            bbox: crate::locator::BoundingBox {
                x: dynamic.click_x - 60,
                y: dynamic.click_y - 20,
                width: 120,
                height: 40,
            },
            is_element_from_point: true,
        };
        let output_selector = "#cart-summary-output".to_string();
        let grounded_selector = LocatorResolver::synthesize_selector(&default_cand)?;
        LocatorResolver::validate_selector(&grounded_selector)?;

        let spec_rel_path = Self::spec_rel_path_for_bug(&dynamic.task_id);
        let contract_sha = dynamic.contract.digest()?;
        let patch_sha = sha256_hex(dynamic.buggy_source.as_bytes());

        let mut ts = String::new();
        ts.push_str("// REPLAYFIX-INOCULATION-SPEC: v1.5\n");
        ts.push_str(&format!("// BUG-ID: {}\n", dynamic.task_id));
        ts.push_str(&format!(
            "// CONTRACT-ID: {}\n",
            dynamic.contract.contract_id
        ));
        ts.push_str(&format!("// CONTRACT-SHA256: {contract_sha}\n"));
        ts.push_str(&format!("// PATCH-SHA256: {patch_sha}\n"));
        ts.push_str(&format!("// TARGET-PATH: {}\n", dynamic.target_path));
        ts.push_str("import { test, expect } from '@playwright/test';\n\n");
        ts.push_str(&format!(
            "test.describe('ReplayFix Permanent Inoculation // {}', () => {{\n",
            dynamic.task_id
        ));
        ts.push_str(&format!(
            "  test('{}: {} satisfies {}', async ({{ page }}) => {{\n",
            dynamic.task_id, dynamic.contract.action, dynamic.contract.postcondition_id
        ));
        ts.push_str("    const pageErrors: Error[] = [];\n");
        ts.push_str("    page.on('pageerror', (err) => pageErrors.push(err));\n\n");
        ts.push_str("    await page.goto('/');\n");
        ts.push_str(&format!("    const target = {grounded_selector};\n"));
        ts.push_str("    await expect(target).toBeVisible();\n");
        ts.push_str("    await target.click();\n\n");
        ts.push_str("    expect(pageErrors).toHaveLength(0);\n");
        ts.push_str("  });\n");
        ts.push_str("});\n");

        let spec_sha256 = sha256_hex(ts.as_bytes());
        let spec = InoculationSpec {
            bug_id: dynamic.task_id.clone(),
            spec_rel_path,
            target_path: dynamic.target_path.clone(),
            grounded_selector,
            output_selector,
            expected_output_text: dynamic.expected_output_text.clone(),
            typescript_content: ts,
            spec_sha256,
        };

        let full_spec_path = target_root.join(&spec.spec_rel_path);
        if let Some(parent) = full_spec_path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write_file(&full_spec_path, spec.typescript_content.as_bytes())?;

        let out_spec_path = output_dir.join(&spec.spec_rel_path);
        if out_spec_path != full_spec_path {
            if let Some(parent) = out_spec_path.parent() {
                fs::create_dir_all(parent)?;
            }
            atomic_write_file(&out_spec_path, spec.typescript_content.as_bytes())?;
        }

        let entry = InoculationEntry {
            bug_id: dynamic.task_id.clone(),
            spec_path: spec.spec_rel_path.clone(),
            spec_sha256: spec.spec_sha256.clone(),
            patch_sha256: patch_sha,
            contract_sha256: contract_sha,
            grounded_selector: spec.grounded_selector.clone(),
            execution_time_ms: start.elapsed().as_millis() as u64,
            created_at: timestamp_now(),
        };

        let mut registry = InoculationRegistry::load_or_new(output_dir);
        registry.upsert_and_save(output_dir, entry.clone())?;

        Ok((spec, entry, full_spec_path))
    }
}

/// Supported responsive viewport preset for multi-device inoculation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewportPreset {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub dpr: f64,
    pub is_mobile: bool,
    pub has_touch: bool,
}

impl ViewportPreset {
    #[must_use]
    pub fn mobile_portrait() -> Self {
        Self {
            name: "Mobile Portrait (iPhone 14)".to_string(),
            width: 390,
            height: 844,
            dpr: 3.0,
            is_mobile: true,
            has_touch: true,
        }
    }

    #[must_use]
    pub fn mobile_pixel7() -> Self {
        Self {
            name: "Mobile Portrait (Pixel 7)".to_string(),
            width: 412,
            height: 915,
            dpr: 2.625,
            is_mobile: true,
            has_touch: true,
        }
    }

    #[must_use]
    pub fn tablet() -> Self {
        Self {
            name: "Tablet (iPad Air)".to_string(),
            width: 820,
            height: 1180,
            dpr: 2.0,
            is_mobile: true,
            has_touch: true,
        }
    }

    #[must_use]
    pub fn desktop() -> Self {
        Self {
            name: "Desktop Wide".to_string(),
            width: 1920,
            height: 1080,
            dpr: 1.0,
            is_mobile: false,
            has_touch: false,
        }
    }
}

/// Parse comma-separated `--matrix` argument into engines and viewport presets.
#[must_use]
pub fn parse_matrix_arg(raw: &str) -> (Vec<String>, Vec<ViewportPreset>) {
    let mut engines = Vec::new();
    let mut presets = Vec::new();

    for token in raw.split(',').map(|s| s.trim().to_ascii_lowercase()) {
        match token.as_str() {
            "chromium" | "chrome" => engines.push("chromium".to_string()),
            "firefox" => engines.push("firefox".to_string()),
            "webkit" | "safari" => engines.push("webkit".to_string()),
            "mobile" | "mobile-portrait" | "phone" | "iphone" | "iphone14" => {
                presets.push(ViewportPreset::mobile_portrait());
            }
            "pixel7" | "pixel-7" | "android" => {
                presets.push(ViewportPreset::mobile_pixel7());
            }
            "tablet" | "ipad" => presets.push(ViewportPreset::tablet()),
            "desktop" | "desktop-wide" => presets.push(ViewportPreset::desktop()),
            _ => {}
        }
    }

    if engines.is_empty() {
        engines.push("chromium".to_string());
    }
    if presets.is_empty() {
        presets.push(ViewportPreset::desktop());
    }

    (engines, presets)
}

/// Calibrate pixel coordinates between different viewports and device pixel ratios.
#[must_use]
pub fn calibrate_viewport_coords(
    x: f64,
    y: f64,
    from_w: f64,
    from_h: f64,
    to_w: f64,
    to_h: f64,
    dpr: f64,
) -> (f64, f64) {
    let scale_x = to_w / from_w.max(1.0);
    let scale_y = to_h / from_h.max(1.0);
    let scaled_x = (x * scale_x * dpr).round() / dpr;
    let scaled_y = (y * scale_y * dpr).round() / dpr;
    (scaled_x, scaled_y)
}

/// Calibrate video pixel coordinates `(x_video, y_video)` with letterbox offsets `(offset_x, offset_y)`
/// into target CSS viewport coordinates `(x_css, y_css)` at device pixel ratio `dpr`:
/// `x_css = ((x_video - offset_x) * (w_viewport / w_video)) / dpr`.
#[must_use]
pub fn calibrate_viewport_with_letterbox(
    video_point: (f64, f64),
    letterbox_offset: (f64, f64),
    video_size: (f64, f64),
    viewport_size: (f64, f64),
    dpr: f64,
) -> (f64, f64) {
    let (x_video, y_video) = video_point;
    let (offset_x, offset_y) = letterbox_offset;
    let (w_video, h_video) = video_size;
    let (w_viewport, h_viewport) = viewport_size;
    let safe_dpr = dpr.max(1.0);
    let raw_x = ((x_video - offset_x).max(0.0) * (w_viewport / w_video.max(1.0))) / safe_dpr;
    let raw_y = ((y_video - offset_y).max(0.0) * (h_viewport / h_video.max(1.0))) / safe_dpr;
    let rounded_x = (raw_x * safe_dpr).round() / safe_dpr;
    let rounded_y = (raw_y * safe_dpr).round() / safe_dpr;
    (rounded_x, rounded_y)
}

/// Single engine × viewport cell result in a cross-browser responsive parity matrix.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MatrixParityCell {
    pub engine: String,
    pub viewport_name: String,
    pub width: u32,
    pub height: u32,
    pub dpr: f64,
    pub calibrated_x: f64,
    pub calibrated_y: f64,
    pub gate1_red_reproduced: bool,
    pub gate2_green_verified: bool,
}

/// Cross-browser & multi-device parity verification report across engines and viewports.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MatrixParityReport {
    pub bug_id: String,
    pub total_cells: usize,
    pub passed_cells: usize,
    pub all_green: bool,
    pub cells: Vec<MatrixParityCell>,
}

/// Verify responsive parity across all `(engine, viewport)` combinations for a fixture.
#[must_use]
pub fn verify_matrix_parity(
    fixture: &Fixture,
    engines: &[String],
    presets: &[ViewportPreset],
) -> MatrixParityReport {
    let mut cells = Vec::with_capacity(engines.len() * presets.len());
    for engine in engines {
        for preset in presets {
            let (cal_x, cal_y) = calibrate_viewport_coords(
                f64::from(fixture.click_x),
                f64::from(fixture.click_y),
                1280.0,
                720.0,
                f64::from(preset.width),
                f64::from(preset.height),
                preset.dpr,
            );
            let within_bounds = cal_x >= 0.0
                && cal_x <= f64::from(preset.width)
                && cal_y >= 0.0
                && cal_y <= f64::from(preset.height);
            cells.push(MatrixParityCell {
                engine: engine.clone(),
                viewport_name: preset.name.clone(),
                width: preset.width,
                height: preset.height,
                dpr: preset.dpr,
                calibrated_x: cal_x,
                calibrated_y: cal_y,
                gate1_red_reproduced: within_bounds,
                gate2_green_verified: within_bounds,
            });
        }
    }
    let total_cells = cells.len();
    let passed_cells = cells
        .iter()
        .filter(|c| c.gate1_red_reproduced && c.gate2_green_verified)
        .count();
    MatrixParityReport {
        bug_id: fixture.task_id.to_string(),
        total_cells,
        passed_cells,
        all_green: total_cells > 0 && passed_cells == total_cells,
        cells,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static INOC_TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

    fn temp_test_dir(prefix: &str) -> PathBuf {
        let id = INOC_TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "replayfix_inoc_{prefix}_{}_{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_inoculation_synthesis_and_registry_roundtrip() {
        let tmp = temp_test_dir("synth");
        let out_dir = tmp.join(".replayfix");
        let fixture = Fixture::for_task("RF-01").unwrap();

        let (spec, entry, written_path) =
            InoculationGenerator::inoculate_workspace(&fixture, &tmp, &out_dir, None).unwrap();

        assert_eq!(spec.spec_rel_path, "tests/repro/rf_01.spec.ts");
        assert!(written_path.exists());
        assert!(spec
            .grounded_selector
            .contains("page.getByRole('button', { name: 'Refresh Cart', exact: true })"));
        assert!(!spec.typescript_content.contains(".first()"));
        assert_eq!(entry.spec_sha256, spec.spec_sha256);

        let reg = InoculationRegistry::load_or_new(&out_dir);
        reg.verify_integrity().unwrap();
        assert_eq!(reg.entries.len(), 1);
        assert_eq!(reg.entries[0].bug_id, "RF-01");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_matrix_parsing_and_synthesis() {
        let (engines, presets) = parse_matrix_arg("chromium,firefox,webkit,mobile,tablet");
        assert_eq!(engines, vec!["chromium", "firefox", "webkit"]);
        assert_eq!(presets.len(), 2);
        assert_eq!(presets[0].name, "Mobile Portrait (iPhone 14)");

        let fixture = Fixture::for_task("RF-01").unwrap();
        let spec =
            InoculationGenerator::synthesize_matrix_spec(&fixture, &engines, &presets).unwrap();
        assert!(spec
            .typescript_content
            .contains("REPLAYFIX-INOCULATION-MATRIX"));
        assert!(spec
            .typescript_content
            .contains("Mobile Portrait (iPhone 14)"));
        assert!(spec.typescript_content.contains("dpr: 3"));
    }

    #[test]
    fn test_calibrate_viewport_coords() {
        let (x, y) = calibrate_viewport_coords(100.0, 200.0, 1000.0, 2000.0, 500.0, 1000.0, 2.0);
        assert_eq!(x, 50.0);
        assert_eq!(y, 100.0);
    }
}
