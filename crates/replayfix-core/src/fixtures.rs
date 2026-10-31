//! Built-in fixtures for simulated and live-browser two-gate verification,
//! plus the 20-bug MetaPulse Benchmark Suite (`RF-01` through `RF-20`) manifest
//! and repository branch runner.

use crate::digest::sha256_hex;
use crate::engine::BehavioralContract;
use crate::error::{CoreError, Result};
use crate::process::run_bounded;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;

/// Canonical GitHub repository URL for the MetaPulse 20-bug SWE-bench application.
pub const METAPULSE_REPO_URL: &str = "https://github.com/aroucadi/MetaPulse.git";

/// Single benchmark case entry in `benchmarks/metapulse/MANIFEST.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaPulseCaseRecord {
    pub bug_id: String,
    pub title: String,
    pub target_branch: String,
    pub repository_url: String,
    pub filename: String,
    pub size_bytes: usize,
    pub sha256: String,
}

/// Immutable manifest cataloging all 20 MetaPulse benchmark recordings (`RF-01`..`RF-20`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaPulseManifest {
    pub schema_version: String,
    pub benchmark_suite: String,
    pub repository_url: String,
    pub total_cases: usize,
    pub cases: Vec<MetaPulseCaseRecord>,
}

impl MetaPulseManifest {
    /// Load `benchmarks/metapulse/MANIFEST.json` relative to `repo_root`.
    pub fn load_from_repo(repo_root: &Path) -> Result<Self> {
        let path = repo_root.join("benchmarks/metapulse/MANIFEST.json");
        let raw = fs::read_to_string(&path)?;
        let manifest: Self = serde_json::from_str(&raw)?;
        Ok(manifest)
    }

    /// Verify that all 20 benchmark `.mp4` files exist on disk, match `size_bytes`,
    /// and hash to their recorded `sha256` digest.
    pub fn verify_all_videos(&self, repo_root: &Path) -> Result<()> {
        if self.total_cases != 20 || self.cases.len() != 20 {
            return Err(CoreError::Verification(format!(
                "Expected 20 MetaPulse benchmark cases, found {}",
                self.cases.len()
            )));
        }
        for case in &self.cases {
            let video_path = repo_root.join(&case.filename);
            if !video_path.exists() {
                return Err(CoreError::Verification(format!(
                    "Missing MetaPulse benchmark video '{}'",
                    video_path.display()
                )));
            }
            let bytes = fs::read(&video_path)?;
            if bytes.len() != case.size_bytes {
                return Err(CoreError::Verification(format!(
                    "MetaPulse video '{}' size mismatch (expected {}, got {})",
                    case.bug_id,
                    case.size_bytes,
                    bytes.len()
                )));
            }
            let actual_sha = sha256_hex(&bytes);
            if actual_sha != case.sha256 {
                return Err(CoreError::Verification(format!(
                    "MetaPulse video '{}' SHA-256 mismatch (expected {}, got {})",
                    case.bug_id, case.sha256, actual_sha
                )));
            }
        }
        Ok(())
    }

    /// Look up a specific benchmark case (`RF-01`..`RF-20`).
    #[must_use]
    pub fn find_case(&self, bug_id: &str) -> Option<&MetaPulseCaseRecord> {
        self.cases.iter().find(|c| c.bug_id == bug_id)
    }
}

/// Native runner to clone or check out the real MetaPulse repository at `bug/RF-01`..`bug/RF-20`.
#[derive(Debug, Clone)]
pub struct MetaPulseRepoRunner {
    pub repo_url: String,
}

impl Default for MetaPulseRepoRunner {
    fn default() -> Self {
        Self {
            repo_url: METAPULSE_REPO_URL.to_string(),
        }
    }
}

impl MetaPulseRepoRunner {
    /// Validate that `bug_id` is in `RF-01`..`RF-20` and return its canonical branch `bug/RF-XX`.
    pub fn target_branch_for_bug(bug_id: &str) -> Result<String> {
        if !Self::all_metapulse_bug_ids().contains(&bug_id) {
            return Err(CoreError::Verification(format!(
                "Unsupported MetaPulse bug_id '{bug_id}': expected RF-01..RF-20"
            )));
        }
        Ok(format!("bug/{bug_id}"))
    }

