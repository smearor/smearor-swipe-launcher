# espeak-ng-rs 0.1.3 Upgrade Concept

## 1. Motivation

The Smearor Swipe Launcher's Voice Assistant uses the `espeak-ng` Rust crate (v0.1) for two critical TTS paths:

1. **Direct synthesis** (`phonemize_enabled: false`): `espeak_ng::text_to_pcm` produces PCM audio directly.
2. **Phonemization for Piper/Kokoro** (`phonemize_enabled: true`): `espeak_ng::text_to_ipa` converts text to IPA phonemes, which are then fed to an ONNX model
   for neural TTS inference.

The current version (0.1.x) has known issues that affect speech quality, particularly for German output:

- Speech runs ~1.2x too slow at default rate.
- Fricative consonants (`s`, `f`, stop bursts) sound distorted due to linear resampling of noise samples.
- German compound numbers (`21`-`91`) read "eins und zwanzig" instead of "einundzwanzig".
- Times (`10:30`) are split into separate clauses instead of being read as times.
- minus signs, degree symbols, and dotted acronyms are mishandled.
- Crash risk on malformed dictionary bytes.

Version 0.1.3 (released 2026-08-01) fixes all of these issues and adds new API surface (`Builder`, `async_synth`, `SynthEvent`, SSML support) that can benefit
Smearor in future phases.

### Current Workarounds in Smearor

The file `services/voice_assistant/src/tts.rs` contains several manual workarounds in `preprocess_text_for_tts` that compensate for espeak-ng 0.1.x
deficiencies:

| Workaround                            | Lines   | Purpose                                             |
|---------------------------------------|---------|-----------------------------------------------------|
| Regex `\d{1,2}:\d{2}` for times       | 336     | Pre-normalize times before espeak-ng mangles them   |
| `.` to `,` for German decimals        | 349-353 | espeak-ng 0.1.x reads "3.14" as "314" in German     |
| Hyphen replace `(\w)-(\w)` to `$1 $2` | 375-376 | Prevent compound-word merging                       |
| Compound splits (`Luftqualität` etc.) | 383-390 | Force espeak-ng to treat parts as separate words    |
| Umlaut restore (`ae` to `ä`)          | 361     | Workaround for `text-processing-rs` transliteration |

With 0.1.3, the first three workarounds become obsolete. The compound splits and umlaut restore remain (the latter is a `text-processing-rs` issue, not an
espeak-ng issue).

---

## 2. Crate Structure

This is a dependency upgrade, not a new feature. No new crates are created. The affected crates are:

| Crate                             | Path                        | Role                                                 |
|-----------------------------------|-----------------------------|------------------------------------------------------|
| `smearor-voice-assistant-service` | `services/voice_assistant/` | Uses `espeak-ng` for TTS synthesis and phonemization |
| Workspace root                    | `Cargo.toml`                | Defines `espeak-ng` workspace dependency             |

### Affected Files

| File                                  | Changes                                                         |
|---------------------------------------|-----------------------------------------------------------------|
| `Cargo.toml` (workspace)              | Bump `espeak-ng` version from `0.1` to `0.1.3`                  |
| `services/voice_assistant/src/tts.rs` | Simplify `preprocess_text_for_tts`, remove obsolete workarounds |
| `services/voice_assistant/Cargo.toml` | No change (uses `{ workspace = true }`)                         |

---

## 3. Current espeak-ng API Usage

### 3.1 Functions Used

| Function                                             | Location     | Purpose                                        |
|------------------------------------------------------|--------------|------------------------------------------------|
| `espeak_ng::text_to_pcm(&lang, &text)`               | `tts.rs:289` | Direct PCM synthesis (non-ONNX path)           |
| `espeak_ng::text_to_ipa(&lang, &text)`               | `tts.rs:430` | IPA phonemization for Piper/Kokoro ONNX models |
| `espeak_ng::install_bundled_languages(&dir, &langs)` | `tts.rs:214` | Install bundled dictionary data as fallback    |

### 3.2 Data Path

```
LLM final_answer
    |
    v
preprocess_text_for_tts(text)   <-- manual workarounds (regex, compound splits)
    |
    +-- phonemize_enabled=false -->  espeak_ng::text_to_pcm  -->  cpal playback
    |
    +-- phonemize_enabled=true  -->  espeak_ng::text_to_ipa  -->  ONNX inference  -->  cpal playback
```

### 3.3 espeak-ng Data Resolution

The function `ensure_espeak_data` (lines 182-225) resolves espeak-ng dictionary data in this order:

1. System installed: `/usr/lib/x86_64-linux-gnu/espeak-ng-data` (preferred, full German dictionary)
2. System installed: `/usr/share/espeak-ng-data`
3. System installed: `/usr/lib/espeak-ng-data`
4. Bundled fallback: `espeak_ng::install_bundled_languages` to temp dir

The system path is preferred because the bundled data only includes a limited dictionary that produces truncated phonemes for inflected German words (e.g.
"verfügbaren" to "fˈɛrfɛr").

---

## 4. Changes in espeak-ng-rs 0.1.3

### 4.1 Fixes Relevant to Smearor

| Fix                                                                                | Impact on Smearor                                | Priority |
|------------------------------------------------------------------------------------|--------------------------------------------------|----------|
| **Default speaking rate calibrated** (1.20x to ~1.05x duration ratio)              | All TTS output speaks at correct pace            | High     |
| **Fricative/stop consonant synthesis** (DoSample2 loop instead of linear resample) | `s`, `f`, stop bursts sound natural              | High     |
| **Crash safety** (`is_letter` no longer panics on bad dictionary bytes)            | Eliminates crash risk with bundled data fallback | High     |
| **German compound numbers** ("einundzwanzig" instead of "eins und zwanzig")        | Correct German number pronunciation              | High     |
| **German compound ordinals** ("einundzwanzigste" instead of "zwanzig erste")       | Correct date pronunciation                       | Medium   |
| **Time reading** ("10:30" to "zehn uhr dreißig" with language connectors)          | Eliminates need for manual time regex            | High     |
| **Dotted acronyms** ("U.S.A." no longer splits into three clauses)                 | Better acronym pronunciation                     | Medium   |
| **Period between words/digits** ("cat.Dog" to "cat dot dog")                       | Better punctuation handling                      | Medium   |
| **Minus sign** ("-5" to "minus five" in all languages)                             | Correct negative number pronunciation            | High     |
| **Degree sign in more languages** (German "Grad")                                  | Correct temperature pronunciation                | High     |
| **Emoji reading** (emoji names from dictionary)                                    | LLM outputs with emojis are spoken correctly     | Low      |
| **CamelCase splitting** ("CamelCase" to "Camel Case")                              | Better app-name pronunciation                    | Low      |
| **Hyphen between letters is word break** ("well-known" to "well known")            | Eliminates need for manual hyphen regex          | Medium   |
| **Leading-zero numbers** ("007" to "zero zero seven")                              | Correct phone-number style reading               | Low      |
| **Hex literals** ("0x1F" no longer reads "zero by one F")                          | Technical text handling                          | Low      |

### 4.2 New API Surface (Future Phases)

| API                            | Purpose                                                      | Smearor Use Case                                                 |
|--------------------------------|--------------------------------------------------------------|------------------------------------------------------------------|
| `Builder` / `VoiceSpec`        | Structured engine configuration with voice selection         | Replace per-call language string with persistent engine instance |
| `async_synth`                  | Non-blocking synthesis entry point                           | Integrate TTS into Tokio executor instead of blocking thread     |
| `SynthEvent` / `EventKind`     | Word-boundary events during synthesis                        | Visual word highlighting in widget                               |
| `OutputMode`                   | Streaming/callback-style synthesis                           | Real-time audio streaming instead of batch                       |
| `TextToPhonemesOptions`        | Configurable phonemization (separator, tie, IPA vs mnemonic) | Fine-tune phoneme output for different ONNX models               |
| SSML (`translate::ssml`)       | `<break>`, `<prosody>`, `<say-as>`, `<mark>`                 | Replace manual `preprocess_text_for_tts` with structured markup  |
| `SoundIcon` / `SoundIconTable` | Inline `<name>` sound icon markup                            | Audio cues for widget interactions                               |
| `mbrola` module                | MBROLA-backed voices                                         | Alternative voice options                                        |
| `synthesize::klatt`            | Klatt formant synthesizer                                    | Alternative synthesis engine                                     |
| `synthesize::tempo`            | Tempo/prosody control                                        | Speed-adjustable TTS output                                      |

### 4.3 Changed Behavior

- Regional voices resolve phoneme table via primary BCP-47 subtag (e.g. `de-DE` resolves `de`).
- Japanese katakana handling routes through general `.replace` mechanism.

---

## 5. Implementation Phases

### Phase 1: Dependency Upgrade (Minimal, Low Risk)

**Goal**: Bump espeak-ng version and verify backward compatibility.

**Tasks**:

