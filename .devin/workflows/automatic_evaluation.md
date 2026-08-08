---
description: Automatic Voice Assistant Evaluation — run modular test suites, compare models, generate structured reports
---

## Automatic Evaluation Skill

You are an evaluation harness for the Smearor Swipe Launcher Voice Assistant. You use the Voice Assistant MCP tools to run test cases, record traces, and
generate structured evaluation reports.

### Prerequisites

- Smearor Swipe Launcher is running with Voice Assistant Service loaded
- MCP Server is connected and voice assistant tools are available
- At least one GGUF model is available in the models directory
- The `launcher_get_logs` MCP tool is available for diagnostic log retrieval

### Parameters

The user may provide any of the following parameters. Use defaults when not specified:

- **models**: `current` (default), `all`, or comma-separated model filenames (e.g. `gemma-4-E4B-it-heretic-ara.Q4_K_M.gguf,qwen2.5-3b-instruct-q4_k_m.gguf`)
- **categories**: `all` (default), or comma-separated categories: `Weather,Time,AppLaunch,AreaConfig,Memory,Clarify`
- **test_ids**: `all` (default), or comma-separated test IDs (e.g. `weather_by_city,time_current`)
- **custom_query**: A custom query string to test (bypasses test_ids/categories selection)
- **matrix**: `false` (default) for quick mode, `true` for full parameter matrix
- **thresholds**: `0.3,0.5` (default) — comma-separated threshold values for matrix mode
- **rolling_windows**: `4,6` (default) — comma-separated rolling window values for matrix mode
- **max_tokens_list**: `512,1024` (default) — comma-separated max_tokens values for matrix mode
- **report_path**: File path to save the report (if not provided, output to chat)
- **baseline_path**: Path to a previous report file for regression comparison
- **clear_between**: `true` (default) — clear conversation between test cases
- **capture_logs**: `true` (default) — retrieve launcher logs after each test case for diagnostics
- **log_level**: `debug` (default) — minimum log level to capture: `trace`, `debug`, `info`, `warn`, `error`
- **log_target_prefix**: `smearor_voice_assistant` (default) — filter logs by tracing target prefix. Set to empty string for all logs.
- **log_since_seconds**: `120` (default) — only retrieve logs from the last N seconds
- **log_limit**: `200` (default) — maximum number of log entries to retrieve per test case

### Test Case Catalog

Use this catalog when `custom_query` is not provided. Filter by `categories` and `test_ids` parameters.

#### Weather

| ID                | Query                                  | Expected Tools                                     | Min Iter | Max Iter | Language |
|-------------------|----------------------------------------|----------------------------------------------------|----------|----------|----------|
| weather_by_city   | "Wie ist das Wetter heute in Bregenz?" | weather_lookup_coordinates, weather_get_forecast   | 1        | 5        | de       |
| weather_by_coords | "What's the weather at 47.5, 9.7?"     | weather_get_forecast                               | 1        | 3        | en       |
| weather_generic   | "Wie ist das Wetter?"                  | weather_lookup_coordinates or weather_get_forecast | 1        | 5        | de       |

#### Time

| ID            | Query                       | Expected Tools   | Min Iter | Max Iter | Language |
|---------------|-----------------------------|------------------|----------|----------|----------|
| time_current  | "Wie spät ist es?"          | get_current_time | 1        | 3        | de       |
| time_timezone | "What time is it in Tokyo?" | get_current_time | 1        | 3        | en       |

#### AppLaunch

| ID              | Query                           | Expected Tools      | Min Iter | Max Iter | Language |
|-----------------|---------------------------------|---------------------|----------|----------|----------|
| app_launch_name | "Öffne Firefox"                 | app_launcher_launch | 1        | 4        | de       |
| app_launch_list | "Welche Apps sind installiert?" | app_launcher_list   | 1        | 4        | de       |

#### AreaConfig

