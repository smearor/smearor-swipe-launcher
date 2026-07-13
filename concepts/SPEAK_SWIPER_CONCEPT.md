# Sprachsteuerung für den Smearor Swipe Launcher

Here is a comprehensive concept document for integrating a local Voice Assistant Agent into the *Smearor Swipe Launcher*. It aligns with the architectural
standards, code quality guidelines, and decoupled SOA patterns specified in `AGENTS.md`, `WEATHER_SERVICE_CONCEPT.md`, and `NETWORK_MENU_CONCEPT.md`.

---

# Concept: Voice Assistant Agent Service & Widget

This document describes the concept for a **Voice Assistant Agent Service** and an associated **Voice Assistant Widget** in the *Smearor Swipe Launcher*. The
system enables completely local, private, and low-latency voice control using state-of-the-art AI on the CPU. It records audio, converts speech to text via
Whisper, orchestrates an autonomous ReAct loop with a 1B–3B parameter Large Language Model (LLM), and interfaces directly with the central message broker to
dynamically discover and control tools from any active service or plugin (e.g., Weather, Wallpaper, Network, or App Launcher).

The system follows the launcher's decoupled SOA architecture:

1. **Model Crate (`model/voice_assistant`):** Shared structs, enums, topics, and stable ABI message formats.
2. **Service Crate (`services/voice_assistant`):** Background service implementing the local AI pipeline (Audio capture, Speech-to-Text, LLM ReAct loop) and
   acting as an in-memory MCP-like orchestrator via internal message routing.
3. **Widget Crate (`plugins/voice_assistant`):** A pure GTK4 touch-optimized UI tile displaying status, transcriptions, and activation states.

---

## 1. System Architecture & Data Flow

Because the architecture relies on completely local execution, the entire process is handled inside the launcher process space using cross-thread message
passing.

```
+---------------------------+                 +----------------------------------+
| Voice Assistant Widget    |                 | Voice Assistant Service          |
| (GTK4 UI Tile / Status)   |                 | (Singleton Background Executor)  |
+---------------------------+                 +----------------------------------+
              │                                                 │
              │  1. Toggle Recording (Click/Long-Press)         │
              ├────────────────────────────────────────────────>│ [Audio Capture: cpal]
              │                                                 │         │ (PCM Data)
              │                                                 │         ▼
              │                                                 │ [STT: whisper-rs]
              │                                                 │         │ (User Text Intent)
              │                                                 │         ▼
              │                                                 │ [LLM Loop: candle/llama.cpp]
              │                                                 │         │
              │                                                 │   === ReAct Loop Begins ===
              │                                                 │   2. Query Broker for Tools
              │                                                 │   3. Evaluate & Match Intent
              │                                                 │   4. Broadcast Target Topic/Payload
              │                                                 │   === ReAct Loop Ends ===
              │                                                 │
              │  5. Broadcast Status Update                     │
              |<────────────────────────────────────────────────┤

```

### The Autonomous Dynamic Execution Loop (ReAct Pattern)

When the user submits an intent, the service acts as an orchestration client. Instead of mapping plugins explicitly, it interacts with the central message
broker to inspect active topics and dynamic schema structures.

1. **Discovery**: The service inspects available services/plugins through the system’s message routing layer.
2. **Reasoning (LLM)**: The local LLM processes the user’s text input alongside the injected metadata of available capabilities.
3. **Execution**: The LLM outputs a structured payload (JSON-formatted string). The service interprets this output and broadcasts the command to the target
   topic.
4. **Observation**: The service subscribes to the outcome, pipes it back into the LLM context, and allows multi-step tasks (e.g., Check Weather
   Status $\rightarrow$ Condition met $\rightarrow$ Trigger Wallpaper Action).

---

## 2. Feature Scope

| Feature                            | Description                                                                                                                                                                                   |
|------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Local Audio Capture**            | Accesses physical hardware microphones via `cpal` to stream PCM audio channels safely into RAM buffers.                                                                                       |
| **Local Offline STT**              | Translates spoken audio buffers into raw strings using an optimized GGML `whisper-tiny` model (~75MB) running entirely on CPU.                                                                |
| **Dynamic Capabilities Injection** | Gathers descriptions, schemas, and topics from all currently registered launcher plugins dynamically.                                                                                         |
| **On-Device LLM Reasoning**        | Uses `candle` or `llama-cpp-2` to run an explicit 1B to 3B token-efficient model (e.g., `Qwen-2.5-1.5B-Instruct` or `Llama-3.2-1B-Instruct`) for zero-network dependency and maximum privacy. |
| **Contextual Multi-Step Chains**   | Allows complex operations like checking data from one plugin before passing a derived action into a second plugin.                                                                            |

