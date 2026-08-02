# Speak Swiper Performance Optimization Strategy

This document outlines comprehensive performance optimization strategies for the Voice Assistant Service in the Smearor Swipe Launcher, focusing on reducing LLM
inference latency and improving overall system responsiveness.

---

## Executive Summary

**Current Performance Issues:**

- **LLM Inference**: 2+ minutes for 2562 tokens (CPU-only)
- **Session Reset**: 20ms overhead per request
- **Tool Invocation**: 2-3 seconds LLM generation per tool call
- **Total Response Time**: 2+ minutes from input to final answer

**Target Performance Goals:**

- **LLM Inference**: 15-30 seconds (5-10x improvement)
- **Session Reset**: 0-5ms (selective clearing)
- **Tool Invocation**: < 1 second (direct routing)
- **Total Response Time**: 30-45 seconds (3-4x improvement)

---

## Model Overview

### Current Model Inventory

The voice assistant system utilizes multiple AI models for different tasks, each optimized for specific use cases:

#### **LLM Models (Language Generation)**

| Model                     | File                                | Size    | Purpose                            | Performance                                 |
|---------------------------|-------------------------------------|---------|------------------------------------|---------------------------------------------|
| **Qwen2.5-1.5B-Instruct** | `qwen2.5-1.5b-instruct-q4_k_m.gguf` | 1.04 GB | Primary LLM for command processing | Fast inference, good German support         |
| **Qwen2.5-3B-Instruct**   | `qwen2.5-3b-instruct-q4_k_m.gguf`   | 1.96 GB | Alternative LLM for complex tasks  | Better reasoning, slower                    |
| **Gemma-4-E4B-it**        | `gemma-4-E4B-it-Q4_K_M.gguf`        | 4.96 GB | High-capacity LLM (experimental)   | Advanced capabilities, heavy resource usage |

**Current Active Model**: `qwen2.5-1.5b-instruct-q4_k_m.gguf`

- **Why this model?**: Optimal balance between performance and resource usage
- **Quantization**: Q4_K_M (4-bit, medium quality) - good compression with acceptable accuracy
- **Context Window**: 4096 tokens (configurable)
- **Language Support**: Excellent German and English capabilities

#### **Speech Recognition Models (Whisper)**

| Model                      | File                           | Size   | Purpose                    | Performance                   |
|----------------------------|--------------------------------|--------|----------------------------|-------------------------------|
| **Whisper Tiny**           | `ggml-tiny.bin`                | 74 MB  | Default speech recognition | Fastest, basic accuracy       |
| **Whisper Tiny.en**        | `ggml-tiny.en-q5_1.bin`        | 31 MB  | English-only recognition   | Very fast, English optimized  |
| **Whisper Base**           | `ggml-base.bin`                | 141 MB | Balanced recognition       | Good accuracy, moderate speed |
| **Whisper Large-v3-turbo** | `ggml-large-v3-turbo-q5_0.bin` | 548 MB | High-accuracy recognition  | Best accuracy, slower         |

**Current Active Model**: `ggml-tiny.bin`

- **Why this model?**: Fastest transcription for real-time voice interaction
- **Language**: Configurable (default: German "de")
- **Sample Rate**: 16 kHz mono
- **Latency**: 5-10 seconds for short commands (CPU-only)

#### **Embedding Models (FastEmbed)**

| Model                           | Name                          | Size    | Purpose                   | Performance          |
|---------------------------------|-------------------------------|---------|---------------------------|----------------------|
| **BGE-Small-EN-v1.5-Q**         | `bge-small-en-v1.5-q`         | ~133 MB | Default semantic memory   | Fast, good English   |
| **BGE-Small-EN-v1.5**           | `bge-small-en-v1.5`           | ~133 MB | Higher quality embeddings | Better accuracy      |
| **All-MiniLM-L6-v2**            | `all-MiniLM-L6-v2`            | ~90 MB  | Lightweight alternative   | Very fast, smaller   |
| **Paraphrase-ML-MiniLM-L12-v2** | `paraphrase-ml-minilm-l12-v2` | ~430 MB | Paraphrase detection      | Specialized use case |

**Current Active Model**: `bge-small-en-v1.5-q`

- **Why this model?**: Quantized version for faster processing
- **Dimensions**: 384-dimensional embeddings
- **Max Sequence Length**: 512 tokens
- **Use Case**: Semantic memory retrieval and fact injection

### Model Usage Patterns

#### **1. Voice Processing Pipeline**

```
Audio Input → Whisper (ggml-tiny.bin) → Text → LLM (qwen2.5-1.5b) → Commands
```

#### **2. Semantic Memory Pipeline**

```
User Query → FastEmbed (bge-small-en-v1.5-q) → Embeddings → Vector Search → Context Injection
```

#### **3. Tool Selection Pipeline**

```
User Query → Nucleo-Matcher → Tool Catalog → Selected Tools → Execution
```

### Model Selection Strategy

#### **Performance vs. Quality Trade-offs**

| Use Case                        | Recommended Model               | Reason                  |
|---------------------------------|---------------------------------|-------------------------|
| **Real-time Voice Commands**    | Qwen2.5-1.5B + Whisper Tiny     | Fastest response time   |
| **Complex Reasoning**           | Qwen2.5-3B + Whisper Base       | Better accuracy         |
| **Resource-Constrained**        | Qwen2.5-1.5B + Whisper Tiny.en  | Minimal memory usage    |
| **High-Accuracy Transcription** | Qwen2.5-1.5B + Whisper Large-v3 | Best speech recognition |

#### **Hardware-Specific Optimization**

**Ryzen 5 8500G (iGPU):**

- **LLM**: Qwen2.5-1.5B (GPU offloading: 8-12 layers)
- **Whisper**: Tiny or Base (GPU acceleration)
- **Embeddings**: BGE-Small-Q (ONNX GPU)
- **Memory Budget**: ~2GB total

**Ryzen 9 9950X3D + RX 7900 XTX:**

- **LLM**: Qwen2.5-3B or Gemma-4E4B (GPU offloading: 20+ layers)
- **Whisper**: Large-v3-turbo (GPU acceleration)
- **Embeddings**: BGE-Small (ONNX GPU)
- **Memory Budget**: ~4GB total

### Model Storage and Management

#### **Current Storage Location**

```
$HOME/git/smearor-swipe-launcher/models/
├── qwen2.5-1.5b-instruct-q4_k_m.gguf (1.04 GB)
├── qwen2.5-3b-instruct-q4_k_m.gguf (1.96 GB)
├── gemma-4-E4B-it-Q4_K_M.gguf (4.96 GB)
├── ggml-tiny.bin (74 MB)
├── ggml-tiny.en-q5_1.bin (31 MB)
├── ggml-base.bin (141 MB)
└── ggml-large-v3-turbo-q5_0.bin (548 MB)
```

#### **Storage Requirements**

- **Total Current Usage**: ~8.8 GB
- **Recommended Free Space**: 15-20 GB for future models
- **Model Loading Strategy**: Load on demand, keep active models in memory

#### **Model Versioning Strategy**

- **Semantic Versioning**: Use model version in filenames
- **Backward Compatibility**: Keep previous versions during testing
- **Automated Updates**: Implement model download and verification

### Future Model Considerations

#### **Potential Model Upgrades**

1. **LLM**: Qwen2.5-7B for better reasoning (if hardware permits)
2. **Whisper**: Whisper-v3 for improved multilingual support
3. **Embeddings**: Multilingual BGE models for better German support

#### **Model Optimization Opportunities**

1. **Dynamic Model Selection**: Choose model based on query complexity
2. **Model Quantization**: Explore INT8/INT4 for further compression
3. **Model Pruning**: Remove unused layers for specific use cases
4. **Knowledge Distillation**: Create smaller, specialized models

---

## 1. GPU Acceleration Strategy

### 1.1 Vulkan Backend Implementation (Primary Strategy)

**Aktueller Stand:**

- Die Voice Assistant Service verwendet ausschließlich CPU-basierte Inferenz mit llama-cpp-4
- Die Konfiguration ist auf `backend = "cpu"` festgelegt, was zu sequenzieller Verarbeitung auf dem Prozessor führt
- Es findet keine GPU-Beschleunigung statt, obwohl die Hardware (Ryzen 5 8500G iGPU, Ryzen 9 9950X3D + RX 7900 XTX) Vulkan-Unterstützung bietet

**Warum der aktuelle Stand nicht optimal ist:**

- **Performance-Bottleneck**: LLM-Inferenz dauert 2+ Minuten für 2562 Tokens, was für interaktive Voice-Assistant-Anwendung zu langsam ist
- **CPU-Overlast**: Die CPU wird während der Inferenz vollständig ausgelastet, was andere Systemprozesse verlangsamt
- **Energieeffizienz**: GPU-Beschleunigung wäre deutlich energieeffizienter für parallele Matrix-Operationen
- **Skalierbarkeit**: Bei komplexeren Anfragen oder längeren Kontexten wird das Problem noch deutlicher

**Target Hardware:**

- **Ryzen 5 8500G**: Integrated Radeon Graphics (RDNA2/RDNA3) - Shared RAM
- **Ryzen 9 9950X3D**: Radeon RX 7900 XTX (24GB VRAM) - Dedicated VRAM

**Wie es verbessert werden soll:**

1. **Vulkan Backend Integration**: llama-cpp-4 wird mit Vulkan-Feature kompiliert und konfiguriert
2. **Dynamische Backend-Erkennung**: Automatische Erkennung der optimalen Hardware (iGPU vs dGPU vs CPU)
3. **Adaptive VRAM-Budgetierung**: Intelligente Allokation von GPU-Speicher basierend auf verfügbarer VRAM
4. **Fallback-Strategie**: Automatische Umschaltung auf CPU bei GPU-Problemen

**Dynamic VRAM Budgeting:**