- Update `Cargo.toml` workspace dependency: `espeak-ng = { version = "0.1.3", features = ["bundled-data-de", "bundled-data-en"] }`
- Run `cargo update -p espeak-ng`
- Verify `cargo check -p smearor-voice-assistant-service` succeeds
- Verify existing `text_to_pcm`, `text_to_ipa`, and `install_bundled_languages` functions still exist (backward-compatible API)
- Test TTS output with a sample German sentence containing numbers, times, and temperatures

**Exit Criteria**: `cargo build` succeeds. TTS output is audible and at correct speed.

### Phase 2: Remove Obsolete Workarounds (Medium Risk)

**Goal**: Simplify `preprocess_text_for_tts` by removing workarounds that espeak-ng 0.1.3 now handles natively.

**Tasks**:

- Remove the time regex `\d{1,2}:\d{2}` from the TN span regex (line 336) — espeak-ng 0.1.3 reads times natively with language-specific connectors ("uhr",
  "heures")
- Remove the `.` to `,` decimal conversion for German (lines 349-353) — espeak-ng 0.1.3 reads "3.14" as "drei punkt vierzehn" in German
- Remove the hyphen replace regex `(\w)-(\w)` to `$1 $2` (lines 375-376) — espeak-ng 0.1.3 treats hyphens between letters as word breaks natively
- Keep the compound splits for `Luftqualität`, `Luftfeuchtigkeit`, `Luftfeuchte` (lines 383-390) — these are German compound words that espeak-ng may still not
  decompose correctly
- Keep the umlaut restore logic (line 361) — this is a `text-processing-rs` issue, not an espeak-ng issue
- Keep the general TN span regex for currencies, measurements, and complex numbers — `text-processing-rs` handles these better than espeak-ng's built-in
  normalization
- Test with diverse LLM outputs: weather reports (temperatures, times), app names (CamelCase, hyphens), numbers (compound, ordinals, negative)

**Exit Criteria**: TTS output for times, decimals, hyphens, and numbers is correct without manual preprocessing. No regression in compound word pronunciation.

### Phase 3: Verify System Data Compatibility (Low Risk)

**Goal**: Ensure the system espeak-ng-data path still works with 0.1.3's changed dictionary format.

**Tasks**:

- Test with system espeak-ng-data at `/usr/lib/x86_64-linux-gnu/espeak-ng-data`
- Test with bundled data fallback (unset `ESPEAK_DATA_PATH`)
- Verify German inflected words still pronounce correctly with system data
- Verify the 0.1.3 dictionary compiler (`dictionary::compile`) produces compatible output if system data needs regeneration

**Exit Criteria**: Both system and bundled data paths produce correct German speech output.

### Phase 4: Builder API Migration (Optional, Future)

**Goal**: Replace per-call `text_to_pcm`/`text_to_ipa` with a persistent `Builder`-configured engine instance.

**Tasks**:

- Create `EspeakNg` engine instance in `TtsEngine::new` using `Builder::new().language(...).build()`
- Replace `espeak_ng::text_to_pcm(&self.language, ...)` with `self.espeak_engine.text_to_pcm(...)`
- Replace `espeak_ng::text_to_ipa(&self.language, ...)` with `self.espeak_engine.text_to_ipa(...)`
- Remove `self.language` field (language is now configured on the engine)
- Test both Piper/Kokoro and direct synthesis paths

**Exit Criteria**: TTS engine holds a persistent espeak-ng instance. No per-call language string passing.

### Phase 5: Async Synthesis (Optional, Future)

**Goal**: Make TTS synthesis non-blocking using `async_synth`.

**Tasks**:

- Replace blocking `speak()` with `async fn speak()`
- Integrate into the Voice Assistant's Tokio runtime
- Replace the `play_audio` blocking sleep-loop with an async-compatible approach
- Add cancellation support (abort synthesis if user interrupts)

**Exit Criteria**: TTS synthesis runs non-blocking. Voice Assistant can process other messages during synthesis.

### Phase 6: SSML and Word Events (Optional, Future)

**Goal**: Use SSML markup and word-boundary events for fine-grained speech control.

**Tasks**:

- Replace remaining `preprocess_text_for_tts` regex workarounds with SSML markup (`<break>`, `<prosody>`, `<say-as>`)
- Implement `SynthEvent` / `EventKind` handling for word-boundary events
- Broadcast word-boundary events to the widget layer for visual highlighting
- Add SSML generation utility for common patterns (numbers, dates, temperatures)

**Exit Criteria**: `preprocess_text_for_tts` is reduced to SSML generation. Widget shows word-by-word highlighting during TTS playback.