| ID                | Query                              | Expected Tools  | Min Iter | Max Iter | Language |
|-------------------|------------------------------------|-----------------|----------|----------|----------|
| area_config_query | "Welche Widgets sind im Macropad?" | get_area_config | 1        | 4        | de       |

#### Memory

| ID            | Query                                                      | Expected Tools | Min Iter | Max Iter | Language |
|---------------|------------------------------------------------------------|----------------|----------|----------|----------|
| memory_store  | "Merke dir, dass mein Lieblingslied Bohemian Rhapsody ist" | memory_store   | 1        | 4        | de       |
| memory_recall | "Was weiß du über mich?"                                   | memory_recall  | 1        | 4        | de       |

#### Clarify

| ID                | Query          | Expected Tools | Min Iter | Max Iter | Language |
|-------------------|----------------|----------------|----------|----------|----------|
| clarify_ambiguous | "Spiele Musik" | (varies)       | 1        | 5        | de       |

### Step 1: Discover Available Models

1. Read the `voice_assistant://models` resource.
2. Parse the JSON response. Extract `current_model` (string) and `available_models` (array of objects with `filename`, `path`, `size_mb`, `metadata`).
3. Determine which models to test based on the `models` parameter:
    - `current`: Use only the `current_model` value.
    - `all`: Use all entries in `available_models`.
    - Comma-separated list: Filter `available_models` by matching filenames.
4. Record the list of models to test.

### Step 2: Build Test Case List

1. If `custom_query` is provided: Create a single test case with `id=custom`, `query=custom_query`, `expected_tools=[]`, `min_iterations=1`, `max_iterations=5`,
   `language=de`, `category=Custom`.
2. Otherwise: Select test cases from the catalog above:
    - If `test_ids` is not `all`: Filter by the specified IDs.
    - If `categories` is not `all`: Filter by the specified categories.
    - If both are `all`: Use all test cases from the catalog.
3. Record the list of test cases to run.

### Step 3: Build Parameter Matrix

1. If `matrix` is `false`: Use a single parameter set: `threshold=0.3, rolling_window=6, max_tokens=1024`.
2. If `matrix` is `true`: Expand the cartesian product of `thresholds × rolling_windows × max_tokens_list`.
    - Parse comma-separated values into arrays.
    - Generate all combinations.
3. Record the list of parameter combinations.

### Step 4: Execute Evaluation

For each model in the model list, for each test case in the test case list, for each parameter combination in the matrix:

#### 4a: Switch Model (only when the model changes from the previous iteration)

1. Call `voice_assistant_switch_model` with `model_path` set to the model's path.
2. Poll `voice_assistant://llm` resource every 3 seconds until `model_path` matches the requested model path.
3. Wait an additional 5 seconds for KV cache initialization.
4. After a model switch, parameters are reset to config defaults. Reapply the current parameter combination (see 4b).

#### 4b: Set Parameters

1. Call `voice_assistant_set_threshold` with the current threshold value.
2. Call `voice_assistant_set_rolling_window` with the current rolling_window value.
3. Call `voice_assistant_set_max_tokens` with the current max_tokens value.
4. Read `voice_assistant://llm` to verify all three parameters were applied correctly.

#### 4c: Clear Conversation

If `clear_between` is `true`:

1. Call `voice_assistant_clear_conversation` to reset the conversation history and KV cache.

#### 4d: Start Training

1. Build a label string: `{model_short}_{test_id}_t{threshold}_rw{rolling_window}_mt{max_tokens}`
    - `model_short`: Model filename without extension, truncated to 20 characters.
2. Call `voice_assistant_training_start` with the label.
3. Record the `trace_id` from the response.

#### 4e: Submit Query

1. Call `voice_assistant_submit_text` with the test case `query` string.

#### 4f: Poll Status

1. Read `voice_assistant://status` resource.
2. Check the `state` field in the JSON response.
3. If `state` is `ThinkingLlm`, `ThinkingStt`, or `Listening`: The pipeline is still active. Wait 3 seconds and poll again. Repeat up to 40 times (maximum 120
   seconds total).