```rust
#[derive(Debug, Clone)]
pub struct GpuConfig {
    pub backend: LlmBackend,
    pub device_type: DeviceType,
    pub vram_budget_mb: usize,
    pub n_gpu_layers: i32,  // Dynamic layer offloading
    pub enable_hipblas: bool,  // Config option for ROCm/HIPBLAS
}

impl GpuConfig {
    pub fn calculate_optimal_layers(model_size_mb: usize, available_vram_mb: usize) -> i32 {
        // q4_k_m model: ~1.5-2GB total
        // Reserve 512MB for system/overhead
        let usable_vram = available_vram_mb.saturating_sub(512);

        if usable_vram >= model_size_mb {
            // Full model fits on GPU
            -1  // All layers on GPU
        } else {
            // Calculate partial offloading
            let ratio = usable_vram as f64 / model_size_mb as f64;
            (ratio * 32.0).round() as i32  // Approximate layer count
        }
    }

    pub fn detect_gpu_memory() -> usize {
        match self.device_type {
            DeviceType::IntegratedGpu => {
                // For iGPU, use 25% of system RAM (conservative)
                let system_ram = get_system_ram_mb();
                (system_ram / 4)
            }
            DeviceType::DiscreteGpu => {
                // Query actual VRAM from Vulkan
                query_vram_info().unwrap_or(4096)
            }
            DeviceType::Cpu => 0,
        }
    }

    pub fn detect_optimal_config() -> Self {
        Self::detect_optimal_config_with_hipblas(true)
    }

    pub fn detect_optimal_config_with_hipblas(enable_hipblas: bool) -> Self {
        // GPU-Erkennung mit AMD-spezifischer Optimierung
        if vulkan_available() {
            // GPU-Typ erkennen und VRAM budgetieren
            if has_discrete_gpu() {
                let vram_mb = get_available_vram().saturating_sub(512); // 512MB Reserve

                // AMD dGPU: Prüfe ROCm/HIPBLAS Verfügbarkeit für maximale Performance
                if is_amd_discrete_gpu() && enable_hipblas && hipblas_libraries_available() {
                    debug!("AMD dGPU detected with ROCm/HIPBLAS support - using HIPBLAS backend");
                    GpuConfig {
                        backend: LlmBackend::Hipblas,
                        device_type: DeviceType::DiscreteGpu,
                        vram_budget_mb: vram_mb,
                        n_gpu_layers: calculate_optimal_layers(2048, vram_mb),
                        enable_hipblas: true,
                    }
                } else {
                    debug!("dGPU detected but no ROCm/HIPBLAS or disabled - falling back to Vulkan");
                    GpuConfig {
                        backend: LlmBackend::Vulkan,
                        device_type: DeviceType::DiscreteGpu,
                        vram_budget_mb: vram_mb,
                        n_gpu_layers: calculate_optimal_layers(2048, vram_mb),
                        enable_hipblas: false,
                    }
                }
            } else {
                // iGPU: Immer Vulkan (perfekte Universallösung)
                let system_ram_mb = get_system_memory();
                let vram_mb = system_ram_mb / 4;
                debug!("iGPU detected - using Vulkan backend");
                GpuConfig {
                    backend: LlmBackend::Vulkan,
                    device_type: DeviceType::IntegratedGpu,
                    vram_budget_mb: vram_mb,
                    n_gpu_layers: calculate_optimal_layers(2048, vram_mb),
                    enable_hipblas: false,
                }
            }
        } else {
            // CPU Fallback
            debug!("No GPU acceleration available - using CPU backend");
            GpuConfig {
                backend: LlmBackend::Cpu,
                device_type: DeviceType::Cpu,
                vram_budget_mb: 0,
                n_gpu_layers: 0,
                enable_hipblas: false,
            }
        }
    }

    /// Check if ROCm/HIPBLAS is available for AMD GPUs
    fn hipblas_available() -> bool {
        // Check for LLAMA_HIPBLAS=1 environment variable (fallback)
        std::env::var("LLAMA_HIPBLAS").is_ok() &&
            std::env::var("LLAMA_HIPBLAS").unwrap_or_default() == "1"
    }

    /// Check if ROCm/HIPBLAS libraries are actually available on the system
    fn hipblas_libraries_available() -> bool {
        use glob::glob;

        // Check for ROCm/HIPBLAS library presence in common locations
        std::path::Path::new("/opt/rocm/lib/libhipblas.so").exists() ||
            std::path::Path::new("/usr/lib/x86_64-linux-gnu/libhipblas.so").exists() ||
            std::path::Path::new("/usr/local/rocm/lib/libhipblas.so").exists() ||
            // Ollama-specific ROCm installation paths (v7.2)
            std::path::Path::new("/usr/local/lib/ollama/rocm_v7_2/libhipblas.so").exists() ||
            std::path::Path::new("/usr/local/lib/ollama/rocm_v7_2/libhipblas.so.3").exists() ||
            std::path::Path::new("/usr/local/lib/ollama/rocm_v7_2/libhipblaslt.so").exists() ||
            std::path::Path::new("/usr/local/lib/ollama/rocm_v7_2/libhipblaslt.so.1").exists() ||
            // Generic Ollama ROCm path pattern for any version
            glob("/usr/local/lib/ollama/rocm_v*/libhipblas.so*")
                .unwrap_or_default()
                .next()
                .is_some()
    }

    /// Check if the discrete GPU is AMD (for ROCm optimization)
    fn is_amd_discrete_gpu() -> bool {
        // This would query GPU vendor information
        // For now, assume AMD if we have a discrete GPU on Linux
        cfg!(target_os = "linux") && has_discrete_gpu()
    }
}

#[derive(Debug, Clone)]
pub enum DeviceType {
    IntegratedGpu,
    DiscreteGpu,
    Cpu,
}

#[derive(Debug, Clone)]
pub enum LlmBackend {
    Vulkan,
    Hipblas,  // AMD ROCm/HIPBLAS for maximum performance
    Cpu,
}
```

**AMD GPU Optimierung & Shader-Stuttering-Vermeidung:**

**Shader-Stuttering Problem:**
Vulkan-Shader werden häufig erst beim ersten Modellstart für die jeweilige GPU-Architektur kompiliert. Dies kann beim ersten Starten der Pipeline zu einer
merklichen Verzögerung (Spike) führen.

**Lösung: Shader Caching & Vorab-Kompilierung**

```rust
impl GpuConfig {
    /// Pre-compile Vulkan shaders to avoid first-run stuttering
    pub fn precompile_shaders(&self) -> Result<(), GpuError> {
        match self.backend {
            LlmBackend::Vulkan => {
                debug!("Pre-compiling Vulkan shaders for {:?}", self.device_type);

                // Trigger shader compilation with small test model
                let test_model_path = "models/test-quant.gguf";
                if std::path::Path::new(test_model_path).exists() {
                    let test_config = GpuConfig {
                        backend: self.backend.clone(),
                        device_type: self.device_type.clone(),
                        vram_budget_mb: 256, // Small budget for test
                        n_gpu_layers: 1,      // Minimal layers
                    };

                    // Load and immediately unload to trigger shader compilation
                    if let Ok(_) = LlmInferenceEngine::load(&test_config) {
                        debug!("Vulkan shaders pre-compiled successfully");
                    }
                }
                Ok(())
            }
            LlmBackend::Hipblas => {
                debug!("ROCm/HIPBLAS backend - no shader pre-compilation needed");
                Ok(())
            }
            LlmBackend::Cpu => Ok(()),
        }
    }

    /// Warm up GPU with small inference to trigger shader compilation
    pub fn warmup_gpu(&self) -> Result<(), GpuError> {
        match self.device_type {
            DeviceType::DiscreteGpu => {
                debug!("Warming up discrete GPU to avoid shader stuttering");
                self.precompile_shaders()
            }
            DeviceType::IntegratedGpu => {
                debug!("iGPU detected - shader compilation typically faster");
                Ok(())
            }
            DeviceType::Cpu => Ok(()),
        }
    }
}
```

**ROCm/HIPBLAS Performance-Vorteile:**

- **Wave64/Matrix-Core-Auslastung**: Maximale mathematische Leistung für AMD RDNA3
- **Spezifische Optimierungen**: Bessere Ausnutzung der RX 7900 XTX Hardware
- **Linux-Native**: Bessere Integration als generisches Vulkan-Compute

**Config-Option für HIPBLAS:**

```rust
// In der Konfigurationsdatei (services.toml)
[voice_assistant.llm]
backend = "auto" # auto , vulkan, hipblas, cpu
enable_hipblas = true # ROCm/HIPBLAS für AMD dGPU aktivieren

// Oder programmatisch:
let config = GpuConfig::detect_optimal_config_with_hipblas(true);
```

**Backend-Selection-Strategie:**

```rust
// Konfiguration für maximale Performance
pub fn create_optimized_config(enable_hipblas: bool) -> GpuConfig {
    let mut config = GpuConfig::detect_optimal_config_with_hipblas(enable_hipblas);

    // GPU Warmup für Shader-Stuttering-Vermeidung
    if let Err(e) = config.warmup_gpu() {
        warn!("GPU warmup failed: {}, continuing with cold start", e);
    }

    config
}

// Aus der Konfiguration lesen
pub fn create_config_from_settings(settings: &VoiceAssistantSettings) -> GpuConfig {
    create_optimized_config(settings.llm.enable_hipblas)
}
```

**Vorteile der Config-Option:**

- **Flexibilität**: Benutzer können HIPBLAS bei Bedarf deaktivieren
- **Kompatibilität**: Fallback auf Vulkan bei ROCm-Problemen
- **Testbarkeit**: Einfaches Umschalten zwischen Backends für Tests
- **Zukunftssicher**: Leichte Erweiterung für weitere GPU-Optionen

**Erwartete Performance-Verbesserungen:**

- **Ryzen 5 8500G (iGPU)**: 5-8x Geschwindigkeitsverbesserung (2+ Min → 20-30 Sek)
- **Ryzen 9 9950X3D (RX 7900 XTX)**:
    - **Vulkan**: 10-15x Geschwindigkeitsverbesserung (2+ Min → 15-20 Sek)
    - **ROCm/HIPBLAS**: 12-18x Geschwindigkeitsverbesserung (2+ Min → 12-18 Sek)
- **Shader-Stuttering**: Eliminiert durch Vorab-Kompilierung
- **Memory-Effizienz**: Shared System RAM für iGPU, dedizierte VRAM für discrete GPU
- **CPU-Entlastung**: CPU wird für andere Aufgaben freigegeben während GPU die Inferenz übernimmt

### 1.2 OpenCL Backend (Fallback Strategy)

**Note:** `llama-cpp-4` currently only supports Vulkan, not OpenCL. For systems without Vulkan support, the CPU backend will be used as fallback.

**Use Cases:**

- Systems without Vulkan support
- Legacy hardware compatibility
- Development/testing environments

### 1.3 Backend Detection Logic

```rust
impl LlmConfig {
    pub fn detect_optimal_backend() -> (LlmBackend, DeviceType) {
        // Priority: Vulkan > CPU (llama-cpp-4 only supports Vulkan)
        if vulkan_available() {
            if has_discrete_gpu() {
                (LlmBackend::Vulkan, DeviceType::DiscreteGpu)
            } else {
                (LlmBackend::Vulkan, DeviceType::IntegratedGpu)
            }
        } else {
            (LlmBackend::Cpu, DeviceType::Cpu)
        }
    }
}
```

---

## 2. Session Management Optimization

### 2.1 Smart Session Reset Strategy

**Aktueller Stand:**

- Der Voice Assistant Service setzt die LLM-Session bei jeder neuen Anfrage komplett zurück
- In `llm.rs` wird `session = None` und `last_system_prompt = None` bei jedem neuen Command aufgerufen
- Dies führt zu vollständiger Neu-Initialisierung des KV-Cache und Kontext-Neuaufbau

**Warum der aktuelle Stand nicht optimal ist:**

- **Performance-Overhead**: Jeder Session Reset kostet 20ms und löscht wertvolle KV-Cache-Einträge
- **Kontext-Verlust**: Nützliche Informationen aus vorherigen Interaktionen werden unnötig verworfen
- **Inkonsistenz**: Die `prompt_shrunk`-Bedingung triggert bei jedem neuen Command, da der neue Prompt kürzer ist als der vorherige
- **Skalierbarkeit**: Bei längeren Konversationen wird das Problem durch wiederholte Resets noch deutlicher

**Wie es verbessert werden soll:**

1. **Rolling Window Context**: Behalte 80% des Kontexts, entferne nur die ältesten 20%
2. **Intelligente Reset-Erkennung**: Reset nur bei echten Kontext-Änderungen, nicht bei neuen Commands
3. **Native KV-Cache Management**: Nutze llama-cpp-4 Methoden zur Cache-Defragmentierung
4. **Kontext-Shifting**: Verschiebe Sequenz-IDs statt kompletter Neuberechnung

**Current Issue:**

```rust
// react.rs:87 - Always resets session
worker.reset().await.map_err( | e| AssistantError::LlmInference(e.to_string())) ?;
```

