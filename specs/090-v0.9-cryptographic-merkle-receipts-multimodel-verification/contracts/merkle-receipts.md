# Contract: RFC 6962 Merkle Receipts

## 1. Scope & Domain Separation
All disaggregated fact artifacts emitted during a verification run are bound into an RFC 6962-compliant Merkle tree.

### 1.1 Canonical Leaf Byte Encoding ($B_i$)
For any evidence leaf $L_i$, the byte sequence $B_i$ is **normatively defined** by **RFC 8785 (JSON Canonicalization Scheme / JCS)** applied to the serialized JSON object representation:

$$B_i = \text{RFC8785\_UTF8}(L_i)$$

The evidence leaf object $L_i$ contains exactly five fields:
1. `artifact_type` (string): Semantic type identifier (e.g. `"candidate_patch"`, `"audit_receipt"`).
2. `content_sha256` (string): 64-character lowercase hex SHA-256 digest of raw artifact bytes.
3. `file_name` (string): Base file name in artifacts store (e.g. `"01_candidate.diff"`).
4. `size_bytes` (integer): Exact uncompressed byte length as non-negative integer.
5. `timestamp` (string): ISO 8601 UTC timestamp with millisecond precision (e.g. `"2026-10-29T16:00:00.000Z"`).

#### Normative Canonicalization Rules (RFC 8785):
- **Key Sorting**: Object member names are sorted lexicographically by their UTF-16 code units.
- **Whitespace**: No whitespace is permitted between tokens (colons `:` and commas `,` have zero surrounding whitespace).
- **String Escaping**: Characters are escaped strictly per RFC 8785 / ECMA-404 rules.
- **Number Format**: Integers are serialized as standard decimal numbers without leading zeros or exponential notation. Floating-point numbers are strictly forbidden and rejected.
- **Encoding**: Output is strictly valid UTF-8.

#### Canonical Leaf Hash:
$$H(L_i) = \text{SHA-256}(0x00 \parallel B_i)$$

#### Test Vector:
Given leaf $L_0$:
```json
{
  "artifact_type": "candidate_patch",
  "content_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "file_name": "01_candidate.diff",
  "size_bytes": 42,
  "timestamp": "2026-10-29T16:00:00.000Z"
}
```
Regardless of input property ordering, the canonical byte string $B_0$ is bitwise identical to:
```
{"artifact_type":"candidate_patch","content_sha256":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","file_name":"01_candidate.diff","size_bytes":42,"timestamp":"2026-10-29T16:00:00.000Z"}
```
And its leaf hash is:
$$H(L_0) = \text{SHA-256}(0x00 \parallel B_0)$$

---

### 1.2 Internal Node Hashing
For child hashes $C_{\text{left}}$ and $C_{\text{right}}$ (each 32 bytes):
$$H(N) = \text{SHA-256}(0x01 \parallel C_{\text{left}} \parallel C_{\text{right}})$$

### 1.3 Empty Tree
If $n = 0$, the tree root is:
$$\text{MTH}(\emptyset) = \text{SHA-256}("") = \text{e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855}$$

---

## 2. Inclusion Proof Keys & Interoperability
In `merkle_receipt.json`, the `inclusion_proofs` map associates evidence leaves with their RFC 6962 audit paths:

- **Normative Canonical Key**: The artifact `file_name` (e.g. `"01_candidate.diff"`).
- **Supported Interoperability Keys**:
  - 0-indexed leaf position as decimal string (e.g. `"0"`, `"1"`).
  - 64-character lowercase hexadecimal leaf hash ($H(L_i)$).
  - 64-character lowercase hexadecimal content SHA-256 digest.
- **Verification Requirement**: A conforming verifier MUST attempt the canonical `file_name` first, then evaluate interoperability fallback keys. A verifier MUST enforce that `proof.leaf_index == i` matches the leaf's position in the ordered `leaves` sequence.

---

## 3. Merkle Receipt Schema (`contracts/merkle_receipt.schema.json`)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "MerkleReceipt",
  "type": "object",
  "required": [
    "schema_version",
    "tree_size",
    "merkle_root",
    "leaves",
    "inclusion_proofs"
  ],
  "properties": {
    "schema_version": { "type": "string", "const": "0.9" },
    "tree_size": { "type": "integer", "minimum": 0 },
    "merkle_root": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
    "leaves": {
      "type": "array",
      "items": {
        "type": "object",
        "required": [
          "artifact_type",
          "file_name",
          "content_sha256",
          "size_bytes",
          "timestamp"
        ],
        "properties": {
          "artifact_type": { "type": "string" },
          "file_name": { "type": "string" },
          "content_sha256": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
          "size_bytes": { "type": "integer", "minimum": 0 },
          "timestamp": { "type": "string", "format": "date-time" }
        }
      }
    },
    "inclusion_proofs": {
      "type": "object",
      "description": "Map of inclusion proofs keyed canonically by leaf file_name, or alternatively by leaf index or leaf hash",
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
    },
    "attestation": {
      "type": "object",
      "description": "Optional RFC 8032 Ed25519 digital attestation binding the Merkle root",
      "required": [
        "schema_version",
        "algorithm",
        "public_key",
        "signature",
        "signing_payload",
        "created_at"
      ],
      "properties": {
        "schema_version": { "type": "string", "const": "0.9" },
        "algorithm": { "type": "string", "const": "Ed25519" },
        "public_key": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
        "signature": { "type": "string", "pattern": "^[0-9a-f]{128}$" },
        "signing_payload": { "type": "string" },
        "created_at": { "type": "string", "format": "date-time" }
      }
    }
  }
}
```