    /// All 20 canonical MetaPulse bug IDs (`RF-01` through `RF-20`).
    #[must_use]
    pub const fn all_metapulse_bug_ids() -> &'static [&'static str] {
        &[
            "RF-01", "RF-02", "RF-03", "RF-04", "RF-05", "RF-06", "RF-07", "RF-08", "RF-09",
            "RF-10", "RF-11", "RF-12", "RF-13", "RF-14", "RF-15", "RF-16", "RF-17", "RF-18",
            "RF-19", "RF-20",
        ]
    }

    /// Clone or check out `bug/<bug_id>` into `dest_dir` with a bounded timeout.
    pub fn checkout_defect_branch(&self, bug_id: &str, dest_dir: &Path) -> Result<String> {
        let branch = Self::target_branch_for_bug(bug_id)?;
        let dest_str = dest_dir.to_string_lossy().to_string();
        let mut cmd = std::process::Command::new("git");
        cmd.args([
            "clone",
            "--depth",
            "1",
            "--branch",
            &branch,
            "--single-branch",
            &self.repo_url,
            &dest_str,
        ]);
        let out = run_bounded(cmd, None, Duration::from_secs(60), 64 * 1024)?;
        if out.exit_code != Some(0) {
            return Err(CoreError::Verification(format!(
                "git clone of {} ({}) failed: {}",
                self.repo_url, branch, out.stderr
            )));
        }
        Ok(branch)
    }
}

/// A self-contained defect scenario: buggy source, HTML host page, click hotspot,
/// behavioral contract, and a known-good unified diff fix.
#[derive(Debug, Clone)]
pub struct Fixture {
    pub task_id: &'static str,
    pub target_path: &'static str,
    pub buggy_source: &'static str,
    pub builtin_diff: &'static str,
    pub index_html: &'static str,
    pub click_x: i32,
    pub click_y: i32,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub expected_output_text: &'static str,
    pub contract: BehavioralContract,
}

// ==========================================
// RF-01: Cart Summary Null Dereference
// ==========================================
const RF01_SOURCE: &str = "export function cartItemCount(cart) {\n  return cart.items.length;\n}\n";

const RF01_DIFF: &str = "--- a/src/cart-summary.js\n+++ b/src/cart-summary.js\n@@ -1,3 +1,3 @@\n export function cartItemCount(cart) {\n-  return cart.items.length;\n+  return cart.items?.length || 0;\n }\n";

const RF01_INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>MetaPulse Cart Summary — RF-01</title>
</head>
<body style="margin:0;padding:0;font-family:sans-serif;">
  <main>
    <h1>Shopping Cart</h1>
    <button id="refresh-cart-btn" data-testid="cart-refresh-btn" style="position:fixed;left:100px;top:100px;width:120px;height:40px;">Refresh Cart</button>
    <div id="cart-summary-output" data-testid="cart-summary-output">Pending</div>
  </main>
  <script type="module">
    import { cartItemCount } from './src/cart-summary.js';
    document.getElementById('refresh-cart-btn').addEventListener('click', () => {
      const count = cartItemCount({});
      document.getElementById('cart-summary-output').textContent = 'Items: ' + count;
    });
  </script>
</body>
</html>
"#;

// ==========================================
// RF-02: Stale Discount Calculation (State Mutation Twice)
// ==========================================
const RF02_SOURCE: &str = "export function applyDiscount(subtotal, coupon) {\n  if (coupon === 'SAVE20') {\n    return subtotal * 0.8 * 0.8;\n  }\n  return subtotal;\n}\n";

const RF02_DIFF: &str = "--- a/src/discount.js\n+++ b/src/discount.js\n@@ -1,6 +1,6 @@\n export function applyDiscount(subtotal, coupon) {\n   if (coupon === 'SAVE20') {\n-    return subtotal * 0.8 * 0.8;\n+    return subtotal * 0.8;\n   }\n   return subtotal;\n }\n";

const RF02_INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>MetaPulse Discount Summary — RF-02</title>
</head>
<body style="margin:0;padding:0;font-family:sans-serif;">
  <main>
    <h1>Checkout Discount</h1>
    <button id="apply-discount-btn" data-testid="discount-btn" style="position:fixed;left:100px;top:100px;width:120px;height:40px;">Apply Coupon</button>
    <div id="discount-output" data-testid="discount-output">Pending</div>
  </main>
  <script type="module">
    import { applyDiscount } from './src/discount.js';
    document.getElementById('apply-discount-btn').addEventListener('click', () => {
      const total = applyDiscount(100, 'SAVE20');
      document.getElementById('discount-output').textContent = 'Total: $' + total;
    });
  </script>
</body>
</html>
"#;

// ==========================================
// RF-03: Inventory Underflow Below Zero
// ==========================================
const RF03_SOURCE: &str =
    "export function decrementQuantity(currentQty) {\n  return currentQty - 1;\n}\n";

const RF03_DIFF: &str = "--- a/src/inventory.js\n+++ b/src/inventory.js\n@@ -1,3 +1,3 @@\n export function decrementQuantity(currentQty) {\n-  return currentQty - 1;\n+  return Math.max(0, currentQty - 1);\n }\n";

