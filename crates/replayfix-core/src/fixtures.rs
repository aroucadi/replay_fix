//! Built-in fixtures for simulated and live-browser two-gate verification.

use crate::engine::BehavioralContract;

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

impl Fixture {
    /// Fixture for a task id, or `None` when no built-in scenario exists.
    pub fn for_task(task_id: &str) -> Option<Fixture> {
        match task_id {
            "RF-01" => Some(Self::rf01()),
            _ => None,
        }
    }

    /// Task ids that have a built-in fixture.
    pub const fn supported_tasks() -> &'static [&'static str] {
        &["RF-01"]
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_and_unknown_tasks() {
        assert!(Fixture::for_task("RF-01").is_some());
        assert!(Fixture::for_task("RF-99").is_none());
        assert_eq!(Fixture::supported_tasks(), &["RF-01"]);
    }

    #[test]
    fn test_contract_matches_fixture_task() {
        let f = Fixture::for_task("RF-01").unwrap();
        assert_eq!(f.contract.task_id, f.task_id);
        assert_eq!(f.click_x, 160);
        assert_eq!(f.click_y, 120);
        assert!(f.index_html.contains("Refresh Cart"));
    }
}
