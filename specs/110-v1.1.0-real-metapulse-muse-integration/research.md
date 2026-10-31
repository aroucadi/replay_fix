# Research: Spec 110 — Meta AI Endpoints, Fallbacks & MetaPulse Suite

## 1. Meta AI Endpoints & Fallback Model Matrix
- **Default Gateway URL**: `https://api.meta.ai/v1/responses` (overridable via `META_BASE_URL` or `GatewayConfig::with_base_url` for Baseten `https://inference.baseten.co/v1` or Together/OpenAI-compatible proxies).
- **Primary & Fallback Models**:
  - Spatial Segmentation: `sam-3.1` (`sam-3-1`)
  - Visual Intent & Action Synthesis: `muse-spark-1.3` (fallback: `meta-llama/Llama-3.2-11B-Vision-Instruct`)
  - Surgical Code Repair: `muse-wonder-1.0` (fallback: `meta-llama/Llama-3.3-70B-Instruct`)
  - Speech Transcription: `muse-voice-transcribe-1.0`

## 2. MetaPulse 20-Bug SWE-Bench Architecture
- Repository: `https://github.com/aroucadi/MetaPulse.git`
- Branches: `bug/RF-01` through `bug/RF-20`, each isolating a single frontend/fullstack defect with a paired `.mp4` cursor video in `benchmarks/metapulse/videos/`.
