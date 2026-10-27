//! Canonical JSON serialization and SHA-256 helpers.
//!
//! The canonical form sorts object keys recursively (UTF-16 code-unit order, as
//! RFC 8785 requires) and emits compact JSON. It supports strings, booleans,
//! null, arrays, objects and integers. Non-integer numbers are rejected rather
//! than serialized ambiguously.

use crate::error::{CoreError, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::Write;

/// Serialize a JSON value into its canonical, compact, key-sorted form.
pub fn canonical_json(value: &Value) -> Result<String> {
    let mut out = String::new();
    write_canonical(value, &mut out)?;
    Ok(out)
}

fn write_canonical(value: &Value, out: &mut String) -> Result<()> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                out.push_str(&n.to_string());
            } else {
                return Err(CoreError::Serialization(
                    <serde_json::Error as serde::ser::Error>::custom(
                        "non-integer numbers are not supported in canonical JSON",
                    ),
                ));
            }
        }
        Value::String(s) => out.push_str(&serde_json::to_string(s)?),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
            out.push('{');
            for (i, (key, val)) in entries.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key)?);
                out.push(':');
                write_canonical(val, out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

/// Lowercase hexadecimal SHA-256 digest of arbitrary bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_canonical_json_sorts_keys_recursively() {
        let value = json!({"b": 1, "a": {"z": [true, null], "y": "x"}});
        assert_eq!(
            canonical_json(&value).unwrap(),
            r#"{"a":{"y":"x","z":[true,null]},"b":1}"#
        );
    }

    #[test]
    fn test_canonical_json_is_order_independent() {
        let a = json!({"one": 1, "two": 2});
        let b = json!({"two": 2, "one": 1});
        assert_eq!(canonical_json(&a).unwrap(), canonical_json(&b).unwrap());
    }

    #[test]
    fn test_canonical_json_rejects_floats() {
        assert!(canonical_json(&json!({"v": 1.5})).is_err());
    }

    #[test]
    fn test_sha256_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn test_sha256_empty_input() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
