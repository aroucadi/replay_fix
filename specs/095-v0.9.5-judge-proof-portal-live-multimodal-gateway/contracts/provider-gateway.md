# Contract: Cloud Model Gateway & Provider Adapters

## 1. Allowed Provider Endpoints & Models
1. **Primary Provider (Meta Model API)**:
   - Base URL: `https://api.meta.ai/v1`
   - Endpoint: `POST /v1/responses`
   - Models:
     - `muse-wonder-1.0`: Deep multi-file reasoning, causal defect analysis.
     - `muse-spark-1.3`: Fast candidate patch synthesis, block AST generation.
     - `sam-3-1`: Visual prompt segmentation and bounding box detection.
     - `muse-voice-transcribe-1.0`: Speech transcription and audio alignment.
2. **Fallback Provider (Baseten Hosted Open Endpoint)**:
   - Base URL: `https://inference.baseten.co/v1`
   - Model: `meta-models/muse-spark-open`
3. **Deterministic Mock Provider**:
   - In-memory / fixture generator simulating offline execution without network or API tokens.

## 2. Authentication & Header Discipline
- Header: `Authorization: Bearer <META_API_KEY>`
- User-Agent: `ReplayFix/<CARGO_PKG_VERSION> (+https://github.com/aroucadi/replay_fix)`
- Redaction: Secrets and bearer tokens MUST NEVER be printed to standard output or persisted into receipt files.