**KV-Cache-Konsistenz Problem:**
Der ursprüngliche Algorithmus kürzt einfach das `LlamaChatMessage`-Array auf Rust-Ebene und sendet das verkleinerte Array neu an die Engine. Dies erzwingt oft
eine vollständige Neuberechnung des gesamten restlichen Kontexts, weil die Historie nicht mehr exakt mit dem gespeicherten Zustand im KV-Cache übereinstimmt.

**Optimized Approach mit Native KV-Cache Management:**

```rust
use llama_cpp_4::context::LlamaKvCache;

impl VoiceAssistantService {
    async fn smart_session_reset(&self, worker: &LlmWorker, user_text: &str) -> Result<(), AssistantError> {
        // Check if context shifting is needed instead of full reset
        let needs_shift = self.should_shift_context(user_text).await?;

        if needs_shift {
            debug!("Smart context shift triggered - using native KV cache methods");
            self.native_context_shift(worker).await?;
        } else {
            debug!("No context management needed - reusing existing session");
        }

        Ok(())
    }

    async fn should_shift_context(&self, user_text: &str) -> Result<bool, AssistantError> {
        // Only shift if context is approaching limits
        let current_context_size = self.get_current_context_size().await?;
        let max_context = self.config.max_context_tokens;

        // Shift if we're at 85% of context limit (conservative)
        if current_context_size > (max_context * 85 / 100) {
            return Ok(true);
        }

        // Don't shift for new commands - reuse existing context
        Ok(false)
    }

    async fn native_context_shift(&self, worker: &LlmWorker) -> Result<(), AssistantError> {
        let session = worker.get_session_mut()
            .ok_or_else(|| AssistantError::LlmInference("No active session".to_string()))?;

        let current_n_cur = session.n_cur() as usize;
        let max_tokens = self.config.max_context_tokens;
        let preserve_ratio = 0.8; // Keep 80% of context

        // Calculate target position (remove oldest 20%)
        let target_n_cur = (current_n_cur as f64 * preserve_ratio) as i32;
        let tokens_to_remove = current_n_cur - target_n_cur as usize;

        debug!("Native context shift: removing {} tokens ({} → {})", 
               tokens_to_remove, current_n_cur, target_n_cur);

        // Use llama-cpp-4 native KV cache defragmentation
        if tokens_to_remove > 0 {
            // Method 1: Remove oldest sequence tokens
            let kv_cache = session.kv_cache_mut();

            // Remove tokens from the beginning of the sequence
            // This shifts the remaining tokens forward in the cache
            let result = kv_cache.seq_rm(0, tokens_to_remove as i32);

            if let Err(e) = result {
                warn!("Failed to remove sequence tokens: {}, falling back to manual shift", e);
                self.manual_context_shift(session, tokens_to_remove).await?;
            } else {
                // Update the session's token position
                session.set_n_cur(target_n_cur);
                debug!("Successfully removed {} tokens from KV cache", tokens_to_remove);
            }
        }

        Ok(())
    }

    async fn manual_context_shift(&self, session: &mut LlmSession, tokens_to_remove: usize) -> Result<(), AssistantError> {
        // Fallback method if native sequence removal fails
        debug!("Using manual context shift as fallback");

        // Shift tokens using llama.cpp's native context shifting
        let shift_result = session.shift_context(tokens_to_remove as i32);

        match shift_result {
            Ok(()) => {
                debug!("Manual context shift successful");
                Ok(())
            }
            Err(e) => {
                error!("Manual context shift failed: {}, falling back to full reset", e);
                // Last resort: full session reset
                session.reset();
                Ok(())
            }
        }
    }

    async fn rolling_window_context_messages(&self, conversation: &[LlamaChatMessage]) -> Vec<LlamaChatMessage> {
        // This method is only used for display/logging purposes
        // The actual context management happens in native_context_shift()
        let max_tokens = self.config.max_context_tokens;
        let preserve_ratio = 0.8; // Keep 80% of context

        // Calculate current token usage
        let current_tokens = self.estimate_tokens(conversation);

        if current_tokens <= max_tokens {
            return conversation.to_vec();
        }

        // Calculate how much to remove
        let target_tokens = (max_tokens as f64 * preserve_ratio) as usize;
        let mut accumulated_tokens = 0;
        let mut messages_to_keep = Vec::new();

        // Process messages in reverse order (keep newest)
        for message in conversation.iter().rev() {
            let message_tokens = self.estimate_message_tokens(message);
            if accumulated_tokens + message_tokens <= target_tokens {
                messages_to_keep.push(message.clone());
                accumulated_tokens += message_tokens;
            } else {
                break;
            }
        }

        // Reverse back to original order
        messages_to_keep.reverse();
        messages_to_keep
    }
}
```

**Native KV-Cache Methoden:**

```rust
// llama-cpp-4 provides these native methods for KV cache management:

impl LlmSession {
    /// Remove tokens from the beginning of the sequence
    /// This shifts remaining tokens forward in the cache
    pub fn shift_context(&mut self, shift_tokens: i32) -> Result<(), LlmError>;

    /// Get mutable reference to KV cache for advanced operations
    pub fn kv_cache_mut(&mut self) -> &mut LlamaKvCache;

    /// Get current token position
    pub fn n_cur(&self) -> i32;

    /// Set current token position
    pub fn set_n_cur(&mut self, n_cur: i32);
}

impl LlamaKvCache {
    /// Remove sequence tokens with precise control
    pub fn seq_rm(&mut self, seq_id: i32, n_tokens: i32) -> Result<(), LlmError>;

    /// Clear cache entries for specific sequence
    pub fn seq_clear(&mut self, seq_id: i32) -> Result<(), LlmError>;

    /// Defragment cache to remove gaps
    pub fn defragment(&mut self) -> Result<(), LlmError>;
}
```

**Vorteile des Native KV-Cache Managements:**

- **Cache-Konsistenz**: KV-Cache bleibt synchron mit der tatsächlichen Token-Historie
- **GPU-Effizienz**: Gelöschte Tokens werden direkt im GPU-Speicher freigegeben
- **Performance**: Verbleibende Tokens müssen nicht neu evaluiert werden
- **Memory Management**: Bessere Speichernutzung durch Defragmentierung

**Expected Performance Improvements:**

- **Session Reset Overhead**: 20ms → 0-5ms (4-20x improvement)
- **Context Preservation**: 80% of valuable context retained
- **KV-Cache Efficiency**: 90%+ of cache entries reused
- **Memory Consistency**: Stable memory usage patterns

---

## 3. Tool Invocation Optimization

### 3.1 Batch Tool Execution

**Aktueller Stand:**

- Der Voice Assistant Service führt Tools sequenziell aus, eins nach dem anderen
- Jede Tool-Invocation benötigt eigenen Round-Trip zum MCP-Server
- Fehlerbehandlung erfolgt individuell pro Tool ohne strukturierte Aggregation
- Es gibt kein Caching von Tool-Ergebnissen, wiederholte Aufrufe werden neu berechnet

**Warum der aktuelle Stand nicht optimal ist:**

- **Latenz-Akkumulation**: Bei 3-4 Tools addieren sich die einzelnen Latenzen (2-3 Sekunden gesamt)
- **Redundante Berechnungen**: Gleiche Tools mit gleichen Parametern werden mehrfach ausgeführt
- **Fehlerbehandlung**: Unstrukturierte Fehler machen Debugging und Recovery schwierig
- **Ressourcen-Verschwendung**: MCP-Server wird unnötig oft kontaktiert für wiederholte Anfragen

**Wie es verbessert werden soll:**

1. **Batch Execution**: Führe mehrere Tools parallel aus statt sequenziell
2. **Moka Caching**: Implementiere intelligentes Caching mit TTL für Tool-Ergebnisse
3. **Strukturierte Fehler**: Aggregiere Fehler von mehreren Tools in einer strukturierten Antwort
4. **Deduplizierung**: Erkenne und vermeide doppelte Tool-Aufrufe

**Implementation:**

```rust
use moka::future::Cache;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    pub tool_name: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_name: String,
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone)]
pub struct OptimizedToolExecutor {
    tool_cache: Cache<String, ToolResult>, // Use string keys for deterministic serialization
    max_concurrent: usize,
}

impl ToolRequest {
    /// Generate deterministic cache key to avoid JSON key ordering issues
    pub fn cache_key(&self) -> String {
        let params_str = self.deterministic_json_string(&self.parameters);
        format!("{}:{}", self.tool_name, params_str)
    }

    /// Convert JSON to deterministic string with sorted keys
    fn deterministic_json_string(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Object(map) => {
                let mut sorted_map = BTreeMap::new();
                for (key, val) in map {
                    sorted_map.insert(key.clone(), self.deterministic_json_string(val));
                }
                serde_json::to_string(&sorted_map).unwrap_or_default()
            }
            serde_json::Value::Array(arr) => {
                let sorted_arr: Vec<String> = arr.iter()
                    .map(|v| self.deterministic_json_string(v))
                    .collect();
                serde_json::to_string(&sorted_arr).unwrap_or_default()
            }
            _ => serde_json::to_string(value).unwrap_or_default(),
        }
    }
}

impl OptimizedToolExecutor {
    pub fn new() -> Self {
        Self {
            tool_cache: Cache::builder()
                .time_to_live(Duration::from_secs(300)) // 5 minutes TTL
                .max_capacity(1000)
                .build(),
            max_concurrent: 4,
        }
    }

    pub async fn execute_batch(&self, requests: Vec<ToolRequest>) -> Vec<ToolResult> {
        // Deduplicate requests using deterministic cache keys
        let mut unique_requests = std::collections::HashMap::new();
        for request in requests {
            let cache_key = request.cache_key();
            if !unique_requests.contains_key(&cache_key) {
                unique_requests.insert(cache_key, request);
            }
        }

        // Check cache first
        let mut results = Vec::new();
        let mut uncached_requests = Vec::new();

        for (cache_key, request) in &unique_requests {
            if let Some(cached_result) = self.tool_cache.get(cache_key) {
                results.push(cached_result);
            } else {
                uncached_requests.push((cache_key.clone(), request.clone()));
            }
        }

        // Execute uncached requests in parallel
        if !uncached_requests.is_empty() {
            let batch_results = self.execute_parallel_batch(uncached_requests).await;

            // Cache results with deterministic keys
            for ((cache_key, _request), result) in uncached_requests.iter().zip(batch_results.iter()) {
                self.tool_cache.insert(cache_key.clone(), result.clone());
            }

            results.extend(batch_results);
        }

        results
    }

    async fn execute_parallel_batch(&self, requests: Vec<(String, ToolRequest)>) -> Vec<ToolResult> {
        use futures::stream::{self, StreamExt};

        stream::iter(requests)
            .map(|(_cache_key, request)| self.execute_single_tool(request))
            .buffer_unordered(self.max_concurrent)
            .collect()
            .await
    }

    async fn execute_single_tool(&self, request: ToolRequest) -> ToolResult {
        let start_time = std::time::Instant::now();

        match self.invoke_tool_mcp(&request.tool_name, &request.parameters).await {
            Ok(result) => ToolResult {
                tool_name: request.tool_name,
                success: true,
                result: Some(result),
                error: None,
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            },
            Err(e) => ToolResult {
                tool_name: request.tool_name,
                success: false,
                result: None,
                error: Some(e.to_string()),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            },
        }
    }
}
```

**JSON Key Ordering Problem:**
Der ursprüngliche Ansatz verwendet `format!("{}:{}", tool, args.to_string())` für Cache-Keys, aber `serde_json::Value` garantiert keine feste Sortierung:

```rust
// PROBLEMATISCH: Unterschiedliche Keys für semantisch identische Objekte
let cache_key = format!("{}:{}", tool, args.to_string());
// { "a": 1, "b": 2 } → "tool:{\"a\":1,\"b\":2}"
// { "b": 2, "a": 1 } → "tool:{\"b\":2,\"a\":1}"  // Cache Miss!
```

**Deterministic Serialization Solution:**

```rust
impl ToolRequest {
    /// Generate deterministic cache key to avoid JSON key ordering issues
    pub fn cache_key(&self) -> String {
        let params_str = self.deterministic_json_string(&self.parameters);
        format!("{}:{}", self.tool_name, params_str)
    }

    /// Convert JSON to deterministic string with sorted keys
    fn deterministic_json_string(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Object(map) => {
                let mut sorted_map = BTreeMap::new();
                for (key, val) in map {
                    sorted_map.insert(key.clone(), self.deterministic_json_string(val));
                }
                serde_json::to_string(&sorted_map).unwrap_or_default()
            }
            serde_json::Value::Array(arr) => {
                let sorted_arr: Vec<String> = arr.iter()
                    .map(|v| self.deterministic_json_string(v))
                    .collect();
                serde_json::to_string(&sorted_arr).unwrap_or_default()
            }
            _ => serde_json::to_string(value).unwrap_or_default(),
        }
    }
}
```

**Expected Performance Improvements:**

- **Tool Invocation Time**: 2-3 seconds → < 1 second (2-3x improvement)
- **Cache Hit Rate**: 70-90% for repeated operations
- **Cache Accuracy**: 100% deterministic keys (no false misses)
- **Parallel Execution**: 3-4x faster for multiple independent tools
- **Error Handling**: Structured error reporting with execution metrics

---

## 4. FastEmbed GPU Acceleration

### 4.1 Embedding Generation Performance

**Aktueller Stand:**

- Der Voice Assistant Service verwendet FastEmbed für Embedding-Generation mit ONNX Runtime
- Die aktuelle Implementierung läuft ausschließlich auf CPU, obwohl GPU-Unterstützung verfügbar wäre
- Embeddings werden einzeln generiert ohne Batch-Processing oder Caching
- Es findet keine GPU-Beschleunigung statt, obwohl die Hardware (Ryzen 5 8500G iGPU, Ryzen 9 9950X3D + RX 7900 XTX) CUDA/OpenVINO-Unterstützung bietet

**Warum der aktuelle Stand nicht optimal ist:**

- **Performance-Bottleneck**: Embedding-Generation dauert 100-500ms pro Embedding, was bei mehreren Texten akkumuliert
- **CPU-Overlast**: Die CPU wird während der Embedding-Generation ausgelastet, was andere Prozesse verlangsamt
- **Kein Caching**: Gleiche Texte werden immer wieder neu embeddet, was redundant ist
- **Skalierbarkeit**: Bei größeren Dokumenten oder Batch-Operationen wird das Problem noch deutlicher

**Wie es verbessert werden soll:**

1. **ONNX Runtime GPU**: Aktiviere GPU-Beschleunigung durch ONNX Runtime (CUDA/OpenVINO/DirectML)
2. **Batch Processing**: Verarbeite mehrere Texte gleichzeitig für bessere GPU-Auslastung
3. **Embedding Caching**: Implementiere intelligentes Caching mit TTL für Embedding-Ergebnisse
4. **Auto-Detection**: Automatische Erkennung und Fallback auf CPU bei GPU-Problemen

**Implementation:**

```rust
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use moka::future::Cache;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub model_name: String,
    pub batch_size: usize,
    pub cache_embeddings: bool,
    pub cache_ttl_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct OptimizedSemanticMemory {
    model: TextEmbedding,
    vectors: Vec<(Vec<f32>, String)>,
    db: Arc<Mutex<Connection>>,
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    embedding_cache: Cache<String, Vec<f32>>,
}

impl OptimizedSemanticMemory {
    pub fn new(config: EmbeddingConfig) -> Result<Self, MemoryError> {
        let model = config.create_model()?;
        let embedding_cache = Cache::builder()
            .time_to_live(Duration::from_secs(config.cache_ttl_seconds))
            .max_capacity(1000)
            .build();

        Ok(Self {
            model,
            vectors: Vec::new(),
            db: Arc::new(Mutex::new(Connection::open("memory.db")?)),
            cache: Arc::new(RwLock::new(HashMap::new())),
            embedding_cache,
        })
    }

    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, MemoryError> {
        // Check cache first
        let mut uncached_texts = Vec::new();
        let mut cached_results = Vec::new();

        for text in &texts {
            if let Some(cached) = self.embedding_cache.get(text) {
                cached_results.push(cached.clone());
            } else {
                uncached_texts.push(text.clone());
            }
        }

        // Generate embeddings for uncached texts in batches
        if !uncached_texts.is_empty() {
            let embeddings = self.model.embed(uncached_texts.clone(), None)
                .map_err(|e| MemoryError::EmbeddingFailed(e.to_string()))?;

            // Cache results
            for (text, embedding) in uncached_texts.iter().zip(embeddings.iter()) {
                self.embedding_cache.insert(text.clone(), embedding.clone());
            }

            cached_results.extend(embeddings);
        }

        Ok(cached_results)
    }
}

impl EmbeddingConfig {
    pub fn create_model(&self) -> Result<TextEmbedding, EmbeddingError> {
        let model = match self.model_name.as_str() {
            "bge-small-en-v1.5-q" => EmbeddingModel::BGESmallENV15Q,
            "bge-small-en-v1.5" => EmbeddingModel::BGESmallENV15,
            "all-MiniLM-L6-v2" => EmbeddingModel::AllMiniLML6V2,
            other => {
                debug!("Unknown embedding model '{}', falling back to BGESmallENV15Q", other);
                EmbeddingModel::BGESmallENV15Q
            }
        };

        let mut options = InitOptions::new(model)
            .with_batch_size(self.batch_size)
            .with_show_download_progress(false);

        // FastEmbed automatically uses GPU if available via ONNX Runtime
        // No explicit GPU flag needed - ort handles device detection

        let embedding_model = TextEmbedding::try_new(options)?;
        Ok(embedding_model)
    }
}
```

**Expected Performance Improvements:**

- **Embedding Generation**: 100-500ms → 10-50ms (10-50x improvement)
- **Batch Processing**: 5-10x improvement with parallelism
- **Cache Hit Rate**: 80-95% for repeated texts
- **Memory Usage**: Additional 200-500MB VRAM for model

**ONNX Runtime GPU Support:**

- **CUDA**: NVIDIA GPUs (if available)
- **OpenVINO**: Intel GPUs
- **DirectML**: Windows GPUs
- **TensorRT**: NVIDIA optimization
- **Auto-detection**: Falls back to CPU if GPU unavailable

---

## 5. Nucleo-Matcher Optimization

### 5.1 Tool Selection Performance

**Aktueller Stand:**

- Der Voice Assistant Service verwendet nucleo-matcher für fuzzy tool selection
- Die aktuelle Implementierung durchsucht linear alle Tools pro Query ohne Optimierungen
- Es findet kein Caching von Query-Ergebnissen oder Pre-filtering statt
- ASCII-Optimierungen werden nicht genutzt, obwohl 90% der Anfragen ASCII-basiert sind

**Warum der aktuelle Stand nicht optimal ist:**

- **Performance-Bottleneck**: Tool-Selection dauert 2-5ms für 50-100 Tools, was sich bei häufigen Anfragen akkumuliert
- **Redundante Berechnungen**: Gleiche Queries werden immer wieder neu berechnet ohne Caching
- **Ineffiziente Filterung**: Alle Tools werden durchsucht statt vorab zu filtern
- **Skalierbarkeit**: Bei wachsender Tool-Anzahl degradiert die Performance linear

**Wie es verbessert werden soll:**

1. **Query Result Caching**: Implementiere LRU-Cache für wiederholte Queries
2. **Keyword Pre-filtering**: Kategorisiere Tools nach Keywords für schnellere Candidate-Reduction
3. **Conservative ASCII Optimization**: Vermeide aggressive Pre-Filtering, nutze nucleo-matcher direkt
4. **Batch Processing**: Verarbeite mehrere Queries gleichzeitig wenn möglich

**ASCII Pre-filtering Problem:**
Der ursprüngliche Algorithmus verwendet ein aggressives Byte-Vergleich, das zu False Negatives führt:

```rust
// PROBLEMATISCH: Wirft Tool weg wenn ein Zeichen fehlt
if ! query_bytes.iter().any( | & c| {
ascii_tool.match_text_bytes.contains( & c)
}) {
return None;  // Falsch-negative!
}
```

**Beispiel**: Query "Zeige mir das Wetter" enthält 'm' aus "mir", aber Tool "get_weather" hat kein 'm' → Tool wird verworfen!

**Optimized Implementation:**

```rust
use nucleo_matcher::Config;
use nucleo_matcher::Matcher;
use nucleo_matcher::pattern::CaseMatching;
use nucleo_matcher::pattern::Normalization;
use nucleo_matcher::pattern::Pattern;
use moka::future::Cache;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct OptimizedToolRouter {
    config: Config,
    tools: Vec<ToolEntry>,
    /// Pre-filtered tool categories for faster lookup
    categorized_tools: HashMap<String, Vec<usize>>,
    /// Cache of recent queries and results
    query_cache: Cache<String, Vec<usize>>,
}

impl OptimizedToolRouter {
    pub fn new() -> Self {
        Self {
            config: Config::DEFAULT,
            tools: Vec::new(),
            categorized_tools: HashMap::new(),
            query_cache: Cache::builder()
                .max_capacity(100)
                .build(),
        }
    }

    /// Rebuilds with optimizations for faster matching
    pub fn rebuild(&mut self, catalog: &[ToolCatalogEntry]) {
        self.tools = catalog
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let match_text = format!("{} {}", t.name, t.description);
                ToolEntry {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    input_schema: t.input_schema.clone(),
                    match_text,
                    index: i,
                }
            })
            .collect();

        // Categorize tools by keywords for pre-filtering
        self.categorized_tools.clear();
        for (i, tool) in self.tools.iter().enumerate() {
            let keywords = extract_keywords(&tool.match_text);
            for keyword in keywords {
                self.categorized_tools
                    .entry(keyword.to_lowercase())
                    .or_default()
                    .push(i);
            }
        }

        debug!("Tool router: rebuilt with {} tools", self.tools.len());
    }

    /// Optimized tool selection with caching and conservative pre-filtering
    pub fn select_tools(&self, query: &str, top_n: usize) -> Vec<ToolCatalogEntry> {
        // Check cache first
        if let Some(cached_indices) = self.query_cache.get(query) {
            return self.indices_to_entries(&cached_indices, top_n);
        }

        // Conservative pre-filtering by keywords only
        let candidate_indices = self.pre_filter_candidates(query);

        // Use nucleo-matcher directly (highly SIMD-optimized)
        let selected_indices = self.select_tools_optimized(query, &candidate_indices, top_n);

        // Cache result
        self.query_cache.insert(query.to_string(), selected_indices.clone());

        self.indices_to_entries(&selected_indices, top_n)
    }

    /// Optimized selection using nucleo-matcher directly
    fn select_tools_optimized(&self, query: &str, candidates: &[usize], top_n: usize) -> Vec<usize> {
        let mut matcher = Matcher::new(self.config.clone());
        let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);

        let mut scored: Vec<(u32, usize)> = candidates
            .iter()
            .filter_map(|&index| {
                if let Some(tool) = self.tools.get(index) {
                    let mut buf: Vec<char> = Vec::new();
                    let haystack = Utf32Str::new(&tool.match_text, &mut buf);
                    pattern.score(haystack, &mut matcher)
                        .map(|score| (score, index))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().take(top_n).map(|(_, index)| index).collect()
    }

    /// Conservative pre-filtering by keywords only
    fn pre_filter_candidates(&self, query: &str) -> Vec<usize> {
        let query_keywords: HashSet<&str> = extract_keywords(query).iter().map(|s| s.as_str()).collect();

        let mut candidate_set: HashSet<usize> = HashSet::new();

        for keyword in &query_keywords {
            if let Some(indices) = self.categorized_tools.get(&keyword.to_lowercase()) {
                candidate_set.extend(indices);
            }
        }

        // If no keyword matches, use all tools (avoid false negatives)
        if candidate_set.is_empty() {
            (0..self.tools.len()).collect()
        } else {
            candidate_set.into_iter().collect()
        }
    }
}

/// Extract keywords from text for categorization
fn extract_keywords(text: &str) -> Vec<String> {
    text.split_whitespace()
        .flat_map(|word| {
            // Extract meaningful keywords (length > 2)
            if word.len() > 2 {
                Some(word.to_lowercase())
            } else {
                None
            }
        })
        .collect()
}
```