---

## 3. Data Model (`model/voice_assistant/`)

Following the strict code isolation pattern, all public types are split into specialized single-file modules under `model/src/voice_assistant/`. To ensure FFI
and ABI stability across plugins via `#[stabby::stabby]`, standard heap structures are wrapped.

### Assistant Status State (`state.rs`)

```rust
#[stabby::stabby]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AssistantState {
    Idle,
    Listening,
    ProcessingSTT,
    ThinkingLLM,
    ExecutingAction(stabby::string::String),
    Error(stabby::string::String),
}

```

### Topic Definition (`topics.rs`)

```rust
pub const TOPIC_VOICE_COMMAND_TRIGGER: &str = "service.voice_assistant.trigger";
pub const TOPIC_VOICE_STATUS_UPDATE: &str = "service.voice_assistant.status";

```

### Message Structs (`messages.rs`)

```rust
#[stabby::stabby]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssistantStatusMessage {
    pub current_state: AssistantState,
    pub partial_transcript: stabby::string::String,
    pub active_error: stabby::option::Option<stabby::string::String>,
}

#[stabby::stabby]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VoiceTriggerMessage {
    pub activate: bool,
}

```

---

## 4. Background Service (`services/voice_assistant/`)

The service runs as an asynchronous worker utilizing the central runtime environment. It holds the reference states for hardware streaming, Whisper contexts,
and LLM static layers.

```rust
use std::sync::Arc;
use tokio::sync::mpsc;
use stabby::stabby;

pub struct VoiceAssistantService {
    state: AssistantState,
    whisper_context: Arc<tokio::sync::Mutex<whisper_rs::WhisperContext>>,
    llm_runner: Arc<LlmInferenceEngine>,
    command_rx: mpsc::Receiver<FfiMessage>,
    broker_tx: mpsc::Sender<FfiMessage>,
}

impl VoiceAssistantService {
    pub fn new(
        command_rx: mpsc::Receiver<FfiMessage>,
        broker_tx: mpsc::Sender<FfiMessage>
    ) -> Self {
        // Initialize whisper context using local ggml-tiny.bin model path safely.
        // Initialize candle / llama.cpp models securely on CPU.
        todo!()
    }

    async fn run_loop(&mut self) {
        while let Some(msg) = self.command_rx.recv().await {
            self.handle_incoming_message(msg).await;
        }
    }

    async fn execute_react_loop(&self, raw_user_prompt: &str) -> Result<(), AssistantError> {
        // 1. Query System Message Broker for dynamic capacities metadata descriptions.
        // 2. Loop local LLM generations until a structural 'final_answer' response schema is found.
        // 3. For each intermediate dynamic tool execution step, forward serialized FfiMessages via broker_tx.
        Ok(())
    }
}

```

---

## 5. UI Widget Plugin (`plugins/voice_assistant/`)

The user interface component is a standard touch-optimized tile widget built purely using GTK4 bindings. It provides continuous visual feedback on voice
processing phases and maps direct click handlers to wake or cancel requests.

### Component Structure

* **Idle View:** Shows a stylized icon or waveform state indicator (muted color).
* **Listening View:** Transforms into an active pulsing state with real-time text transcription updates pushed directly via cross-thread channels.
* **Thinking/Processing View:** Triggers an explicit loading animation reflecting CPU inferencing periods.

### UI Thread Interaction

To keep rendering completely fluid, UI transformations are executed entirely on the main thread via event driven updates using `glib::MainContext::spawn_local`.

```rust
// Inside Widget Message Handler implementation
fn handle_message(&self, topic: &str, payload: &[u8]) {
    if topic == TOPIC_VOICE_STATUS_UPDATE {
        if let Ok(status) = serde_json::from_slice::<AssistantStatusMessage>(payload) {
            glib::MainContext::spawn_local(glib::clone!(@weak self.ui_label as label, @weak self.status_spinner as spinner => move || {
                match status.current_state {
                    AssistantState::Listening => {
                        label.set_text(status.partial_transcript.as_str());
                        spinner.set_active(false);
                    }
                    AssistantState::ThinkingLLM => {
                        label.set_text("Thinking...");
                        spinner.set_active(true);
                    }
                    _ => {
                        spinner.set_active(false);
                    }
                }
            }));
        }
    }
}

```

---

## 6. Implementation Checklist & Development Rules

Following the **Clean Code Guidelines (`AGENTS.md`)**, the code must strictly respect these implementation mandates:

