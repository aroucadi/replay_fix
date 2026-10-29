# ADR-02: Merkle Tree Protocol (RFC 6962 Domain Separation & Inclusion Proofs)

## Status
Accepted

## Context
A primary requirement of v0.9.0 is to emit RFC 6962-compliant Merkle receipts that mathematically bind all verification artifacts. Without strict domain separation, second-preimage attacks are possible where an internal node hash could be forged as a leaf node hash or vice-versa.

## Decision
1. **RFC 6962 Domain Separation**:
   - **Leaf Node Hashing**:
     $\text{MTH}(\text{Leaf}) = \text{SHA-256}(0x00 \parallel \text{LeafBytes})$
   - **Internal Node Hashing**:
     $\text{MTH}(\text{Internal}) = \text{SHA-256}(0x01 \parallel \text{LeftChildHash} \parallel \text{RightChildHash})$
   - **Empty Tree Hashing**:
     An empty tree (0 leaves) yields $\text{SHA-256}(" ")$ = `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

2. **Tree Construction (RFC 6962 Binary Tree Split)**:
   For $n > 1$ leaves, split at $k = 2^{\lfloor \log_2(n - 1) \rfloor}$ (the largest power of 2 strictly less than $n$):
   $\text{MTH}(D[0..n]) = \text{SHA-256}(0x01 \parallel \text{MTH}(D[0..k]) \parallel \text{MTH}(D[k..n]))$

3. **Inclusion Proofs**:
   An inclusion proof for leaf index $i$ in a tree of size $n$ consists of:
   ```rust
   pub struct MerkleInclusionProof {
       pub leaf_index: usize,
       pub tree_size: usize,
       pub audit_path: Vec<String>, // Hexadecimal SHA-256 node hashes
   }
   ```
   Verification follows the RFC 6962 audit path algorithm, computing the expected root and asserting exact equality with the signed Merkle root.

4. **Scope Boundaries**:
   - Inclusion proofs for all committed artifacts are mandatory.
   - Consistency proofs between consecutive trees are deliberately deferred to v1.0 transparency log extensions, as ReplayFix operates in batch verification receipt mode.

## Consequences
- **Positive**: Strict immunity against second-preimage collision attacks; fully reproducible proof generation and verification.
- **Test Strategy**: Validated against RFC 6962 standard test vectors and property-based test sweeps (tree sizes $0, 1, 2, 3, 5, 8, 16, 17$).
