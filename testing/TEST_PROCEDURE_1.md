# Test Procedure 1: Voice Assistant ReAct Loop mit konfigurierbarem Rolling Window

## Ziel

Validierung der Fixes 2–6 und der neuen konfigurierbaren Parameter (`rolling_window_keep_last`, `tool_selection_threshold`) im Voice Assistant Service.

## Voraussetzungen

- Smearor Swipe Launcher läuft mit geladenem Voice Assistant Service
- MCP Server ist verbunden
- Modelle verfügbar: `models/qwen2.5-3b-instruct-q4_k_m.gguf` und `models/gemma-4-E4B-it-Q4_K_M.gguf`
- Wetter-Tool (`weather_get_forecast`) und App-Launcher-Tool sind registriert

## Getestete Fixes

| Fix    | Beschreibung                                                                      | Status           |
|--------|-----------------------------------------------------------------------------------|------------------|
| Fix 2  | Rolling-Window-Trimming schont Context-Message und Tool-Responses                 | ✅ Implementiert |
| Fix 3  | Session-Reset zwischen ReAct-Iterationen durch `clear_kv_cache` ersetzen          | ✅ Implementiert |
| Fix 4  | Parameter-Schema in Tool-Fehlermeldung aufnehmen                                  | ✅ Implementiert |
| Fix 5  | Final-Answer-Hinweis nach erfolgreichem Tool-Call                                 | ✅ Implementiert |
| Konfig | `rolling_window_keep_last` konfigurierbar (services.toml, MCP Tool, MCP Resource) | ✅ Implementiert |

## Test-Abdeckung

### Schritt 1: MCP Resource validieren

```
read_resource: voice_assistant://llm
```

**Erwartet**: JSON-Antwort enthält `rolling_window_keep_last`, `context_keep_ratio`, `min_preserve_tokens`.

### Schritt 2: MCP Tool — Rolling Window ändern

```
voice_assistant_set_rolling_window(keep_last=4)
```

**Erwartet**: Bestätigungsnachricht "Rolling window keep_last set to 4".

### Schritt 3: MCP Resource — Wert bestätigen

```
read_resource: voice_assistant://llm
```

**Erwartet**: `rolling_window_keep_last` ist jetzt `4`.

### Schritt 4: MCP Tool — Threshold ändern

```
voice_assistant_set_threshold(threshold=0.5)
```

**Erwartet**: Bestätigungsnachricht "Tool selection threshold set to 0.50".

### Schritt 5: Training Mode aktivieren

```
voice_assistant_training_start(label="test_<config>")
```

### Schritt 6: Voice Assistant Text-Submit

```
voice_assistant_submit_text(text="Wie ist das Wetter in Berlin?")
```

### Schritt 7: Status abfragen (wiederholt bis State != "ThinkingLlm")

```
read_resource: voice_assistant://status
```

**Beobachten**:

- `state`: "ThinkingLlm" → "Idle" / "Error" / "Speaking"
- `response_type`: "final_answer" / "clarify" / null
- `final_answer`: Text der Antwort oder Fehlermeldung
- `last_tool_ranking`: Semantische Tool-Auswahl + Scores

### Schritt 8: Training Trace abrufen

```
voice_assistant_training_end()
voice_assistant_training_get(limit=1)
```

**Beobachten**:

- `steps[]`: Jede ReAct-Iteration mit `thought` (LLM-Output), `action` (Tool/Resource/Final), `parameters`, `observation` (Tool-Result), `answer`
- `success`: true/false

### Schritt 9: LLM Resource nach Test

```
read_resource: voice_assistant://llm
```

**Beobachten**: `last_tool_calls` enthält die aufgerufenen Tools.

## Test-Matrix

4 Kombinationen pro Modell:

| Test | Threshold | Rolling Window | Label             |
|------|-----------|----------------|-------------------|
| 1    | 0.3       | 4              | `<model>_t03_rw4` |
| 2    | 0.5       | 4              | `<model>_t05_rw4` |
| 3    | 0.3       | 6              | `<model>_t03_rw6` |
| 4    | 0.5       | 6              | `<model>_t05_rw6` |

## Modell-Wechsel

```
voice_assistant_switch_model(model_path="models/gemma-4-E4B-it-Q4_K_M.gguf")
```

Warten bis `voice_assistant://llm` den neuen `model_path` anzeigt.

