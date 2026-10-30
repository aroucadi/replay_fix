# ADR-04: Cloud Gateway and Provider Adapters (`replayfix-core::gateway`)

## Status
Accepted

## Context
ReplayFix v0.9.5 requires a unified, bounded Multimodal Cloud Gateway (`crates/replayfix-core/src/gateway.rs`) that interfaces with verified Meta Model APIs (`muse-wonder-1.0`, `muse-spark-1.3`, `sam-3-1`, `muse-voice-transcribe-1.0`), supports an approved open-weight fallback (`meta-models/muse-spark-open` via Baseten `https://inference.baseten.co/v1`), and provides deterministic offline mock execution when credentials are absent.

## Decision
1. **Provider Adapter Architecture**:
   - Defines `GatewayProvider` (`MetaAi`, `BasetenFallback`, `MockOffline`), `ModelRole` (`DeepReasoning`, `FastSynthesis`, `SpatialSegmentation`, `SpeechTranscription`), `GatewayRequest`, and `GatewayResponse`.
   - Enforces model-role compatibility: `sam-3-1` is restricted to `SpatialSegmentation`; `muse-voice-transcribe-1.0` is restricted to `SpeechTranscription`; `muse-wonder-1.0` and `muse-spark-1.3` handle code reasoning/synthesis.
2. **Secret Redaction & Provenance Separation**:
   - API keys are read from environment variables (`META_API_KEY`, `BASETEN_API_KEY`) and redacted in all `Debug`, `Display`, log, and error outputs (`Bearer [REDACTED]`).
   - Every `GatewayResponse` records `requested_model`, `serving_model`, `provider`, `provenance` (`LiveProvider` vs. `DeterministicFixture`), `request_fingerprint_sha256`, token usage, nano-dollar cost, and wall-clock latency.

## Consequences
- **Positive**: Clean separation between offline deterministic testing and live cloud inference, with zero secret leakage and strict model-role safety.
