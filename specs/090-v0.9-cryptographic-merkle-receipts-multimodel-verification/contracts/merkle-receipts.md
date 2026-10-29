# Contract: RFC 6962 Merkle Receipts

## 1. Scope & Domain Separation
All disaggregated fact artifacts emitted during a verification run are bound into an RFC 6962 compliant Merkle tree.

### 1.1 Leaf Hashing
For a leaf $L_i$ with canonical byte encoding $B_i$:
$$H(L_i) = \text{SHA-256}(0x00 \parallel B_i)$$

### 1.2 Internal Node Hashing
For child hashes $C_{\text{left}}$ and $C_{\text{right}}$ (each 32 bytes):
$$H(N) = \text{SHA-256}(0x01 \parallel C_{\text{left}} \parallel C_{\text{right}})$$

### 1.3 Empty Tree
If $n = 0$, the tree root is:
$$\text{MTH}(\emptyset) = \text{SHA-256}("") = \text{e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855}$$

---

## 2. Merkle Receipt Schema (`contracts/merkle_receipt.schema.json`)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "MerkleReceipt",
  "type": "object",
  "required": ["schema_version", "tree_size", "merkle_root", "leaves", "inclusion_proofs"],
  "properties": {
    "schema_version": { "type": "string", "const": "0.9" },
    "tree_size": { "type": "integer", "minimum": 0 },
    "merkle_root": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
    "leaves": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["artifact_type", "file_name", "content_sha256", "size_bytes"],
        "properties": {
          "artifact_type": { "type": "string" },
          "file_name": { "type": "string" },
          "content_sha256": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
          "size_bytes": { "type": "integer", "minimum": 0 }
        }
      }
    },
    "inclusion_proofs": {
      "type": "object",
      "additionalProperties": {
        "type": "object",
        "required": ["leaf_index", "tree_size", "audit_path"],
        "properties": {
          "leaf_index": { "type": "integer", "minimum": 0 },
          "tree_size": { "type": "integer", "minimum": 1 },
          "audit_path": {
            "type": "array",
            "items": { "type": "string", "pattern": "^[0-9a-f]{64}$" }
          }
        }
      }
    }
  }
}
```