const RF03_INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>MetaPulse Inventory Counter — RF-03</title>
</head>
<body style="margin:0;padding:0;font-family:sans-serif;">
  <main>
    <h1>Item Inventory</h1>
    <button id="decrement-btn" data-testid="decrement-btn" style="position:fixed;left:100px;top:100px;width:120px;height:40px;">Decrement</button>
    <div id="inventory-output" data-testid="inventory-output">Pending</div>
  </main>
  <script type="module">
    import { decrementQuantity } from './src/inventory.js';
    document.getElementById('decrement-btn').addEventListener('click', () => {
      const qty = decrementQuantity(0);
      document.getElementById('inventory-output').textContent = 'Qty: ' + qty;
    });
  </script>
</body>
</html>
"#;

// ==========================================
// RF-04: Cart State Reset on Tab Switch
// ==========================================
const RF04_SOURCE: &str = "export function switchTab(tab, currentCart) {\n  return [];\n}\n";

const RF04_DIFF: &str = "--- a/src/cart-store.js\n+++ b/src/cart-store.js\n@@ -1,3 +1,3 @@\n export function switchTab(tab, currentCart) {\n-  return [];\n+  return currentCart || [];\n }\n";

const RF04_INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>MetaPulse Tab State — RF-04</title>
</head>
<body style="margin:0;padding:0;font-family:sans-serif;">
  <main>
    <h1>Feed Navigation</h1>
    <button id="tab-switch-btn" data-testid="tab-switch-btn" style="position:fixed;left:100px;top:100px;width:120px;height:40px;">Switch Tab</button>
    <div id="tab-output" data-testid="tab-output">Pending</div>
  </main>
  <script type="module">
    import { switchTab } from './src/cart-store.js';
    document.getElementById('tab-switch-btn').addEventListener('click', () => {
      const items = switchTab('feed', ['item1']);
      document.getElementById('tab-output').textContent = 'Retained: ' + items.length;
    });
  </script>
</body>
</html>
"#;

// ==========================================
// RF-05: Double Submit Idempotency Race
// ==========================================
const RF05_SOURCE: &str = "export function submitOrder(order, isSubmitting) {\n  return true;\n}\n";

const RF05_DIFF: &str = "--- a/src/order.js\n+++ b/src/order.js\n@@ -1,3 +1,6 @@\n export function submitOrder(order, isSubmitting) {\n+  if (isSubmitting) {\n+    return false;\n+  }\n   return true;\n }\n";

const RF05_INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>MetaPulse Order Submission — RF-05</title>
</head>
<body style="margin:0;padding:0;font-family:sans-serif;">
  <main>
    <h1>Checkout Order</h1>
    <button id="submit-order-btn" data-testid="submit-order-btn" style="position:fixed;left:100px;top:100px;width:120px;height:40px;">Submit Order</button>
    <div id="order-output" data-testid="order-output">Pending</div>
  </main>
  <script type="module">
    import { submitOrder } from './src/order.js';
    document.getElementById('submit-order-btn').addEventListener('click', () => {
      const accepted = submitOrder({ id: 1 }, true);
      document.getElementById('order-output').textContent = 'Accepted: ' + accepted;
    });
  </script>
</body>
</html>
"#;

impl Fixture {
    /// Fixture for a task id, or `None` when no built-in scenario exists.
    pub fn for_task(task_id: &str) -> Option<Fixture> {
        match task_id {
            "RF-01" => Some(Self::rf01()),
            "RF-02" => Some(Self::rf02()),
            "RF-03" => Some(Self::rf03()),
            "RF-04" => Some(Self::rf04()),
            "RF-05" => Some(Self::rf05()),
            _ => None,
        }
    }