* [ ] **One Struct Per File**: Ensure every single type (`AssistantState`, `AssistantStatusMessage`, etc.) occupies its own distinct file within the module
  structure.
* [ ] **No Implicit Panics**: Absolutely no usage of `unwrap()`, `expect()`, or `panic!`. Use explicit error handling types mapping down to `thiserror` or
  `miette`.
* [ ] **ABI Stability Layer**: Ensure every single type traversing the FFI system edge utilizes the `#[stabby::stabby]` compiler layouts. Replace standard
  collections with verified `stabby` equivalents (`stabby::string::String`).
* [ ] **CPU Resource Cap**: Quantize the required LLM models to 4-bit (`Q4_K_M` or similar) inside the implementation layer to limit memory footprints to < 1.5
  GB RAM total overhead, protecting general launcher reactivity during inference tasks.

## Anforderung

Der [Smearor Swipe Launcher](https://github.com/smearor/smearor-swipe-launcher) stellt einen MCP Server bereit. Ich möchte, dass man den Smearor Swipe Launcher
mit Sprachbefehlen steuern kann. Wäre folgendes mit Rust möglich?

1. Einbau einer Bibliothek, die Sprachbefehle (Audio) entgegen nimmt und Umwandlung in Text
2. Einbau eines minimalen Agent mit einem kleinen bzw. effizienten LLM Model (muss auf einer CPU laufen können) bekommt die Sprachbefehle eingegeben
3. Der minimale Agent benutzt den MCP-Server des Smearor Swipe Launcher um ihn zu steuern

## Machbarkeit und Analyse

**Ja, das ist absolut machbar und Rust ist dafür sogar eine hervorragende Wahl.** Rust bietet durch sein Speichermanagement und die Performance eine ideale
Grundlage, um Audioverarbeitung, lokale LLMs auf der CPU und MCP-Kommunikation effizient in einer einzigen, schlanken Anwendung zu bündeln.

Hier ist die Analyse, wie du die drei Schritte konkret in Rust umsetzen kannst:

---

### 1. Sprache-zu-Text (STT) in Rust

Um Audio aufzunehmen und in Text umzuwandeln, ohne eine externe Cloud-API (wie Google oder OpenAI) anzufragen, ist **Whisper** von OpenAI der aktuelle
Goldstandard. Es gibt eine native Portierung in C/C++ (`whisper.cpp`), für die es exzellente Rust-Bindings gibt.

* **Bibliothek:** [whisper-rs](https://codeberg.org/tazz4843/whisper-rs) (Bindings für `whisper.cpp`).
* **Modell-Größe:** Für einfache Sprachbefehle reicht das `tiny.en` (oder `tiny` für Multilingual/Deutsch) Modell vollkommen aus. Es ist nur ca. 75 MB groß,
  extrem schnell und läuft auf fast jeder modernen CPU in Echtzeit.
* **Audio-Aufnahme:** Mit Bibliotheken wie `cpal` (Cross-Platform Audio Library) kannst du das Mikrofon anzapfen und die Audiodaten direkt an Whisper übergeben.

---

### 2. Minimaler Agent & Effizientes LLM auf der CPU

Ein LLM auf einer CPU laufen zu lassen, war früher träge, ist heute dank quantisierter Modelle (GGUF-Format) aber erstaunlich performant. Da du nur Befehle für
den Launcher interpretieren möchtest, reicht ein sehr kleines Modell (z. B. ein **1B bis 3B Parameter Modell** wie *Llama-3.2-1B-Instruct* oder
*Qwen-2.5-1.5B-Instruct*).

* **Inferenz-Engine (Rust):** * **`llama-cpp-2`** (Rust-Bindings für `llama.cpp`): Das ist die ausgereifteste Option für CPU-Inferenz. Sie nutzt
  CPU-Erweiterungen wie AVX2 oder AVX-512 optimal aus.
* [candle](https://github.com/huggingface/candle) (von Hugging Face): Eine reine, extrem performante Machine-Learning-Bibliothek in Rust. Sie erlaubt es dir,
  LLMs direkt in Rust ohne C-Abhängigkeiten auszuführen.


* **Ressourcenverbrauch:** Ein auf 4-Bit quantisiertes 1B-Modell benötigt gerade einmal rund 1 GB RAM und antwortet auf einer modernen CPU innerhalb weniger
  Millisekunden – perfekt für ein Hintergrund-Tool.

---

### 3. Steuerung des MCP-Servers

Da der *Smearor Swipe Launcher* bereits einen MCP-Server bereitstellt, muss dein Rust-Programm als **MCP-Client** agieren.

* **Der Ablauf:**

1. Dein Programm startet den MCP-Server des Launchers als Subprozess (meist über Standard-I/O / `stdin`/`stdout`).
2. Beim Start fragt dein Rust-Client den MCP-Server nach seinen verfügbaren **Tools** (z. B. `open_app`, `close_app`, `switch_workspace`).
3. Diese Tool-Definitionen (Namen und JSON-Schemas) gibst du deinem lokalen LLM im System-Prompt mit (das sogenannte *Tool Calling* oder *Function Calling*).
4. Wenn du sagst: *"Öffne bitte den Browser"*, erkennt das LLM, dass es das Tool `open_app(app_name: "browser")` aufrufen muss.
5. Dein Rust-Code nimmt diese strukturierte Antwort des LLMs entgegen und schickt den entsprechenden JSON-RPC-Befehl an den MCP-Server des Launchers.


* **Bibliothek:** Da das Protokoll auf JSON-RPC basiert, kannst du die Kommunikation entweder selbst kurz schreiben (`serde_json` für die Serialisierung und
  `tokio::process` für die Pipes) oder auf Community-Crates wie `mcp-core` bzw. offizielle SDK-Implementierungen zurückgreifen, falls verfügbar.

---

### Architektur-Skizze deines Systems

```
[ Mikrofon ] ──(Audio)──> [ cpal / whisper-rs ]
                                 │
                            (Text-String)
                                 ▼
                     [ Kleines LLM (Candle/llama.cpp) ]
                                 │
                        (Gewähltes MCP Tool)
                                 ▼
[ Rust MCP Client ] ──(JSON-RPC via Stdio)──> [ Smearor Swipe Launcher MCP Server ]

```

### Fazit & Ausblick

Das Projekt ist ein genialer Use-Case für Rust. Weil alles lokal läuft (STT, LLM und MCP), hast du **keine Latenz durch das Internet** und **100% Datenschutz**,
da keine Audiodaten dein Gerät verlassen.

*Einziger Tipp vorab:* Wenn die Befehle sehr standardisiert sind (z. B. immer nur "Öffne X", "Schließe Y"), könnte ein LLM sogar "Overkill" sein. Du könntest
dann alternativ im ersten Schritt prüfen, ob ein einfaches regelbasiertes Matching oder ein kleinerer Intent-Classifier (wie `rasa`-ähnliche Ansätze in Rust)
ausreicht. Wenn die Befehle aber komplex und natürlich formuliert werden sollen (z. B. *"Es ist zu voll auf dem Bildschirm, mach mal alles zu außer das
Terminal"*), dann ist der Weg über das lokale LLM genau der richtige.

Das macht das Projekt extrem mächtig! Da der Launcher den MCP-Server direkt **integriert** hat und Plugins dynamisch hinzuladen kann, baust du im Grunde das
perfekte lokale "Operating System Agent"-Setup.

Das Schöne an MCP ist: Das LLM muss die Plugins (wie Wetter oder Wallpaper) nicht im Voraus kennen. Es fragt beim Start einfach die Liste der Tools (
`list_tools`) ab, liest deren JSON-Schema-Beschreibungen und kombiniert sie intelligent, um selbst komplexe, mehrstufige Aufgaben (wie in deinen Beispielen) zu
lösen.

Hier sind die Antworten auf deine Fragen und wie du diese Logik in Rust implementierst:

---

## 1. Woher bekommt man das `tiny`-Modell für Whisper?

Die Whisper-Modelle für `whisper.cpp` (und damit für `whisper-rs`) müssen in einem speziellen Format vorliegen, dem **ggml**-Format.

* **Direkter Download von Hugging Face:**
  G there is a dedicated repository by Georgi Gerganov (the creator of `whisper.cpp`):
* Multilingual (unterstützt
  Deutsch): [whisper-tiny-ggml](https://www.google.com/search?q=https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin) (~75 MB)
* Nur Englisch (etwas präziser, falls du nur Englisch
  sprichst): [whisper-tiny.en-ggml](https://www.google.com/search?q=https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin)


* **Automatisiert via Skript:**
  Wenn du das `whisper.cpp` Repository klonst, gibt es dort im Ordner `models` ein Skript namens `./download-ggml-model.sh tiny`, das die Datei automatisch
  zieht. In deinem Rust-Projekt reichst du beim Initialisieren von `whisper-rs` einfach den Pfad zu dieser `.bin`-Datei ein.

---

## 2. Integration: MCP im selben Prozess

Da der MCP-Server direkt im Launcher integriert ist und du deine Sprachsteuerung vermutlich *in* den Launcher einbauen oder ihn als Bibliothek einbinden willst,
entfällt der Overhead von Standard-I/O oder TCP-Sockets.

In Rust nutzt du dafür den **In-Memory Transport Layer**. Das bedeutet, dass deine Sprachsteuerung und der MCP-Server über interne Rust-Kanäle (
`tokio::sync::mpsc` oder ähnliches) direkt im RAM miteinander kommunizieren. Das ist extrem schnell (Latenz gegen Null).

---

## 3. Umsetzung der Multi-Step-Logik (ReAct Pattern)

Um die von dir beschriebenen Beispiele ("Wenn Temperatur > 30 Grad, dann...") zu ermöglichen, reicht es nicht, das LLM nur *einmal* zu fragen. Du musst ein
sogenanntes **ReAct** (Reasoning and Acting) Pattern oder eine **Agenten-Schleife** implementieren.

### Der Ablauf im Detail (Beispiel 1: Wetter & Wallpaper)

```
[Sprachbefehl] ──> "Wenn >30°C, ändere Wallpaper..."
                         │
                         ▼
┌───────────────── Agenten-Schleife (Rust) ────────────────┐
│                                                          │
│  1. LLM liest verfügbare Tools (weather, wallpaper)      │
│  2. LLM entscheidet: "Ich muss zuerst das Wetter prüfen" │
│  3. LLM gibt Tool-Call aus: `get_temperature()`          │
│                                                          │
│  4. Rust fängt Call ab ──> Ruft integrierten MCP auf     │
│  5. MCP gibt zurück: "32 Grad"                           │
│                                                          │
│  6. Rust füttert "32 Grad" zurück ins LLM                │
│  7. LLM entscheidet: ">30, also Wallpaper ändern!"       │
│  8. LLM gibt Tool-Call aus: `set_wallpaper("sunny")`     │
│                                                          │
│  9. Rust fängt Call ab ──> Ruft integrierten MCP auf     │
│                                                          │
└──────────────────────────────────────────────────────────┘
                         │
                         ▼
                 [ Aktion ausgeführt ]

```

### Wie sieht das im Code aus? (Lokale CPU Inferenz mit `candle`)

Wenn du Hugging Face's `candle` oder `llama-cpp-2` nutzt, sieht deine Hauptschleife in Rust pseudocode-artig so aus:

```rust
// 1. Hole alle aktuellen Tools vom integrierten MCP Server
let tools = mcp_server.list_tools().await?;

// 2. Erstelle den System-Prompt für dein 1B/3B LLM
let mut system_prompt = format!(
    "Du bist ein Desktop-Assistent. Dir stehen folgende Tools zur Verfügung: {:?}. 
    Antworte im JSON-Format: entweder mit {{'tool': 'name', 'args': {{...}}}} oder mit {{'final_answer': '...'}}",
    tools
);

// 3. Der User-Befehl aus Whisper
let user_command = "Wenn die heutige Temperatur über 30 Grad ist, ändere das sonnige Wallpaper...";
let mut conversation_history = vec![user_command.to_string()];

// 4. Die Agenten-Schleife
loop {
// Generiere Antwort vom lokalen LLM (z.B. Llama-3.2-1B)
let llm_response = local_llm.generate( & system_prompt, & conversation_history).await ?;

// Parsen, was das LLM tun möchte
if let Some(tool_call) = parse_tool_call( & llm_response) {
println ! ("LLM möchte Tool aufrufen: {}", tool_call.name);

// Direkter, internen Aufruf des integrierten MCP-Servers!
let tool_result = mcp_server.call_tool( & tool_call.name, tool_call.args).await ?;

// Das Ergebnis wird der Konversation hinzugefügt, damit das LLM im nächsten Schritt damit arbeiten kann
conversation_history.push(format ! ("Tool {} Ergebnis: {}", tool_call.name, tool_result));
} else if let Some(final_answer) = parse_final_answer(& llm_response) {
println ! ("Assistent fertig: {}", final_answer);
break;
}
}

```

### Warum das mit kleinen Modellen (1B - 3B) klappt

Früher brauchte man für solches "Tool Calling" riesige Modelle. Moderne kleine Modelle wie **Llama-3.2-3B-Instruct** oder **Qwen-2.5-1.5B-Instruct** wurden
explizit auf das Verstehen von JSON-Strukturen und Tool-Aufrufe trainiert. Da sie lokal über `candle` oder `llama.cpp` laufen, dauert ein Durchlauf der obigen
Schleife auf einer CPU oft nur wenige Hundert Millisekunden, was die Interaktion flüssig macht.