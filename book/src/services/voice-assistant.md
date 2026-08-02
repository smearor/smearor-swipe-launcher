# voice_assistant (Service)

Local LLM-based voice assistant service with ReAct tool selection, speech recognition, and text-to-speech.

## Description

The voice_assistant service provides a fully local voice assistant:

1. **Speech-to-Text** — Uses `whisper-rs` for offline speech recognition
2. **LLM Reasoning** — Uses `llama-cpp-4` for local LLM inference with a ReAct (Reason + Act) loop
3. **Tool Selection** — Discovers available MCP tools from the registry and selects appropriate ones based on the user query
4. **Text-to-Speech** — Uses `espeak-ng` for offline TTS
5. **Memory** — Maintains conversation context using embeddings (`fastembed`) and a SQLite vector store

## Topics

| Topic                                | Direction         | Description                                         |
|--------------------------------------|-------------------|-----------------------------------------------------|
| `service.voice_assistant.command`    | Widget → Service  | Activate, deactivate, text input                    |
| `service.voice_assistant.status`     | Service → Widgets | State changes (idle, listening, thinking, speaking) |
| `service.voice_assistant.transcript` | Service → Widgets | Conversation transcript updates                     |

## MCP Integration

The voice assistant uses the MCP registry to:

- Build a tool catalog from all registered tools
- Select tools via the ReAct loop based on user queries
- Invoke tools via `InvokeToolMessage`
- Receive results via `InvokeToolResponse`

## MCP Resources

| Resource                       | Description                     |
|--------------------------------|---------------------------------|
| `voice_assistant://llm`        | Active LLM model details        |
| `voice_assistant://transcript` | Current conversation transcript |

## MCP Prompts

| Prompt                 | Description                       |
|------------------------|-----------------------------------|
| `launcher_overview`    | System overview for AI context    |
| `area_control_help`    | How to control areas              |
| `broker_message_guide` | Message broker usage guide        |
| `weather_query_guide`  | Weather query instructions        |
| `mpris_control_guide`  | Media player control instructions |
| `power_action_guide`   | Power management instructions     |

## Configuration

```toml
[[services]]
id = "voice_assistant"
path = "target/release/libsmearor_voice_assistant_service.so"
```

## Crate

- **Path**: `services/voice_assistant/`
- **Library**: `libsmearor_voice_assistant_service.so`
- **Model**: `model/voice_assistant/`