4. If `state` is `Speaking`: The LLM has produced a final answer and TTS is playing. Wait 2 seconds and poll again until `state` is `Idle`. This ensures the
   trace captures the complete ReAct loop including the final answer step. If the state remains `Speaking` for more than 30 seconds (15 polls), proceed to 4g —
   TTS playback may be stuck or the audio device may be unavailable.
5. If `state` is `Idle` or `Error`: Proceed to the next step.
6. If the 120-second timeout is reached: Record a timeout failure and proceed to 4g.

**Critical**: Do NOT call `voice_assistant_training_end` while the state is `ThinkingLlm`. The trace will be finalized before the LLM finishes, causing the
final answer step to be missing from the trace. Always wait until the state transitions to `Idle`.

#### 4g: End Training

1. Call `voice_assistant_training_end` to finalize the trace.
2. Record the `trace_id` from the response.

#### 4h: Retrieve Trace

1. Call `voice_assistant_training_get` with `limit=1`.
2. Parse the JSON response to extract the trace object.
3. If no trace is returned, record a retrieval failure.

#### 4i: Retrieve Launcher Logs

If `capture_logs` is `true`:

1. Call `launcher_get_logs` with the following parameters:
    - `min_level`: the `log_level` parameter value
    - `target_prefix`: the `log_target_prefix` parameter value (if provided)
    - `since_seconds`: the `log_since_seconds` parameter value
    - `limit`: the `log_limit` parameter value
2. Parse the JSON response. Extract `entries` (array of log entries), `total_returned`, `total_in_buffer`, `buffer_capacity`.
3. Filter log entries to those with `timestamp_ms` >= the test case start time (recorded at step 4d).
4. Store the filtered log entries alongside the trace for use in failure analysis (step 4j) and the report (step 5c).
5. Look for log entries with `level` >= `warn` that may indicate errors during the test run (e.g. LLM inference errors, tool invocation failures, parse errors).
6. Check for premature termination: compare the trace's `end_time` (in seconds, multiply by 1000 for ms) against the latest
   `smearor_voice_assistant_service::react` log entry timestamp. If there are `react.rs` log entries with `timestamp_ms` > trace `end_time * 1000`, the trace
   was likely finalized before the ReAct loop completed (premature `training_end`). Record this as a diagnostic finding.

#### 4j: Analyze Trace

Evaluate the trace against the following criteria. Record pass/fail for each:

1. **success**: The trace's `success` field is `true`.
2. **iterations**: `steps.len()` is within `[min_iterations, max_iterations]`.
3. **expected_tools_called**: Every tool in `expected_tools` appears in at least one step's `action` field (format: `tool:{tool_name}`). Skip this check if
   `expected_tools` is empty.
4. **no_generation_loop**: No single step's `thought` length exceeds 50% of `max_tokens` (estimated as character count > max_tokens * 2). Also check for
   repeated identical `thought` content across 2+ consecutive steps.
5. **no_parse_errors**: No step's `action` field contains `parse_error`, `unknown_action`, or `error`.
6. **response_language**: If the trace has a final answer (any step with `answer` is not null), check that the answer text contains characters consistent with
   the expected language. For `de`: check for German-specific words or umlauts. For `en`: check for English words. This is a heuristic check.
7. **response_relevance**: Use your judgment as an AI to assess whether the final answer is relevant and responsive to the query. Record `pass` or `fail` with a
   brief reason.

**Overall result**: `Pass` if all criteria pass. `Fail` if any criterion fails. Record which criteria failed.

**Log-assisted diagnostics**: If `capture_logs` is `true` and the test failed, review the captured log entries for the test case to identify potential root
causes:

- `error` level entries during the test window may indicate crashes or infrastructure failures.
- `warn` level entries from `smearor_voice_assistant` may indicate LLM output sanitization, parse repairs, or discarded JSON.
- `debug` level entries from `smearor_voice_assistant::react` may show the raw LLM output, sanitized output, and plan injection activity.
- `debug` level entries from `smearor_voice_assistant::llm` show session creation, prompt formatting, tokenization, KV-cache shifts, and streaming output —
  useful for diagnosing timeouts and context overflow.
- `debug` level entries from `smearor_voice_assistant::catalog_router` show tool selection scores and threshold filtering — useful for diagnosing
  wrong-tool-selection failures.
- `debug` level entries from `smearor_voice_assistant::tts` show text normalization, phoneme generation, and audio playback — useful for diagnosing TTS-related
  delays.
- Correlate log timestamps with trace step iterations to pinpoint where the failure occurred.
- **Premature termination**: If `react.rs` log entries exist with `timestamp_ms` > trace `end_time * 1000`, the trace was finalized before the ReAct loop
  completed. This indicates `training_end` was called too early (see §4f). The trace may be missing the final answer step even though the LLM produced one.
- **Unknown correlation_id**: Log entries containing `"received tool response for unknown correlation_id"` in `react.rs` indicate that MCP tool responses (e.g.
  `training_end`, `training_get`) arrived after the ReAct loop already moved on. This is a side effect of calling training tools during an active ReAct loop and
  is expected — it does not indicate a failure.

### Step 5: Generate Report

After all test runs are complete, generate a markdown report with the following sections:

#### 5a: Header

```markdown
# Voice Assistant Evaluation Report

**Date**: {current date} **Models tested**: {comma-separated model filenames} **Mode**: {quick or matrix} **Total test runs**: {count}
```

#### 5b: Summary Table (per model)

For each model, generate a table:

```markdown
## Model: {model filename}

| Test ID | T | RW | MT | Result | Iterations | Tools Called | Trace ID | Failure Reason |
|---------|---|----|----|--------|------------|--------------|----------|----------------|
| ...     |   |    |    | ✅/❌  | N          | tool1, tool2 | trace-...| (empty or reason) |
```

- `Tools Called`: Comma-separated list of `tool:{name}` values extracted from all steps' `action` fields.
- `Failure Reason`: If the test failed, a short summary of which criteria failed (e.g. "Generation loop at iteration 3").

#### 5c: Failure Details

For each failed test, generate a detailed section:

```markdown
### Failure: {test_id} ({model short}, T={threshold}, RW={rw}, MT={mt})

**Query**: {query} **Trace ID**: {trace_id} **Failure reason**: {detailed reason}

**Steps**:

1. Iteration {n}: action={action}, params={parameters}, observation={first 200 chars of observation}
2. ...

**Relevant Logs** (if `capture_logs` is `true`):

```

[timestamp] LEVEL target: message fields...

```

Include only log entries from the test case time window with level >= `warn`, plus any `debug` entries from `smearor_voice_assistant::react` that show LLM output sanitization or parse repair activity. Limit to 20 most relevant entries.

**Recommendation**: {suggested fix based on failure pattern and log evidence}
```

Failure pattern recommendations:

- **Generation loop**: "The model enters a generation loop after a tool call. Consider increasing max_tokens, adjusting the system prompt's final-answer hint,
  or trying a different model. Check logs for repeated LLM outputs or plan injection activity."
- **Parse error**: "The model output does not match the expected JSON format. Check the system prompt format instructions. Consider enabling grammar mode if
  available. Review `smearor_voice_assistant::react` debug logs for raw LLM output and sanitization attempts."
- **Timeout**: "The model did not produce a final answer within 120 seconds. Check if the model is too large for the available VRAM, or if the context window is
  too small. Review launcher logs for error-level entries during the test window."
- **Wrong tools**: "The model did not call the expected tools. Check the tool selection threshold, the tool descriptions in the catalog, or the system prompt.
  Review logs for threshold-related warnings or tool selection debug output."