**Expected Performance Improvements:**

- **Tool Selection**: 2-5ms → 1-3ms (1.2-2x improvement)
- **Query Caching**: 90%+ faster for repeated queries
- **Keyword Pre-filtering**: 30-50% faster candidate reduction
- **Reliability**: 100% accuracy (no false negatives)

**Memory Impact:**

- **Additional cache**: ~10-50KB for 100 cached queries
- **Keyword index**: ~5-10KB for tool categorization
- **No ASCII overhead**: Eliminated problematic byte-filtering

**Why Conservative Approach is Better:**

- **Nucleo-matcher is already highly optimized**: SIMD-accelerated, sub-millisecond for hundreds of entries
- **Avoids false negatives**: No aggressive character-based filtering
- **Maintains accuracy**: Fuzzy matching works correctly for all queries
- **Simpler implementation**: Less code, fewer edge cases

#[derive (Debug, Clone, Builder)]
pub struct ContextConfig { /// Maximum tokens before context shifting #[builder (default = "4096")]
pub max_context_tokens: usize,

    /// Ratio of tokens to keep when shifting (0.0-1.0)
    #[builder(default = "0.8")]
    pub context_keep_ratio: f64,
    
    /// Minimum tokens to preserve (system prompt)
    #[builder(default = "512")]
    pub min_preserve_tokens: usize,

}

```

### 2.3 Selective Cache Clearing

```rust
impl LlmWorker {
    /// Clear conversation history while preserving KV cache
    pub async fn clear_conversation(&mut self) -> Result<(), LlmError> {
        if let Some(session) = &mut self.session {
            // Clear conversation tokens but keep model weights in memory
            session.clear_conversation_history()?;
            self.last_system_prompt = None;
        }
        Ok(())
    }
    
    /// Clear only recent context while preserving older conversation
    pub async fn trim_context(&mut self, keep_last_n: usize) -> Result<(), LlmError> {
        if let Some(session) = &mut self.session {
            session.trim_context(keep_last_n)?;
        }
        Ok(())
    }
}
```

---

## 3. Tool Invocation Optimization

### 3.1 Tool Result Caching with Moka

```rust
use moka::future::Cache;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ToolCache {
    cache: Cache<String, CachedToolResult>,
}

#[derive(Debug, Clone)]
pub struct CachedToolResult {
    pub result: String,
    pub timestamp: Instant,
    pub tool: String,
    pub args: serde_json::Value,
}

impl ToolCache {
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .time_to_live(Duration::from_secs(300)) // 5 minutes TTL
                .time_to_idle(Duration::from_secs(60))  // 1 minute idle
                .max_capacity(1000)                     // Max 1000 entries
                .build(),
        }
    }

    pub async fn get_or_execute<F, Fut>(&self, tool: &str, args: &serde_json::Value, executor: F) -> Result<String, AssistantError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output=Result<String, AssistantError>>,
    {
        let cache_key = format!("{}:{}", tool, args.to_string());

        // Check cache (async, thread-safe)
        if let Some(cached) = self.cache.get(&cache_key).await {
            debug!("Tool cache hit for {}", tool);
            return Ok(cached.result.clone());
        }

        // Execute and cache
        debug!("Tool cache miss for {}", tool);
        let result = executor().await?;

        let cached_result = CachedToolResult {
            result: result.clone(),
            timestamp: Instant::now(),
            tool: tool.to_string(),
            args: args.clone(),
        };

        // Insert into cache (async, automatic eviction)
        self.cache.insert(cache_key, cached_result).await;

        Ok(result)
    }

    pub async fn invalidate_tool(&self, tool_name: &str) {
        // Invalidate all cache entries for a specific tool
        let keys_to_remove: Vec<String> = self.cache
            .keys()
            .await
            .into_iter()
            .filter(|key| key.starts_with(&format!("{}:", tool_name)))
            .collect();

        for key in keys_to_remove {
            self.cache.invalidate(&key).await;
        }
    }
}
```

### 3.2 Batch Tool Execution

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ToolResult {
    pub tool_name: String,
    pub success: bool,
    pub result: Option<String>,
    pub error: Option<ToolError>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl VoiceAssistantService {
    async fn execute_tools_batch(&self, tools: Vec<ToolCall>) -> Result<Vec<ToolResult>, AssistantError> {
        let futures: Vec<_> = tools.into_iter()
            .map(|tool| self.execute_tool_with_structured_error(tool))
            .collect();

        let results = futures::future::join_all(futures).await;

        let mut tool_results = Vec::new();
        for result in results {
            match result {
                Ok(r) => tool_results.push(r),
                Err(e) => tool_results.push(ToolResult {
                    tool_name: "unknown".to_string(),
                    success: false,
                    result: None,
                    error: Some(ToolError {
                        code: "EXECUTION_ERROR".to_string(),
                        message: e.to_string(),
                        retryable: false,
                    }),
                }),
            }
        }

        Ok(tool_results)
    }

    async fn execute_tool_with_structured_error(&self, tool: ToolCall) -> Result<ToolResult, AssistantError> {
        match self.invoke_tool(&tool.name, &tool.arguments).await {
            Ok(result) => Ok(ToolResult {
                tool_name: tool.name,
                success: true,
                result: Some(result),
                error: None,
            }),
            Err(e) => Ok(ToolResult {
                tool_name: tool.name,
                success: false,
                result: None,
                error: Some(ToolError {
                    code: self.classify_error(&e),
                    message: e.to_string(),
                    retryable: self.is_retryable(&e),
                }),
            }),
        }
    }

    fn classify_error(&self, error: &AssistantError) -> String {
        match error {
            AssistantError::ToolNotFound(_) => "TOOL_NOT_FOUND".to_string(),
            AssistantError::ToolExecution(_) => "EXECUTION_ERROR".to_string(),
            AssistantError::InvalidArguments(_) => "INVALID_ARGUMENTS".to_string(),
            _ => "UNKNOWN_ERROR".to_string(),
        }
    }

    fn is_retryable(&self, error: &AssistantError) -> bool {
        !matches!(error, AssistantError::ToolNotFound(_) | AssistantError::InvalidArguments(_))
    }
}
```

---

## 4. FastEmbed GPU Acceleration

### 4.1 Embedding Generation Performance

**Current Issue:**

- FastEmbed uses ONNX Runtime via `ort` crate
- CPU-only embedding generation for semantic memory
- **Opportunity**: GPU acceleration reduces embedding time significantly

**Implementation with FastEmbed:**

```rust
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};

#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub model_name: String,
    pub use_gpu: bool,
    pub batch_size: usize,
    pub cache_embeddings: bool,
}

impl EmbeddingConfig {
    pub fn create_model(&self) -> Result<TextEmbedding, EmbeddingError> {
        let model = match self.model_name.as_str() {
            "bge-small-en-v1.5-q" => EmbeddingModel::BGESmallENV15Q,
            "bge-small-en-v1.5" => EmbeddingModel::BGESmallENV15,
            "all-MiniLM-L6-v2" => EmbeddingModel::AllMiniLML6V2,
            other => {
                debug!("Unknown embedding model '{}', falling back to BGESmallENV15Q", other);
                EmbeddingModel::BGESmallENV15Q
            }
        };

        let mut options = InitOptions::new(model)
            .with_batch_size(self.batch_size)
            .with_show_download_progress(false);

        // FastEmbed automatically uses GPU if available via ONNX Runtime
        // No explicit GPU flag needed - ort handles device detection

        let embedding_model = TextEmbedding::try_new(options)?;
        Ok(embedding_model)
    }
}

pub struct OptimizedSemanticMemory {
    model: TextEmbedding,
    vectors: Vec<(Vec<f32>, String)>,
    db: Arc<Mutex<Connection>>,
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
}

impl OptimizedSemanticMemory {
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, MemoryError> {
        // Check cache first
        let mut uncached_texts = Vec::new();
        let mut cached_results = Vec::new();

        for text in &texts {
            if let Some(cached) = self.cache.read().unwrap().get(text) {
                cached_results.push(cached.clone());
            } else {
                uncached_texts.push(text.clone());
            }
        }

        // Generate embeddings for uncached texts
        if !uncached_texts.is_empty() {
            let embeddings = self.model.embed(uncached_texts.clone(), None)
                .map_err(|e| MemoryError::EmbeddingFailed(e.to_string()))?;

            // Cache results
            {
                let mut cache = self.cache.write().unwrap();
                for (text, embedding) in uncached_texts.iter().zip(embeddings.iter()) {
                    cache.insert(text.clone(), embedding.clone());
                }
            }

            cached_results.extend(embeddings);
        }

        Ok(cached_results)
    }
}
```

**Expected Performance:**

- **CPU-only**: 100-500ms per embedding (BGE-small model)
- **GPU (ONNX Runtime)**: 10-50ms per embedding
- **Batch Processing**: 5-10x improvement with parallelism
- **Memory**: Additional 200-500MB VRAM for model

**ONNX Runtime GPU Support:**

- **CUDA**: NVIDIA GPUs (if available)
- **OpenVINO**: Intel GPUs
- **DirectML**: Windows GPUs
- **TensorRT**: NVIDIA optimization
- **Auto-detection**: Falls back to CPU if GPU unavailable

---

## 5. Nucleo-Matcher Optimization

### 5.1 Tool Selection Performance

**Current Implementation:**

- Nucleo-matcher for fuzzy tool selection
- Linear search through all tools for each query
- **Opportunity**: Pre-filtering and caching optimizations

**Performance Analysis:**

- **Current**: 2-5ms for 50-100 tools (typical use case)
- **Benchmark**: Nucleo is 6x faster than skim, 2-10x faster than fzf
- **Scaling**: Performance degrades with tool count growth

**Optimization Strategies:**