---

## 6. Dependencies

### Current

| Dependency  | Version | Features                             |
|-------------|---------|--------------------------------------|
| `espeak-ng` | `0.1`   | `bundled-data-de`, `bundled-data-en` |

### After Phase 1

| Dependency  | Version | Features                             |
|-------------|---------|--------------------------------------|
| `espeak-ng` | `0.1.3` | `bundled-data-de`, `bundled-data-en` |

### After Phase 4 (Optional)

| Dependency  | Version | Features                                                                        |
|-------------|---------|---------------------------------------------------------------------------------|
| `espeak-ng` | `0.1.3` | `bundled-data-de`, `bundled-data-en`, `wav-analysis` (if WAV comparison needed) |

---

## 7. Testing and Verification

### 7.1 Phase 1 Tests

- **Build check**: `cargo build -p smearor-voice-assistant-service` succeeds
- **Unit tests**: All existing tests pass
- **Smoke test**: Voice Assistant speaks "Hallo, wie geht es dir?" at correct speed
- **German numbers**: "Die Temperatur beträgt 21 Grad" reads "einundzwanzig" (not "eins und zwanzig")
- **Times**: "Es ist 10:30 Uhr" reads "zehn uhr dreißig" (not "zehn" pause "dreißig")
- **Negative numbers**: "Minus 5 Grad" reads "minus fünf Grad" (not "fünf Grad")
- **Fricatives**: "Sechs scharfe Schlangen" — `s`, `ʃ` sounds are natural, not metallic

### 7.2 Phase 2 Tests

- **Times without regex**: "10:30" passes through `preprocess_text_for_tts` unchanged and espeak-ng reads it correctly
- **Decimals without conversion**: "22.9 Grad" passes through with `.` and espeak-ng reads "zweiundzwanzig punkt neun"
- **Hyphens without regex**: "Wallpaper-Themen" — espeak-ng reads "Wallpaper Themen" natively
- **Compound words still split**: "Luftqualität" still split to "Luft Qualität" (workaround kept)
- **Umlauts still restored**: TN output still has `ä`, `ö`, `ü` restored (workaround kept)
- **Weather report**: Full LLM weather answer with temperatures, times, percentages, wind speeds reads naturally

### 7.3 Phase 3 Tests

- **System data**: With `ESPEAK_DATA_PATH=/usr/lib/x86_64-linux-gnu/espeak-ng-data`, "verfügbaren" pronounces correctly
- **Bundled data**: Without system data, bundled fallback installs and basic German works
- **English path**: English TTS output correct with both data sources

### 7.4 Regression Tests

- **Piper model path**: ONNX inference with espeak-ng IPA phonemization produces correct audio
- **Kokoro model path**: Same as Piper
- **Direct PCM path**: `text_to_pcm` produces correct audio without ONNX
- **Audio playback**: cpal stream starts and completes without underruns
- **Resampling**: Model sample rate to cpal sample rate conversion still works

---

## 8. Risk Assessment

| Risk                                          | Likelihood | Impact | Mitigation                                                                                      |
|-----------------------------------------------|------------|--------|-------------------------------------------------------------------------------------------------|
| API breaking change despite semver            | Low        | High   | Phase 1 verifies all three functions still exist before proceeding                              |
| System espeak-ng-data incompatible with 0.1.3 | Low        | Medium | Phase 3 tests both data paths; bundled data as fallback                                         |
| German compound words still mispronounced     | Medium     | Low    | Keep compound split workarounds in Phase 2                                                      |
| TN span regex removal causes regression       | Low        | Medium | Keep general TN regex for currencies and measurements; only remove time and decimal workarounds |
| ONNX phoneme mapping changes                  | Very Low   | High   | IPA output format is stable across espeak-ng versions; verify with Piper model                  |

---

## 9. Future Enhancements (Out of Scope for Initial Upgrade)

- **MBROLA voices**: Alternative voice synthesis via MBROLA backend
- **Klatt formant synthesizer**: Lightweight formant-based synthesis as alternative to ONNX
- **Tempo/prosody control**: User-configurable speech speed via `synthesize::tempo`
- **Sound icons**: Audio cues for widget interactions via `SoundIcon` / `SoundIconTable`
- **WAV analysis**: Automated TTS quality regression testing via `wav-analysis` feature
- **Multi-language voice selection**: Runtime language switching via `VoiceSpec` without restarting the engine
- **Phoneme output customization**: `TextToPhonemesOptions` for model-specific phoneme formatting
