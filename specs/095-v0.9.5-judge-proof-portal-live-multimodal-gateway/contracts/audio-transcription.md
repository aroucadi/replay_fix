# Contract: Audio Transcription & The Zero-Audio Rule

## 1. Zero-Audio Rule (Mandatory Invariant)
- **Principle**: Text-only tasks (such as code repair, preflight syntax checking, and diff generation) MUST NOT include audio attachments, process audio streams, or invoke audio transcription models.
- **Enforcement**: Gateways and request builders validate that if task type is not `AudioGroundedDefect`, audio data is strictly absent (`None`), preventing accidental billing of audio tokens.

## 2. Speech Transcription Specifications
- **Model**: `muse-voice-transcribe-1.0`
- **Audio Constraints**: Max duration 60 seconds; 16-bit PCM WAV (24 kHz mono preferred).
- **Alignment Tolerance**: Transcribed utterance timestamps must correlate with video keyframe timestamps within $\pm 250\text{ms}$ of observed click actions.
