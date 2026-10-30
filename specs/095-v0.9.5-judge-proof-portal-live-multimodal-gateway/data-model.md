# Data Model: Spec 095 — Judge Proof Portal & Live Multimodal Gateway

## 1. Domain Entities & Schemas

### 1.1 Portal Verification Record (`PortalVerificationState`)
Represents the client-side state of an inspected receipt in `proof_viewer.html`:
```typescript
interface PortalVerificationState {
  // Input integrity
  rawJson: string;
  receiptType: "AuditReceipt" | "SweepReceipt";
  schemaVersion: "0.9";
  
  // Cryptographic verdicts
  schemaValid: boolean;
  canonicalizationValid: boolean;
  merkleRootVerified: boolean;
  inclusionProofsVerified: boolean;
  signatureValid: boolean;
  signerTrusted: boolean;
  overallStatus: 
    | "VERIFIED_GENUINE"
    | "SIGNATURE_VALID_UNTRUSTED_SIGNER"
    | "INTEGRITY_CHECK_FAILED"
    | "UNSIGNED_RECEIPT"
    | "MALFORMED_RECEIPT";

  // Merkle details
  recordedMerkleRoot: string;
  computedMerkleRoot: string;
  treeSize: number;
  leaves: Array<PortalEvidenceLeaf>;

  // Attestation details
  publicKey: string;
  signature: string;
  signingPayload: string;
  createdAt: string;

  // Domain payloads
  auditReceipt?: AuditReceiptPayload;
  sweepReceipt?: SweepReceiptPayload;
}
```

### 1.2 Portal Evidence Leaf (`PortalEvidenceLeaf`)
Represents an individual leaf node in the Merkle visualizer:
```typescript
interface PortalEvidenceLeaf {
  index: number;
  fileName: string;
  artifactType: string;
  contentSha256: string;
  sizeBytes: number;
  timestamp: string;
  leafHashHex: string;
  auditPath: Array<{
    hashHex: string;
    direction: "left" | "right";
  }>;
  verified: boolean;
  canonicalJsonPreview: string;
}
```

### 1.3 Cloud Gateway Request & Response Models (`crates/replayfix-core/src/gateway.rs`)
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GatewayProvider {
    MetaAi,
    BasetenFallback,
    MockOffline,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GatewayRequest {
    pub provider: GatewayProvider,
    pub model_slug: String,
    pub prompt: String,
    pub reasoning_effort: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub include_audio: bool, // Checked by Zero-Audio Rule
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GatewayResponse {
    pub provider: GatewayProvider,
    pub model_slug: String,
    pub response_text: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cached_tokens: u32,
    pub latency_ms: u64,
    pub raw_response_id: String,
}
```

### 1.4 Circuit Breaker State (`CircuitBreaker`)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Tripped due to failures, rejecting/diverting traffic
    HalfOpen, // Testing upstream health with canary request
}
```
