//! Built-in mock fixtures for the simulated oracle.

use crate::engine::BehavioralContract;

/// A self-contained defect scenario: buggy source, its contract and a known-good fix.
#[derive(Debug, Clone)]
pub struct Fixture {
    pub task_id: &'static str,
    pub target_path: &'static str,
    pub buggy_source: &'static str,
    pub builtin_diff: &'static str,
    pub contract: BehavioralContract,
}

const RF01_SOURCE: &str = "export function cartItemCount(cart) {\n  return cart.items.length;\n}\n";

const RF01_DIFF: &str = "--- a/src/cart-summary.js\n+++ b/src/cart-summary.js\n@@ -1,3 +1,3 @@\n export function cartItemCount(cart) {\n-  return cart.items.length;\n+  return cart.items?.length || 0;\n }\n";

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
    }
}
