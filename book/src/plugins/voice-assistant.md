# voice_assistant (Plugin)

Voice assistant widget providing speech-to-text, LLM reasoning, and text-to-speech. Fully local, using `whisper-rs` for STT and `espeak-ng` for TTS.

## Description

The voice assistant widget communicates with the [voice_assistant service](../services/voice-assistant.md). It displays a microphone icon with state-dependent
colors and provides visual feedback during listening, thinking, and speaking phases.

## States

| State     | Icon               | Description                         |
|-----------|--------------------|-------------------------------------|
| Idle      | Microphone icon    | Waiting for wake word or activation |
| Listening | Pulsing microphone | Recording audio                     |
| Thinking  | Spinner/brain icon | LLM processing                      |
| Speaking  | Speaker icon       | TTS playback                        |
| Error     | Error icon         | Error occurred                      |

## Configuration

```toml
[voice_assistant_widget]
icon_size = 32
```

| Field             | Type             | Description                      |
|-------------------|------------------|----------------------------------|
| `icon_size`       | `i32`            | Icon size in pixels              |
| State icon fields | `Option<String>` | 7 configurable state icon fields |

## Action Bindings

Supports all [action binding types](../features/action-bindings.md). Click activates/deactivates the assistant.

## Related Service

- [voice_assistant (service)](../services/voice-assistant.md) — LLM inference, ReAct tool selection, MCP integration, memory

## Crate

- **Path**: `plugins/voice_assistant/`
- **Library**: `libsmearor_voice_assistant_widget.so`
- **Model**: `model/voice_assistant/`
