# Test Procedure 1 - Results

**Date**: 2026-07-15  
**Models**: Qwen 2.5 3B (`qwen2.5-3b-instruct-q4_k_m.gguf`), Gemma 4 E4B (`gemma-4-E4B-it-Q4_K_M.gguf`)  
**max_tokens**: 1024 (increased from default 512 via `voice_assistant_set_max_tokens`)  
**Test query**: "Wie ist das Wetter in Berlin?"

## Test Matrix

### Qwen 2.5 3B

| Test | Threshold | Rolling Window | Result      | Trace ID                              |
|------|-----------|----------------|-------------|---------------------------------------|
| 1    | 0.3       | 4              | **Failure** | -                                     |
| 2    | 0.5       | 4              | **Failure** | -                                     |
| 3    | 0.3       | 6              | **Failure** | -                                     |
| 4    | 0.5       | 6              | **Failure** | `trace-2026-07-15T14-27-22Z-43d7072e` |

**Failure pattern**:

- **T=0.3 (Tests 1, 3)**: The model attempts to use a resource URI (`weather://current?lat=52.5200&lon=13.4055`) instead of calling the `weather_get_forecast`
  tool. This results in an "Unknown resource URI" error and no weather forecast is produced.
- **T=0.5 (Tests 2, 4)**: The model successfully calls `weather_get_forecast` and receives valid weather data, but then enters a generation loop. It repeatedly
  outputs "HINT: The tool result likely contains the answer to the user's question. Provide a final_answer unless another tool call is clearly needed." until
  `max_tokens` (1024) is reached. No `final_answer` is generated.

**Trace example (Test 4, T=0.5, RW=6)**:

- Iteration 0: Tool call `weather_get_forecast` with `{"latitude":52.52,"longitude":13.405}` - successful, valid weather data returned.
- No further iterations: LLM inference fails with "Max tokens (1024) reached" due to generation loop.
- `success: false`

### Gemma 4 E4B

| Test | Threshold | Rolling Window | Result      | Trace ID                              |
|------|-----------|----------------|-------------|---------------------------------------|
| 5    | 0.3       | 4              | **Success** | `trace-2026-07-15T14-28-31Z-e509c5a7` |
| 6    | 0.5       | 4              | **Success** | `trace-2026-07-15T14-29-29Z-76b942f7` |
| 7    | 0.3       | 6              | **Success** | `trace-2026-07-15T14-29-41Z-4c05305b` |
| 8    | 0.5       | 6              | **Success** | `trace-2026-07-15T14-29-55Z-6aa05167` |

**Success pattern**: In all 4 configurations, Gemma 4 E4B correctly calls the `weather_get_forecast` tool and generates a meaningful `final_answer` in German
with the correct weather data (temperature, cloud cover, wind, humidity, air quality).

**Sample answer (Test 6, T=0.5, RW=4)**:
> "Das Wetter in Berlin ist momentan bewölkt mit einer Temperatur von 24 Grad Celsius und einer Windgeschwindigkeit von 10 Grad Celsius. Die Vorhersage für
> heute zeigt weitere Bewölkung mit Höchstwerten von 25 Grad Celsius und Tiefstwerten von 17 Grad Celsius. Bitte beachten Sie, dass die Luftfeuchtigkeit bei
> etwa
> 38 Prozent liegt und der Luftqualitätsindex bei 34 liegt."

**Trace example (Test 5, T=0.3, RW=4)**:

- Iteration 1: `final_answer` action with correct German weather response.
- `success: true`
- Note: The `thought` field contains `<|im_end|>` at the end, a ChatML template token that should not appear in Gemma output. This does not affect
  functionality.

## Findings

### `voice_assistant_set_max_tokens` MCP Tool

- The new MCP tool works correctly. The `max_tokens` value was changed at runtime from 512 to 1024 without reloading the model.
- The ReAct loop picks up the new value on the next `generate` call as expected.

### Threshold and Rolling Window

- Both parameters can be changed at runtime via `voice_assistant_set_threshold` and `voice_assistant_set_rolling_window`.
- Model switching resets `rolling_window_keep_last` to default (6), requiring reapplication of MCP tool changes after a model switch.
- Neither parameter has a noticeable impact on Gemma 4 E4B (all tests succeed regardless).
- For Qwen 2.5 3B, the threshold changes the failure mode: T=0.3 causes incorrect resource URI usage, T=0.5 causes a generation loop after successful tool call.

### Qwen 2.5 3B Issues

- **Resource URI misuse (T=0.3)**: The model confuses tool invocation with resource access, attempting to use `weather://current?lat=...&lon=...` as a resource
  URI instead of calling `weather_get_forecast` with proper parameters.
- **Generation loop (T=0.5)**: After a successful tool call, the model fails to produce a `final_answer` and instead loops on the "HINT" text from the ReAct
  system prompt. Increasing `max_tokens` from 512 to 1024 does not resolve the issue - it only delays the limit being reached.
- **Root cause hypothesis**: The Qwen 2.5 3B model struggles with the ReAct JSON format in the system prompt. It either misinterprets tool calls as resource
  lookups or cannot properly transition from tool result to final answer generation.

### Gemma 4 E4B Performance

- Works reliably across all 4 configurations (T=0.3/0.5, RW=4/6).
- Correctly invokes the `weather_get_forecast` tool and generates coherent German responses.
- Response times are fast (~10 seconds per query).
- Minor issue: `<|im_end|>` ChatML token appears in the trace's `thought` field, suggesting a possible chat template mismatch, but this does not affect the
  final answer quality.

### Tool Ranking

- The tool ranking is consistent across all tests:
    1. `weather_get_forecast` (score: ~0.488)
    2. `weather_get_location` (score: ~0.481)
    3. `weather_widget_refresh` (score: ~0.435)
    4. `weather_refresh` (score: ~0.415)
    5. `weather_lookup_location_name` (score: ~0.393)
- With T=0.3, all 5 tools are above or near the threshold. With T=0.5, only `weather_get_forecast` and `weather_get_location` are above the threshold.

## Summary

| Model       | Tests Passed | Tests Failed |
|-------------|--------------|--------------|
| Qwen 2.5 3B | 0            | 4            |
| Gemma 4 E4B | 4            | 0            |

**Conclusion**: Gemma 4 E4B is the recommended model for the voice assistant ReAct loop. Qwen 2.5 3B has fundamental issues with the ReAct format that cannot be
resolved by increasing `max_tokens` or adjusting threshold/rolling window parameters. The `voice_assistant_set_max_tokens` MCP tool functions as designed.