**Wichtig**: Nach Modellwechsel wird `rolling_window_keep_last` auf den Config-Default (6) zurückgesetzt, da `to_llm_config_with_model` die Service-Config
liest. MCP-Tool-Änderungen müssen nach Modellwechsel erneut angewendet werden.

## Test-Kommandos für manuelle Wiederholung

### Qwen 2.5 3B — Vollständige Test-Sequenz

```text
# Setup
voice_assistant_set_threshold(threshold=0.3)
voice_assistant_set_rolling_window(keep_last=4)

# Test 1: T=0.3, RW=4
voice_assistant_training_start(label="qwen_t03_rw4")
voice_assistant_submit_text(text="Wie ist das Wetter in Berlin?")
# Warten bis State != ThinkingLlm
voice_assistant_training_end()
voice_assistant_training_get(limit=1)

# Test 2: T=0.5, RW=4
voice_assistant_set_threshold(threshold=0.5)
voice_assistant_training_start(label="qwen_t05_rw4")
voice_assistant_submit_text(text="Wie ist das Wetter in Berlin?")
# Warten bis State != ThinkingLlm
voice_assistant_training_end()
voice_assistant_training_get(limit=1)

# Test 3: T=0.3, RW=6
voice_assistant_set_threshold(threshold=0.3)
voice_assistant_set_rolling_window(keep_last=6)
voice_assistant_training_start(label="qwen_t03_rw6")
voice_assistant_submit_text(text="Wie ist das Wetter in Berlin?")
# Warten bis State != ThinkingLlm
voice_assistant_training_end()
voice_assistant_training_get(limit=1)

# Test 4: T=0.5, RW=6
voice_assistant_set_threshold(threshold=0.5)
voice_assistant_training_start(label="qwen_t05_rw6")
voice_assistant_submit_text(text="Wie ist das Wetter in Berlin?")
# Warten bis State != ThinkingLlm
voice_assistant_training_end()
voice_assistant_training_get(limit=1)
```

### Gemma 4 E4B — Vollständige Test-Sequenz

```text
# Modell wechseln
voice_assistant_switch_model(model_path="models/gemma-4-E4B-it-Q4_K_M.gguf")
# Warten bis model_path in voice_assistant://llm aktualisiert ist

# Setup (nach Modellwechsel nötig, da Config zurückgesetzt wird)
voice_assistant_set_threshold(threshold=0.3)
voice_assistant_set_rolling_window(keep_last=4)

# Test 1–4: Wie oben, Labels mit "gemma_" Prefix
```

## Zusätzliche Test-Fälle

### Fix 5 (Schema-Hint) isoliert testen

```text
voice_assistant_training_start(label="fix5_schema_hint")
voice_assistant_submit_text(text="Wie ist das Wetter in Berlin?")
# Erwartet: LLM ruft weather_lookup_coordinates mit latitude/longitude auf
# → Fehler: "Missing 'place_name' parameter"
# → Schema-Hint wird in Fehlermeldung eingefügt
# → LLM sollte in nächster Iteration place_name verwenden
voice_assistant_training_end()
voice_assistant_training_get(limit=1)
```

### Fix 6 (Answer-Hint) isoliert testen

```text
voice_assistant_training_start(label="fix6_answer_hint")
voice_assistant_submit_text(text="What time is it?")
# Erwartet: LLM ruft get_current_time auf
# → Answer-Hint wird angefügt (Info-Tool + Frage-Indikator)
# → LLM sollte final_answer geben
voice_assistant_training_end()
voice_assistant_training_get(limit=1)
```

## Beobachtungs-Checkliste

- [ ] MCP Resource `voice_assistant://llm` zeigt `rolling_window_keep_last`
- [ ] MCP Tool `set_rolling_window` ändert den Wert live
- [ ] MCP Tool `set_threshold` ändert den Threshold live
- [ ] Training Trace enthält alle ReAct-Schritte
- [ ] `last_tool_calls` in Resource zeigt aufgerufene Tools
- [ ] Fix 5: Schema-Hint in Tool-Fehlermeldung sichtbar
- [ ] Fix 6: Answer-Hint nach Info-Tool-Call gesendet
- [ ] Rolling Window Trimming erhält Context-Message (Index 0)
- [ ] `clear_kv_cache` wird statt `create_session` bei Prompt-Shrink verwendet