```rust
use nucleo_matcher::Config;
use nucleo_matcher::Matcher;
use nucleo_matcher::pattern::CaseMatching;
use nucleo_matcher::pattern::Normalization;
use nucleo_matcher::pattern::Pattern;

#[derive(Debug, Clone)]
pub struct OptimizedToolRouter {
    config: Config,
    tools: Vec<ToolEntry>,
    /// Pre-filtered tool categories for faster lookup
    categorized_tools: HashMap<String, Vec<usize>>,
    /// Cache of recent queries and results
    query_cache: Arc<RwLock<LruCache<String, Vec<usize>>>>,
    /// Pre-computed ASCII optimizations
    ascii_tools: Vec<AsciiToolEntry>,
}

#[derive(Debug, Clone)]
struct AsciiToolEntry {
    name: String,
    description: String,
    /// ASCII-only version for fast matching
    match_text_bytes: Vec<u8>,
    /// Tool index in main list
    index: usize,
}

impl OptimizedToolRouter {
    pub fn new() -> Self {
        Self {
            config: Config::DEFAULT,
            tools: Vec::new(),
            categorized_tools: HashMap::new(),
            query_cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(100).unwrap()
            ))),
            ascii_tools: Vec::new(),
        }
    }

    /// Rebuilds with optimizations for faster matching
    pub fn rebuild(&mut self, catalog: &[ToolCatalogEntry]) {
        self.tools = catalog
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let match_text = format!("{} {}", t.name, t.description);
                ToolEntry {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    input_schema: t.input_schema.clone(),
                    match_text,
                    index: i,
                }
            })
            .collect();

        // Pre-filter ASCII tools for fast path
        self.ascii_tools = self.tools
            .iter()
            .filter_map(|tool| {
                if tool.match_text.is_ascii() {
                    Some(AsciiToolEntry {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        match_text_bytes: tool.match_text.as_bytes().to_vec(),
                        index: tool.index,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Categorize tools by keywords for pre-filtering
        self.categorized_tools.clear();
        for (i, tool) in self.tools.iter().enumerate() {
            let keywords = extract_keywords(&tool.match_text);
            for keyword in keywords {
                self.categorized_tools
                    .entry(keyword.to_lowercase())
                    .or_default()
                    .push(i);
            }
        }

        debug!("Tool router: rebuilt with {} tools ({} ASCII optimized)", 
               self.tools.len(), self.ascii_tools.len());
    }

    /// Optimized tool selection with caching and pre-filtering
    pub fn select_tools(&self, query: &str, top_n: usize) -> Vec<ToolCatalogEntry> {
        // Check cache first
        if let Ok(mut cache) = self.query_cache.write() {
            if let Some(cached_indices) = cache.get(query) {
                return self.indices_to_entries(cached_indices, top_n);
            }
        }

        // Pre-filter by keywords
        let candidate_indices = self.pre_filter_candidates(query);

        // Fast ASCII path for ASCII queries
        let selected_indices = if query.is_ascii() && !self.ascii_tools.is_empty() {
            self.select_tools_ascii(query, &candidate_indices, top_n)
        } else {
            self.select_tools_unicode(query, &candidate_indices, top_n)
        };

        // Cache result
        if let Ok(mut cache) = self.query_cache.write() {
            cache.put(query.to_string(), selected_indices.clone());
        }

        self.indices_to_entries(&selected_indices, top_n)
    }

    /// Fast ASCII-only matching using memchr optimizations
    fn select_tools_ascii(&self, query: &str, candidates: &[usize], top_n: usize) -> Vec<usize> {
        let query_bytes = query.as_bytes();
        let mut matcher = Matcher::new(self.config.clone());
        let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);

        let mut scored: Vec<(u32, usize)> = candidates
            .iter()
            .filter_map(|&index| {
                if let Some(ascii_tool) = self.ascii_tools.get(index) {
                    // Use memchr for fast pre-filtering
                    if !query_bytes.iter().any(|&c| {
                        ascii_tool.match_text_bytes.contains(&c)
                    }) {
                        return None;
                    }

                    let mut buf: Vec<char> = Vec::new();
                    let haystack = Utf32Str::new(&ascii_tool.match_text, &mut buf);
                    pattern.score(haystack, &mut matcher)
                        .map(|score| (score, index))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().take(top_n).map(|(_, index)| index).collect()
    }

    /// Unicode matching with pre-filtered candidates
    fn select_tools_unicode(&self, query: &str, candidates: &[usize], top_n: usize) -> Vec<usize> {
        let mut matcher = Matcher::new(self.config.clone());
        let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
        let mut buf: Vec<char> = Vec::new();

        let mut scored: Vec<(u32, usize)> = candidates
            .iter()
            .filter_map(|&index| {
                if let Some(tool) = self.tools.get(index) {
                    let mut buf: Vec<char> = Vec::new();
                    let haystack = Utf32Str::new(&tool.match_text, &mut buf);
                    pattern.score(haystack, &mut matcher)
                        .map(|score| (score, index))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().take(top_n).map(|(_, index)| index).collect()
    }

    /// Pre-filter candidates by keyword matching
    fn pre_filter_candidates(&self, query: &str) -> Vec<usize> {
        let query_keywords: HashSet<&str> = extract_keywords(query).iter().map(|s| s.as_str()).collect();

        let mut candidate_set: HashSet<usize> = HashSet::new();

        for keyword in &query_keywords {
            if let Some(indices) = self.categorized_tools.get(&keyword.to_lowercase()) {
                candidate_set.extend(indices);
            }
        }

        // If no keyword matches, use all tools
        if candidate_set.is_empty() {
            (0..self.tools.len()).collect()
        } else {
            candidate_set.into_iter().collect()
        }
    }

    fn indices_to_entries(&self, indices: &[usize], top_n: usize) -> Vec<ToolCatalogEntry> {
        indices
            .iter()
            .take(top_n)
            .filter_map(|&index| self.tools.get(index))
            .map(|tool| ToolCatalogEntry {
                name: tool.name.clone(),
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
            })
            .collect()
    }
}

/// Extract keywords from text for categorization
fn extract_keywords(text: &str) -> Vec<String> {
    text.split_whitespace()
        .flat_map(|word| {
            // Extract meaningful keywords (length > 2)
            if word.len() > 2 {
                Some(word.to_lowercase())
            } else {
                None
            }
        })
        .collect()
}
```

**Expected Performance Improvements:**

- **ASCII queries**: 2-3x faster (memchr pre-filtering)
- **Keyword pre-filtering**: 50-70% faster candidate reduction
- **Query caching**: 90%+ faster for repeated queries
- **Overall**: 1.5-2x improvement for typical workloads

**Memory Impact:**

- **Additional cache**: ~10-50KB for 100 cached queries
- **ASCII optimization**: ~20% memory overhead for duplicate data
- **Keyword index**: ~5-10KB for tool categorization

---

## 6. Whisper GPU Acceleration

### 6.1 Speech Recognition Performance

**Current Issue:**

- Whisper CPU-only initialization: `use gpu = 0`
- Transcription time: 5-10 seconds for short commands
- **Opportunity**: GPU acceleration reduces to < 100ms

**Implementation with whisper-rs:**

```rust
use whisper_rs::{WhisperContext, FullParams, SamplingStrategy};

#[derive(Debug, Clone)]
pub struct WhisperConfig {
    pub model_path: PathBuf,
    pub use_gpu: bool,
    pub gpu_device: Option<u32>,
    pub threads: u32,
}

impl WhisperConfig {
    pub fn create_context(&self) -> Result<WhisperContext, WhisperError> {
        let ctx = WhisperContext::new(&self.model_path)?;

        if self.use_gpu {
            // Enable GPU acceleration if available
            if let Some(device) = self.gpu_device {
                ctx.set_gpu_device(device)?;
            }
        }

        Ok(ctx)
    }
}

pub struct SpeechRecognizer {
    context: WhisperContext,
    config: WhisperConfig,
}

impl SpeechRecognizer {
    pub async fn transcribe(&mut self, audio_data: &[f32]) -> Result<String, WhisperError> {
        let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("de"));
        params.set_translate(false);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        let mut state = self.context.create_state()?;
        state.full(params, audio_data)?;

        let mut segments = Vec::new();
        for segment in state.iter_segments() {
            segments.push(segment.get_text()?.to_string());
        }

        Ok(segments.join(" ").trim().to_string())
    }
}
```

**Expected Performance:**

- **CPU-only**: 5-10 seconds
- **GPU (Vulkan/OpenCL)**: 50-200ms
- **Memory**: Additional 100-200MB VRAM

---

## 7. Configuration Extensions

### 7.1 Performance Configuration

```rust
#[derive(Debug, Clone, Builder)]
pub struct PerformanceConfig {
    /// GPU backend selection
    #[builder(default = "LlmBackend::Auto")]
    pub gpu_backend: LlmBackend,

    /// Session management strategy
    #[builder(default = "SessionStrategy::Smart")]
    pub session_strategy: SessionStrategy,

    /// GPU layer offloading configuration
    #[builder(default)]
    pub gpu_config: GpuConfig,

    /// Context window management
    #[builder(default)]
    pub context_config: ContextConfig,

    /// Tool cache configuration
    #[builder(default)]
    pub cache_config: CacheConfig,

    /// Whisper speech recognition
    #[builder(default)]
    pub whisper_config: WhisperConfig,

    /// FastEmbed embedding generation
    #[builder(default)]
    pub embedding_config: EmbeddingConfig,

    /// Nucleo-matcher optimization
    #[builder(default)]
    pub tool_router_config: ToolRouterConfig,
}

#[derive(Debug, Clone, Builder)]
pub struct EmbeddingConfig {
    /// Embedding model name
    #[builder(default = "\"bge-small-en-v1.5-q\".to_string()")]
    pub model_name: String,

    /// Batch size for embedding generation
    #[builder(default = "256")]
    pub batch_size: usize,

    /// Enable embedding caching
    #[builder(default = true)]
    pub cache_embeddings: bool,

    /// Cache TTL for embeddings in seconds
    #[builder(default = "3600")]
    pub cache_ttl_seconds: u64,
}

#[derive(Debug, Clone, Builder)]
pub struct CacheConfig {
    /// Tool cache TTL in seconds
    #[builder(default = "300")]
    pub ttl_seconds: u64,

    /// Maximum cache entries
    #[builder(default = "1000")]
    pub max_entries: u64,

    /// Idle time before eviction
    #[builder(default = "60")]
    pub idle_seconds: u64,
}

#[derive(Debug, Clone)]
pub enum SessionStrategy {
    AlwaysReset,    // Current behavior
    Smart,          // Optimized behavior
    NeverReset,     // Maximum performance, potential memory issues
}

#[derive(Debug, Clone)]
pub enum LlmBackend {
    Auto,
    Vulkan,
    Cpu,
}
```

### 6.2 Default Configuration

```rust
impl Default for PerformanceConfig {
    fn default() -> Self {
        PerformanceConfig {
            gpu_backend: LlmBackend::Auto,
            session_strategy: SessionStrategy::Smart,
            gpu_config: GpuConfig::default(),
            context_config: ContextConfig::builder()
                .max_context_tokens(4096)
                .context_keep_ratio(0.8)
                .min_preserve_tokens(512)
                .build()
                .unwrap(),
            cache_config: CacheConfig::default(),
            whisper_config: WhisperConfig::builder()
                .use_gpu(true)
                .threads(4)
                .build()
                .unwrap(),
            embedding_config: EmbeddingConfig::default(),
        }
    }
}

impl Default for GpuConfig {
    fn default() -> Self {
        GpuConfig {
            backend: LlmBackend::Auto,
            device_type: DeviceType::Cpu,
            vram_budget_mb: 0,
            n_gpu_layers: 0,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        CacheConfig {
            ttl_seconds: 300,
            max_entries: 1000,
            idle_seconds: 60,
        }
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        EmbeddingConfig {
            model_name: "bge-small-en-v1.5-q".to_string(),
            batch_size: 256,
            cache_embeddings: true,
            cache_ttl_seconds: 3600,
        }
    }
}

#[derive(Debug, Clone, Builder)]
pub struct ToolRouterConfig {
    /// Enable query result caching
    #[builder(default = true)]
    pub enable_query_cache: bool,

    /// Maximum cached queries
    #[builder(default = "100")]
    pub max_cached_queries: usize,

    /// Enable ASCII optimization
    #[builder(default = true)]
    pub enable_ascii_optimization: bool,

    /// Enable keyword pre-filtering
    #[builder(default = true)]
    pub enable_keyword_prefilter: bool,

    /// Minimum keyword length for categorization
    #[builder(default = "3")]
    pub min_keyword_length: usize,
}

impl Default for ToolRouterConfig {
    fn default() -> Self {
        ToolRouterConfig {
            enable_query_cache: true,
            max_cached_queries: 100,
            enable_ascii_optimization: true,
            enable_keyword_prefilter: true,
            min_keyword_length: 3,
        }
    }
}
```

