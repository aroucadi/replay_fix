# Research: Spec 151 — Live Meta Multimodal Wire Protocol & Full Muse Model Utilization (v1.5.1)

## 1. Live `sam-3.1` Image & Video Segmentation Wire Schema (`POST https://api.meta.ai/v1/responses`)
- **Decision**: Format all multimodal `sam-3.1` and `muse-spark-1.3` requests using the OpenAI/Meta Responses API content array inside `input`:
  - **Image Segmentation (`Images segmented`)**:
    ```json
    {
      "model": "sam-3.1",
      "input": [
        {
          "role": "user",
          "content": [
            { "type": "input_image", "image_url": "data:image/png;base64,<BASE64>" },
            { "type": "input_text", "text": "button" }
          ]
        }
      ]
    }
    ```
  - **Video Segmentation (`Video frames segmented`)**:
    ```json
    {
      "model": "sam-3.1",
      "input": [
        {
          "role": "user",
          "content": [
            { "type": "input_video", "video_url": "data:video/mp4;base64,<BASE64>" },
            { "type": "input_text", "text": "button" }
          ]
        }
      ]
    }
    ```
- **Rationale**: Live probing against `https://api.meta.ai/v1/responses` confirmed that legacy top-level `"image_base64"` returns `HTTP 400: unknown parameter image_base64`, whereas `input_image` and `input_video` content parts return `HTTP 200` and populate `usage.images_processed` and `usage.video_frames_processed`.
- **Wire Token Grammar**: Live `sam-3.1` outputs `<0f>0<|box;x1=1172;y1=596;x2=1238;y2=622;w=1280;h=720|><|mask;x=0;y=0;data=...|>,1<|box;...|>` with optional `;w=\d+;h=\d+` attributes inside `<|box;...|>`. When multiple boxes are returned across a full UI frame, `SamSegmenter::snap_to_centroid` selects the highest-scoring contour closest to the optical-flow interaction hotspot.

## 2. Live `muse-voice-transcribe-1.0` Multipart Form-Data Wire Schema (`POST https://api.meta.ai/v1/asr/transcribe`)
- **Decision**: Construct a standards-compliant RFC 7578 `multipart/form-data` request body with boundary `----ReplayFixAsrBoundary7MA4YWxkTrZu0gW`:
  - Part 1 (`name="request"`): `{"model":"muse-voice-transcribe-1.0","audioEncoding":"WAV"}`
  - Part 2 (`name="audio"; filename="audio.wav"`, `Content-Type: audio/wav`): Raw 24 kHz 16-bit mono PCM WAV bytes.
- **Rationale**: Live probing confirmed `/v1/asr/transcribe` rejects `application/json` with `HTTP 415` and requires both `'request'` and `'audio'` multipart parts with `"audioEncoding":"WAV"`, returning `HTTP 200 {"sessionId":"...","transcript":"...","audioDurationMs":1040,"turns":[...]}`.

## 3. Live `muse-spark-1.3` / `muse-wonder-1.0` Native Tool Calling (`web_search`) & `muse-image-1.0` Badge Generation
- **Decision**:
  - Attach `"tools": [{"type": "web_search"}]` on live `muse-spark-1.3` and `muse-wonder-1.0` requests when `enable_web_search` is set, and parse `web_search_call` / `function_call` items in the `output` array.
  - Add `MetaGatewayClient::generate_verification_badge` calling `POST https://api.meta.ai/v1/images/generations` with `{"model": "muse-image-1.0", "prompt": "..."}` and decoding `data[0].b64_json` to `.replayfix/keyframes/muse_verification_badge.webp`.
- **Rationale**: Exercises all 5 models in the Meta Model API catalog (`muse-wonder-1.0`, `muse-spark-1.3`, `sam-3.1`, `muse-voice-transcribe-1.0`, `muse-image-1.0`) and increments every usage counter on the Meta Developer Dashboard.
