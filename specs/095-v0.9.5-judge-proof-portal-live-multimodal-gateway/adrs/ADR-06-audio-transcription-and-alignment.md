# ADR-06: Audio Transcription, Temporal Alignment, and the Zero-Audio Rule

## Status
Accepted

## Context
Video recordings may include spoken bug narration transcribed via `muse-voice-transcribe-1.0`, while text/code repair models (`muse-spark-1.3`, `muse-wonder-1.0`) must never be billed for unnecessary audio payloads.

## Decision
1. **Zero-Audio Rule Enforcement at Gateway Layer**:
   - Any `GatewayRequest` targeting a text/code model (`muse-spark-1.3`, `muse-wonder-1.0`, `meta-models/muse-spark-open`) that sets `include_audio == true` or attaches audio bytes is rejected fail-closed with `CoreError::SecurityViolation("ZERO_AUDIO_RULE_VIOLATION: ...")` before any network call occurs.
2. **Verified Audio Metadata & Temporal Alignment**:
   - Audio inputs to `muse-voice-transcribe-1.0` validate WAV header sample rate (`24_000` Hz mono PCM expected when preprocessed by `VideoPreprocessor`), duration ($\le 60,000	ext{ms}$), and SHA-256 digest.
   - Never relabels un-resampled audio as 24 kHz; records actual inspected sample rate and channels.
   - Correlates transcript segment timestamps `[start_ms, end_ms]` with visual keyframe timestamps within a documented tolerance window ($\pm 250	ext{ms}$).

## Consequences
- **Positive**: Eliminates accidental audio token billing on text models and preserves verifiable provenance for audio-visual event alignment.