- **Language mismatch**: "The model responded in the wrong language. Check the system prompt language instructions."

#### 5d: Model Comparison (if multiple models tested)

```markdown
## Model Comparison

| Test ID | {Model A short} | {Model B short} | Winner |
|---------|-----------------|-----------------|--------|
| ...     | ✅ N iter / ❌ reason | ✅ N iter / ❌ reason | Model A / Model B / Tie |
| **Overall** | **X/Y passed** | **X/Y passed** | **Winner** |
```

#### 5e: Baseline Comparison (if baseline_path provided)

1. Read the baseline report file.
2. Extract the baseline test results (test IDs and pass/fail status).
3. Generate a regression table:

```markdown
## Regression Check vs Baseline ({baseline date})

| Test ID | Baseline Result | Current Result | Regression? |
|---------|-----------------|----------------|-------------|
| ...     | ✅ Pass          | ✅ Pass        | No          |
| ...     | ✅ Pass          | ❌ Fail (reason) | **Yes**   |
```

#### 5f: Save or Output Report

1. If `report_path` is provided: Write the full report to that file path. Confirm to the user that the report was saved.
2. Otherwise: Output the full report in the chat.

### Step 6: Summary to User

After the report is generated, provide a brief summary to the user:

- Total test runs executed
- Overall pass rate (passed/total)
- Best performing model (if multiple tested)
- Any regressions detected (if baseline provided)
- Path to the saved report (if applicable)

### Important Notes

- **Parameter reset after model switch**: `voice_assistant_switch_model` resets `rolling_window_keep_last` and `threshold` to config defaults. Always reapply
  parameters after a model switch before running test cases.
- **Conversation contamination**: Always call `voice_assistant_clear_conversation` between test cases when `clear_between` is `true`. Previous conversation
  context can affect tool selection and response quality.
- **Polling patience**: LLM inference can take 10-60 seconds depending on model size and query complexity. The 120-second timeout is generous but not infinite.
  Always wait until the state is `Idle` before calling `training_end` — calling it during `ThinkingLlm` will truncate the trace.
- **Trace retrieval**: If `voice_assistant_training_get` returns an empty array, the trace may not have been finalized. Check that
  `voice_assistant_training_end` was called successfully.
- **Model loading time**: Large models (e.g. 7B+ parameters) may take 30+ seconds to load. Be patient when polling `voice_assistant://llm` after a model switch.
- **Tool name format**: In traces, tool calls appear as `tool:{tool_name}` in the `action` field. Resource reads appear as `resource:{uri}`. Final answers
  appear as `final_answer`. Clarifications appear as `clarify`.
- **Log retrieval**: The `launcher_get_logs` tool returns entries from a bounded ring buffer (default capacity 10000). Old entries are evicted. If logs are
  missing for a test case, increase `log_since_seconds` or reduce `log_limit` for other test cases. Use `log_target_prefix` to narrow to a specific module.
  Plugin logs (from `smearor_voice_assistant_service::*` and other service/widget plugins) are forwarded to the host's `LogBuffer` via FFI log forwarding. This
  includes `react.rs` (ReAct loop, JSON parsing, plan injection), `llm.rs` (session management, tokenization, streaming), `catalog_router.rs` (tool selection
  scoring), `tts.rs` (text-to-speech), `audio.rs` (audio capture), and `wake_word.rs` (wake word detection).
- **Log entry format**: Each log entry contains `timestamp_ms` (Unix epoch milliseconds), `level` (string: trace/debug/info/warn/error), `target` (module path),
  `message` (formatted message), `fields` (array of `key=value` strings from structured tracing fields), `file` (optional source file), `line` (optional line
  number).
- **Log correlation**: To correlate logs with a specific test case, record the test case start time (in milliseconds) at step 4d and filter log entries by
  `timestamp_ms >= start_time`. This isolates log entries generated during that test run.