    /// Task ids that have a built-in fixture.
    pub const fn supported_tasks() -> &'static [&'static str] {
        &["RF-01", "RF-02", "RF-03", "RF-04", "RF-05"]
    }

    fn rf01() -> Fixture {
        Fixture {
            task_id: "RF-01",
            target_path: "src/cart-summary.js",
            buggy_source: RF01_SOURCE,
            builtin_diff: RF01_DIFF,
            index_html: RF01_INDEX_HTML,
            click_x: 160,
            click_y: 120,
            viewport_width: 1280,
            viewport_height: 720,
            expected_output_text: "Items: 0",
            contract: BehavioralContract {
                contract_id: "RF-01-cart-count".to_string(),
                task_id: "RF-01".to_string(),
                setup: "render the cart summary for a cart whose items list is undefined"
                    .to_string(),
                target: "cartItemCount".to_string(),
                action: "request the item count".to_string(),
                postcondition_id: "returns-a-number-without-throwing".to_string(),
            },
        }
    }

    fn rf02() -> Fixture {
        Fixture {
            task_id: "RF-02",
            target_path: "src/discount.js",
            buggy_source: RF02_SOURCE,
            builtin_diff: RF02_DIFF,
            index_html: RF02_INDEX_HTML,
            click_x: 160,
            click_y: 120,
            viewport_width: 1280,
            viewport_height: 720,
            expected_output_text: "Total: $80",
            contract: BehavioralContract {
                contract_id: "RF-02-discount-calculation".to_string(),
                task_id: "RF-02".to_string(),
                setup: "apply a coupon code to a subtotal".to_string(),
                target: "applyDiscount".to_string(),
                action: "apply coupon SAVE20".to_string(),
                postcondition_id: "applies-discount-exactly-once".to_string(),
            },
        }
    }

    fn rf03() -> Fixture {
        Fixture {
            task_id: "RF-03",
            target_path: "src/inventory.js",
            buggy_source: RF03_SOURCE,
            builtin_diff: RF03_DIFF,
            index_html: RF03_INDEX_HTML,
            click_x: 160,
            click_y: 120,
            viewport_width: 1280,
            viewport_height: 720,
            expected_output_text: "Qty: 0",
            contract: BehavioralContract {
                contract_id: "RF-03-inventory-underflow".to_string(),
                task_id: "RF-03".to_string(),
                setup: "decrement an inventory quantity that is currently zero".to_string(),
                target: "decrementQuantity".to_string(),
                action: "decrement quantity from zero".to_string(),
                postcondition_id: "quantity-never-underflows-zero".to_string(),
            },
        }
    }

    fn rf04() -> Fixture {
        Fixture {
            task_id: "RF-04",
            target_path: "src/cart-store.js",
            buggy_source: RF04_SOURCE,
            builtin_diff: RF04_DIFF,
            index_html: RF04_INDEX_HTML,
            click_x: 160,
            click_y: 120,
            viewport_width: 1280,
            viewport_height: 720,
            expected_output_text: "Retained: 1",
            contract: BehavioralContract {
                contract_id: "RF-04-tab-state-reset".to_string(),
                task_id: "RF-04".to_string(),
                setup: "switch views while items exist in the cart store".to_string(),
                target: "switchTab".to_string(),
                action: "switch view tab".to_string(),
                postcondition_id: "cart-state-retained-across-tabs".to_string(),
            },
        }
    }

    fn rf05() -> Fixture {
        Fixture {
            task_id: "RF-05",
            target_path: "src/order.js",
            buggy_source: RF05_SOURCE,
            builtin_diff: RF05_DIFF,
            index_html: RF05_INDEX_HTML,
            click_x: 160,
            click_y: 120,
            viewport_width: 1280,
            viewport_height: 720,
            expected_output_text: "Accepted: false",
            contract: BehavioralContract {
                contract_id: "RF-05-double-submit-idempotency".to_string(),
                task_id: "RF-05".to_string(),
                setup: "submit an order when a submission is already pending".to_string(),
                target: "submitOrder".to_string(),
                action: "submit second order while in-flight".to_string(),
                postcondition_id: "rejects-duplicate-in-flight-submission".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_and_unknown_tasks() {
        for task in ["RF-01", "RF-02", "RF-03", "RF-04", "RF-05"] {
            let f = Fixture::for_task(task).unwrap();
            assert_eq!(f.task_id, task);
            assert_eq!(f.contract.task_id, task);
            assert!(f.index_html.contains(task));
            assert!(!f.target_path.is_empty());
            assert!(!f.buggy_source.is_empty());
            assert!(!f.builtin_diff.is_empty());
        }

        assert!(Fixture::for_task("RF-99").is_none());
        assert_eq!(
            Fixture::supported_tasks(),
            &["RF-01", "RF-02", "RF-03", "RF-04", "RF-05"]
        );
    }

    #[test]
    fn test_contract_matches_fixture_task() {
        let f = Fixture::for_task("RF-01").unwrap();
        assert_eq!(f.contract.task_id, f.task_id);
        assert_eq!(f.click_x, 160);
        assert_eq!(f.click_y, 120);
        assert!(f.index_html.contains("Refresh Cart"));
    }

    #[test]
    fn test_metapulse_20_bug_manifest_and_video_sha256_integrity() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let manifest = MetaPulseManifest::load_from_repo(&repo_root).unwrap();
        assert_eq!(manifest.total_cases, 20);
        assert_eq!(manifest.cases.len(), 20);
        manifest.verify_all_videos(&repo_root).unwrap();

        let rf01 = manifest.find_case("RF-01").unwrap();
        assert_eq!(rf01.target_branch, "bug/RF-01");
        assert!(rf01.title.contains("Quote Repost"));

        let rf20 = manifest.find_case("RF-20").unwrap();
        assert_eq!(rf20.target_branch, "bug/RF-20");
        assert_eq!(
            MetaPulseRepoRunner::target_branch_for_bug("RF-15").unwrap(),
            "bug/RF-15"
        );
        assert!(MetaPulseRepoRunner::target_branch_for_bug("RF-99").is_err());
    }
}