---

## 8. Implementation Plan

### Phase 1: GPU Acceleration (High Priority)

- [x] Implement Vulkan backend detection
- [x] Add dynamic VRAM budgeting for iGPU/dGPU
- [x] Implement n_gpu_layers calculation
- [ ] Test with Ryzen 5 8500G iGPU
- [ ] Benchmark performance improvements

### Phase 2: Session Management (High Priority)

- [x] Implement rolling window context management
- [x] Add context shifting with preserve ratio
- [x] Implement selective cache clearing
- [ ] Test session reuse scenarios

### Phase 3: Tool Optimization (Medium Priority)

- [x] Replace manual cache with moka library
- [x] Implement structured error handling for batch execution
- [x] Add tool-specific cache invalidation
- [x] Optimize tool invocation patterns

### Phase 4: Tool Router Optimization (Low Priority)

- [x] Implement query result caching with LRU
- [x] Add ASCII optimization with memchr pre-filtering
- [x] Implement keyword pre-filtering
- [ ] Test tool selection performance improvements

### Phase 5: Embedding Optimization (Medium Priority)

- [x] Add FastEmbed ONNX Runtime GPU acceleration
- [x] Implement embedding caching with TTL
- [x] Add batch embedding processing
- [ ] Test embedding performance on iGPU/dGPU

### Phase 6: Speech Recognition (Medium Priority)

- [x] Add whisper-rs GPU acceleration
- [x] Implement Vulkan/OpenCL backend for Whisper
- [ ] Test transcription performance on iGPU/dGPU
- [ ] Benchmark speech recognition improvements

### Phase 7: Integration & Testing (Medium Priority)

- [x] Update configuration files
- [x] Add performance monitoring
- [x] Create benchmarking suite
- [ ] Document optimization results

---

## 9. Expected Performance Improvements

### 9.1 Quantitative Improvements

| Component                   | Current      | Target        | Improvement |
|-----------------------------|--------------|---------------|-------------|
| LLM Inference (2562 tokens) | 2+ minutes   | 15-30 seconds | 5-10x       |
| Session Reset Overhead      | 20ms         | 0-5ms         | 4-20x       |
| Tool Invocation             | 2-3 seconds  | < 1 second    | 2-3x        |
| Tool Selection              | 2-5ms        | 1-2ms         | 1.5-2x      |
| Embedding Generation        | 100-500ms    | 10-50ms       | 10-50x      |
| Speech Recognition          | 5-10 seconds | 50-200ms      | 25-50x      |
| Total Response Time         | 2+ minutes   | 15-30 seconds | 4-8x        |

### 9.2 Hardware-Specific Expectations

**Ryzen 5 8500G (iGPU):**

- **LLM Inference**: 2+ minutes → 20-30 seconds
- **Embedding Generation**: 100-500ms → 20-100ms
- **Speech Recognition**: 5-10 seconds → 100-200ms
- **Memory Usage**: Shared system RAM (+2.5GB for models + embeddings)
- **Thermal Impact**: Moderate increase

**Ryzen 9 9950X3D (RX 7900 XTX):**

- **LLM Inference**: 2+ minutes → 15-20 seconds
- **Embedding Generation**: 100-500ms → 10-50ms
- **Speech Recognition**: 5-10 seconds → 50-100ms
- **Memory Usage**: Dedicated VRAM (2GB models + 500MB embeddings + 200MB Whisper)
- **Thermal Impact**: Minimal increase

---

## 9. Monitoring & Debugging

### 9.1 Performance Metrics

```rust
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub llm_inference_time: Duration,
    pub session_reset_time: Duration,
    pub tool_invocation_time: Duration,
    pub embedding_generation_time: Duration,
    pub speech_recognition_time: Duration,
    pub total_response_time: Duration,
    pub gpu_utilization: f32,
    pub memory_usage: usize,
}
```

### 9.2 Debug Logging

```rust
debug!("Voice Assistant: Performance metrics - {:?}", metrics);
info!("Voice Assistant: GPU backend: {:?}", config.gpu_backend);
warn!("Voice Assistant: High memory usage detected: {} MB", memory_usage_mb);
```

---

## 10. Risk Assessment & Mitigation

### 10.1 GPU Acceleration Risks

**Risks:**

- Vulkan driver compatibility issues
- VRAM constraints on iGPU systems
- Dynamic layer offloading complexity
- Increased thermal load

**Mitigations:**

- Fallback to CPU backend
- Adaptive VRAM budgeting
- Thermal monitoring
- Conservative layer offloading for iGPU

### 10.2 Session Management Risks

**Risks:**

- Context overflow causing crashes
- Memory leaks from improper cleanup
- Reduced model accuracy from context reuse

**Mitigations:**

- Robust overflow detection
- Automatic session cleanup
- Context validation

### 10.3 Tool Optimization Risks

**Risks:**

- Stale cache results
- Cache memory bloat
- Structured error complexity
- Moka dependency overhead

**Mitigations:**

- Cache TTL and size limits
- Tool-specific invalidation
- Memory monitoring
- Fallback to simple cache if needed

### 10.4 Embedding Optimization Risks

**Risks:**

- ONNX Runtime GPU compatibility
- Embedding cache memory growth
- Batch processing latency
- Model loading overhead

**Mitigations:**

- CPU fallback for ONNX Runtime
- Embedding cache TTL and size limits
- Adaptive batch sizing
- Model preloading and reuse
- Fallback to simple error strings
- Monitor cache hit rates

---

## 11. Recommended Rust Libraries

### 11.1 Core Dependencies

```toml
[dependencies]
# GPU-accelerated LLM inference
llama-cpp-4 = { version = "0.4", features = ["vulkan", "openmp"] }

# High-performance async caching
moka = { version = "0.12", features = ["future"] }

# GPU-accelerated speech recognition
whisper-rs = { version = "0.10", features = ["vulkan", "opencl"] }

# Async utilities
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
thiserror = "1.0"
miette = "5.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"
```

### 11.2 Library Justification

**llama-cpp-4**: Production-ready bindings with Vulkan support, OpenMP multi-threading, dynamic layer offloading, and native context shifting.

**moka**: Inspired by Java's Caffeine cache, offers superior performance over manual HashMap+RwLock implementations with built-in TTL and eviction.

**whisper-rs**: Bindings to whisper.cpp with GPU acceleration support, reducing transcription time from seconds to milliseconds.

**fastembed**: Rust implementation of fastembed with ONNX Runtime GPU acceleration, providing 10-50x faster embedding generation through CUDA/OpenVINO/DirectML
backends.

---

## 12. Conclusion

The proposed performance optimizations target the primary bottlenecks in the Voice Assistant Service:

1. **GPU Acceleration** provides the most significant performance gains (5-15x for LLM, 25-50x for speech recognition)
2. **Smart Session Management** reduces unnecessary overhead through rolling window context (4-20x)
3. **Tool Optimization** reduces invocation overhead through moka caching and structured errors (2-3x)
4. **Embedding Optimization** accelerates semantic memory with ONNX Runtime GPU acceleration (10-50x)
5. **Speech Recognition** eliminates the transcription bottleneck with GPU acceleration (25-50x)

These optimizations maintain the system's architectural integrity while delivering substantial performance improvements across all target hardware
configurations. The modular implementation approach allows for gradual deployment and testing of each optimization independently.

**Expected Total Performance Improvement**: 4-8x faster end-to-end response times, reducing 2+ minute interactions to 15-30 seconds.

---

## 13. Comparison: Documented vs. Actual Performance

**Date**: 2026-07-14 **Hardware**: Ryzen 9 9950X3D + RX 7900 XTX (24GB VRAM)
**Models**: Qwen2.5-1.5B-Instruct (Q4_K_M), Whisper Large-v3-Turbo (Q5_0), BGE-Small-EN-v1.5-Q **Backend**: Vulkan with HIPBLAS (AMD ROCm)

### 13.1 Measured Metrics (from production log)

The following metrics were extracted from a single voice assistant interaction on 2026-07-14, involving one ReAct loop with 2 LLM calls, 1 tool invocation, and
1 embedding generation:

| Metric                         | Value                                              |
|--------------------------------|----------------------------------------------------|
| LLM calls (avg per call)       | 359.0 ms                                           |
| Tool calls (avg per call)      | 0.0 ms (direct MCP routing, negligible overhead)   |
| Embedding calls (avg per call) | 21.0 ms                                            |
| STT calls (avg per call)       | 0.0 ms (not measured in this interaction)          |
| Tool selection (avg per call)  | 0.00 ms (Nucleo-Matcher, sub-millisecond)          |
| ReAct loop (total)             | 741.0 ms                                           |
| Tool cache hit rate            | 0% (0 hits / 1 miss)                               |
| KV-Cache reuse (2nd LLM call)  | 2666 of 2714 tokens already in cache (98.2% reuse) |

### 13.2 Side-by-Side Comparison

| Component                | Old (CPU-only)      | Target (Concept)  | Actual (GPU/Vulkan)       | Improvement vs. Old |
|--------------------------|---------------------|-------------------|---------------------------|---------------------|
| **LLM Inference**        | 2+ min (120.000 ms) | 15-30 s (5-10x)   | **359 ms** per call       | **~334x**           |
| **Total Response Time**  | 2+ min (120.000 ms) | 30-45 s (3-4x)    | **741 ms** (1 ReAct loop) | **~162x**           |
| **Tool Invocation**      | 2-3 s               | < 1 s (2-3x)      | **~0 ms**                 | **>2000x**          |
| **Session Reset**        | 20 ms               | 0-5 ms (4-20x)    | **N/A** (KV-Cache reuse)  | **100%**            |
| **Embedding Generation** | 100-500 ms          | 10-50 ms (10-50x) | **21 ms**                 | **5-24x**           |
| **Tool Selection**       | 2-5 ms              | 1-2 ms (1.5-2x)   | **< 1 ms**                | **2-5x**            |

### 13.3 Key Observations

1. **GPU Acceleration (Vulkan/HIPBLAS)**: The single largest contributor to performance gain. The first LLM call processed ~2647 tokens in ~359 ms, compared to
   2+ minutes on CPU. This represents a ~334x improvement, far exceeding the 5-10x target.

2. **KV-Cache Reuse**: The second LLM call benefited from 2666 of 2714 tokens already being in the KV-Cache, requiring only 48 new tokens to be decoded. This
   eliminated nearly all recomputation overhead.

3. **Tool Invocation**: Direct MCP routing reduced tool invocation time from 2-3 seconds to effectively zero, exceeding the < 1 second target by orders of
   magnitude.

4. **Embedding Generation**: At 21 ms per call, this is within the target range of 10-50 ms, representing a 5-24x improvement over the CPU baseline.

5. **End-to-End Response Time**: A complete ReAct loop (reasoning, tool selection, tool execution, observation, final answer) completed in 741 ms — a ~162x
   improvement over the 2+ minute baseline and ~40-60x better than the 30-45 second target.

### 13.4 Target Achievement Summary

| Target                          | Goal     | Actual       | Status       |
|---------------------------------|----------|--------------|--------------|
| LLM Inference improvement       | 5-10x    | ~334x        | **Exceeded** |
| Total Response Time improvement | 3-4x     | ~162x        | **Exceeded** |
| Tool Invocation time            | < 1 s    | ~0 ms        | **Exceeded** |
| Session Reset overhead          | 0-5 ms   | N/A (cached) | **Exceeded** |
| Embedding Generation            | 10-50 ms | 21 ms        | **Achieved** |

### 13.5 Remaining Issues

While performance targets have been overwhelmingly exceeded, the following accuracy issues remain:

- **LLM Tool Selection Accuracy**: The 1.5B parameter model (Qwen2.5-1.5B) selected `audio_volume_up` with a `volume` parameter, which is not supported by that
  tool. The correct tool would have been `audio_set_volume`. The volume was set to ~68% instead of the requested 90%.
- **LLM Final Answer Accuracy**: The LLM reported "volume increased to 90%" despite the actual volume being ~68%, indicating a hallucination.
- **Root Cause**: These issues are related to model capacity (1.5B parameters), not infrastructure performance. A larger model (e.g., Qwen2.5-3B) or improved
  tool descriptors may resolve these accuracy issues without impacting the sub-second response time.

---

## 14. Grammar-Based Sampling (GBNF) for JSON Enforcement in the ReAct Loop

### 14.1 Problem Statement

**Current State (before GBNF):**

- The ReAct loop expects the LLM to produce strictly JSON responses in three formats:
    - `{"tool": "<tool_name>", "arguments": {...}}` for tool calls
    - `{"final_answer": "<text>"}` for final answers
    - `{"clarify": "<question>"}` for clarifying questions to the user
- Small models (1.5B–3B parameters) frequently ignore prompt instructions like "Output ONLY JSON!" and generate free-form text, Markdown blocks, or mixed
  formats
- This leads to parse errors ("Failed to extract JSON from LLM output") that block the ReAct loop or cause infinite loops

**Observed Error Patterns (from `voice-assistant-log.txt`):**

- **Rounds 1, 2**: LLM wraps JSON in Markdown code blocks (`` ```json ... ``` ``), causing the JSON parser to fail
- **Round 4**: LLM generates free-form explanations before the JSON object
- **Round 6**: LLM produces no JSON at all, but a natural-language response
- **Round 7+**: Infinite loop of repeated `app_launcher_search_apps` calls because the model retries tool invocation after a parse error

**Why Prompt Engineering Is Not Sufficient:**

- Prompt instructions are "soft" constraints — the model can ignore them at any time
- Especially during complex tasks (tool selection with multiple arguments), small models fall back to free-form text mode
- Even large models struggle to maintain strict JSON output with long contexts and many tools
- Each parse error costs a complete ReAct loop iteration cycle (LLM generation + parse attempt + error handling)

### 14.2 Solution: GBNF Grammar-Based Sampling

**Principle:**

Instead of asking the model in the prompt to output JSON, a **GBNF grammar** (llama.cpp Grammar-Based Sampling) is inserted into the sampler chain. This grammar
**physically prohibits** the generation of non-JSON tokens. The model cannot produce any free-form text — every generated token must conform to the grammar.

**How GBNF Works:**

1. **Token Masking**: After each generated token, llama.cpp computes which tokens are grammatically valid next
2. **Sampler Chain**: The grammar sampler is inserted before Temperature/Top-K/Top-P in the chain and restricts the token space
3. **Physical Constraint Enforcement**: The model cannot generate tokens outside the grammar, regardless of prompt instructions or model size

**GBNF Grammar for the ReAct Loop:**

```gbnf
root ::= "{" ws (tool_call | final_answer | clarify) ws "}"
tool_call ::= "\"tool\"" ws ":" ws string ws "," ws "\"parameters\"" ws ":" ws value
final_answer ::= "\"final_answer\"" ws ":" ws string
clarify ::= "\"clarify\"" ws ":" ws clarify_object
clarify_object ::= "{" ws "\"question\"" ws ":" ws string ws "}"
ws ::= [ \t\n]*
string ::= "\"" char* "\""
char ::= [^"\\] | "\\" escape
escape ::= ["\\/bfnrt] | "u" hex hex hex hex
hex ::= [0-9a-fA-F]
value ::= string | number | object | array | "true" | "false" | "null"
number ::= "-"? (0 | [1-9] [0-9]*) ("." [0-9]+)? ([eE] [-+]? [0-9]+)?
object ::= "{" ws (string ws ":" ws value (ws "," ws string ws ":" ws value)*)? ws "}"
array ::= "[" ws (value (ws "," ws value)*)? ws "]"
```

**What This Grammar Enforces:**

- The response **must** start with `{` and end with `}`
- The first field **must** be either `"tool"`, `"final_answer"`, or `"clarify"`
- For `"tool"`, a `"parameters"` field with an arbitrary JSON value **must** follow (aligned with OpenAPI standard)
- For `"final_answer"`, a string value **must** follow
- For `"clarify"`, a nested object `{"question": "..."}` **must** follow (structured format for clarifying questions)
- No Markdown code blocks, no free-form text, no comments

### 14.3 Implementation

**Architecture:**

The implementation uses the native `LlamaSampler::grammar()` API of the `llama-cpp-4` crate (v0.4.2). The grammar is inserted as an additional sampler into the
existing sampler chain.

**Components:**

1. **`REACT_GRAMMAR` constant** (`services/voice_assistant/src/llm.rs`):
    - Static GBNF grammar definition as a `&str` constant
    - Root rule: `root` — defines the entry point
    - Uses `"parameters"` (not `"arguments"`) to align with OpenAPI standard and system prompt
    - `"clarify"` uses a nested object `{"question": "..."}` for structured clarifying queries
    - Supports all JSON types: strings, numbers, objects, arrays, booleans, null

2. **`create_sampler(use_grammar: bool)` method** (`LlmInferenceEngine`):
    - When `true`: Prepends `LlamaSampler::grammar(&model, REACT_GRAMMAR, "root")` to the chain
    - When `false`: Normal chain without grammar (for summarization, where free-form text is desired)
    - Chain order: Grammar → Temperature → Top-K → Top-P → Dist

3. **`use_grammar` parameter** (propagated through all layers):
    - `LlmWorker::generate(system_prompt, conversation, max_tokens, use_grammar)` → async API
    - `LlmWorkerCommand::Generate { ..., use_grammar }` → command enum
    - `handle_generate(..., use_grammar)` → worker thread function
    - `session.sampler = engine.create_sampler(use_grammar)` → sampler is set before each generation

4. **Call sites:**
    - **ReAct loop** (`react.rs:172`): `worker.generate(..., true)` — grammar enabled
    - **Summarization** (`react.rs:432`): `worker.generate(..., false)` — grammar disabled (free-form summary desired)

5. **System prompt** (`tool_catalog.rs:build_system_prompt`):
    - Structured with Markdown sections (`# ROLE`, `# OUTPUT FORMAT`, `# CRITICAL RULES & PIPELINES`) for better LLM instruction weighting
    - Defines the app-launching pipeline: search → read results → execute (no premature `final_answer`)
    - Enforces strict schema adherence and exact string copying from tool results
    - Removed the problematic rule "After a tool has been executed successfully, always respond with a final_answer" that blocked multi-step pipelines

**Code Example (Sampler Creation):**

```rust
pub fn create_sampler(&self, use_grammar: bool) -> LlamaSampler {
    if use_grammar {
        debug!("LLM: creating sampler with GBNF grammar enforcement");
        LlamaSampler::chain_simple([
            LlamaSampler::grammar(&self.model, REACT_GRAMMAR, "root"),
            LlamaSampler::temp(self.config.temperature),
            LlamaSampler::top_k(self.config.top_k),
            LlamaSampler::top_p(self.config.top_p, 1),
            LlamaSampler::dist(0),
        ])
    } else {
        LlamaSampler::chain_simple([
            LlamaSampler::temp(self.config.temperature),
            LlamaSampler::top_k(self.config.top_k),
            LlamaSampler::top_p(self.config.top_p, 1),
            LlamaSampler::dist(0),
        ])
    }
}
```

### 14.4 Performance Impact

**Overhead:**

- The grammar sampler computes the set of valid next tokens after each generated token
- For simple grammars (such as the JSON grammar above), this overhead is **minimal** (< 1% of generation time)
- The grammar reduces the token space, which can slightly accelerate the sampling phase

**Net Effect:**

- **No more parse errors**: Every generated output is valid JSON in the expected format
- **No more retry loops**: Eliminates the "Failed to extract JSON" → retry → fail → retry cycles
- **Saved ReAct iterations**: Previously 2-3 iterations were wasted on parse errors; these are fully eliminated
- **Reduced context size**: Fewer error messages in context → smaller prompts → faster inference

### 14.5 Limitations and Constraints

**What GBNF Does **Not** Solve:**

- **Wrong parameter names**: The model can generate `{"tool": "app_launcher_exec", "arguments": {"uri": "..."}}` even though the correct parameter is
  `desktop_file`. The grammar allows arbitrary strings as keys — it validates only the JSON structure, not the semantics.
- **Wrong tool names**: The grammar allows arbitrary strings as tool names. An invalid tool name like `"app_launcher_launch"` would pass the grammar but fail at
  runtime.
- **Wrong argument values**: The grammar allows arbitrary JSON values as arguments. Semantic validation (e.g., "the file must exist") still occurs at runtime.

**Potential Future Extensions:**

- **Dynamic grammar generation**: Instead of a static grammar, the grammar could be generated at runtime from the registered tool schemas. This would also
  validate tool names and parameter keys.
- **JSON Schema → GBNF conversion**: Tools like `jsonschema2gbnf` can automatically convert a JSON Schema into a GBNF grammar that also validates value ranges
  and types.
- **Lazy grammar**: The `grammar_lazy_patterns` API of llama-cpp-4 activates the grammar only when certain trigger patterns appear in the output. This could be
  useful when the model should sometimes produce free-form text (e.g., for intermediate reasoning).

### 14.6 Summary

| Aspect                   | Before GBNF                             | With GBNF                                        |
|--------------------------|-----------------------------------------|--------------------------------------------------|
| **JSON Compliance**      | Prompt instruction (ignorable)          | Physically enforced (unbypassable)               |
| **Parse Error Rate**     | High (rounds 1, 2, 4, 6 in log)         | 0% (grammatically impossible)                    |
| **Wasted Iterations**    | 2-3 per ReAct loop (parse + retry)      | 0                                                |
| **Free-Form Generation** | Possible (causes parse error)           | Impossible                                       |
| **Clarifying Questions** | Not supported (model had no way to ask) | Supported via `{"clarify": {"question": "..."}}` |
| **Performance Overhead** | N/A                                     | < 1% (grammar sampling)                          |
| **Semantic Validation**  | Runtime check                           | Runtime check (unchanged)                        |
| **Model Independence**   | Only large models adhere to JSON        | All model sizes produce JSON                     |
