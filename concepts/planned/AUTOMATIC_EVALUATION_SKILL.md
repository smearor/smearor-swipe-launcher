# Concept: Automatic Evaluation Skill for Cascade

This document describes a **Cascade Skill** (workflow file) that automates Voice Assistant ReAct loop evaluation. The skill is loaded into Cascade (or any
MCP-compatible AI agent) and uses the existing Voice Assistant MCP tools to run modular test suites, compare models, and produce structured evaluation reports.

---

## 1. Motivation

### Current State

The Voice Assistant Service exposes a comprehensive set of MCP tools and resources:

- **Training Mode**: `voice_assistant_training_start`, `voice_assistant_training_end`, `voice_assistant_training_get` — records ReAct loop traces with
  per-iteration `thought`, `action`, `parameters`, `observation`, and `answer`.
- **Model Management**: `voice_assistant_switch_model`, `voice_assistant://models` resource — lists available GGUF models with metadata and switches at runtime.
- **Parameter Tuning**: `voice_assistant_set_threshold`, `voice_assistant_set_rolling_window`, `voice_assistant_set_max_tokens` — runtime parameter changes
  without reload.
- **Conversation Control**: `voice_assistant_submit_text`, `voice_assistant_clear_conversation` — inject queries and reset context between tests.
- **System Prompt**: `voice_assistant_get_system_prompt`, `voice_assistant_set_system_prompt` — inspect and override the system prompt.
- **Status Inspection**: `voice_assistant://status`, `voice_assistant://llm`, `voice_assistant://tool_catalog` — observe state, configuration, and available
  tools.

The previous manual test procedure (`deprecated/TEST_PROCEDURE_1.md`) demonstrated the value of systematic parameter-matrix testing but was limited to a single
query ("Wie ist das Wetter in Berlin?") and required manual execution of every step.

### Problem

- **No reusable evaluation**: Each test run requires manually calling 8+ MCP tools in sequence for every test case.
- **No model comparison automation**: Switching models, reconfiguring parameters, and comparing results across models is tedious and error-prone.
- **No modular test cases**: The manual procedure hardcoded a single weather query. Different capabilities (time, app-launch, area-config, memory) need separate
  test cases.
- **No structured reporting**: Results are captured ad-hoc in markdown files with no consistent schema.
- **No regression detection**: There is no baseline to compare against after code changes.

### Required Capabilities

| Capability                  | Example                                           | Solution                                      |
|-----------------------------|---------------------------------------------------|-----------------------------------------------|
| Run a single test case      | "Test weather query with current model"           | Modular test case definition + execution flow |
| Run a parameter matrix      | "Test all threshold × rolling-window combos"      | Matrix expansion in the skill                 |
| Compare models              | "Compare Qwen vs Gemma on all test cases"         | Model iteration + per-model test suite        |
| Discover available models   | "Which models are available?"                     | Read `voice_assistant://models` resource      |
| Structured result reporting | "Give me a summary table of all tests"            | Trace analysis + markdown report generation   |
| Regression detection        | "Did the last code change break weather queries?" | Baseline comparison mode                      |

---

## 2. Skill Architecture

### 2.1 What is a Cascade Skill?

A Cascade Skill is a **workflow file** (`.devin/workflows/automatic_evaluation.md` or `.windsurf/workflows/automatic_evaluation.md`) with YAML frontmatter and
markdown instructions. When loaded into Cascade, it provides the AI agent with a structured procedure to follow using the available MCP tools.

The skill is **not a Rust crate** — it requires no code changes to the launcher. It is a pure instruction document that leverages existing MCP tools and
resources.

### 2.2 High-Level Flow

```
┌──────────────────────────────────────────────────────────────────────┐
│                     Cascade AI Agent (Skill)                         │
│                                                                      │
│  1. Discover available models (voice_assistant://models)             │
│  2. For each model:                                                  │
│     a. Switch model (voice_assistant_switch_model)                   │
│     b. Wait for model to load (poll voice_assistant://llm)           │
│     c. For each test case:                                           │
│        i.   Clear conversation (voice_assistant_clear_conversation)  │
│        ii.  Set parameters (threshold, rolling_window, max_tokens)   │
│        iii. Start training (voice_assistant_training_start)          │
│        iv.  Submit query (voice_assistant_submit_text)               │
│        v.   Poll status (voice_assistant://status) until Idle/Error  │
│        vi.  End training (voice_assistant_training_end)              │
│        vii. Retrieve trace (voice_assistant_training_get)            │
│        viii. Analyze trace (success, steps, errors, loops)           │
│  3. Generate structured report (markdown table + per-test details)   │
│  4. Optionally compare against baseline                              │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

### 2.3 MCP Tools & Resources Used

| Tool / Resource                      | Type     | Purpose                                     |
|--------------------------------------|----------|---------------------------------------------|
| `voice_assistant://models`           | Resource | List available GGUF models + metadata       |
| `voice_assistant://llm`              | Resource | Current model path, parameters, tool calls  |
| `voice_assistant://status`           | Resource | Current state, transcript, final answer     |
| `voice_assistant://tool_catalog`     | Resource | List of registered tools (for test design)  |
| `voice_assistant_switch_model`       | Tool     | Switch to a specific model                  |
| `voice_assistant_set_threshold`      | Tool     | Set tool selection threshold (0.0–1.0)      |
| `voice_assistant_set_rolling_window` | Tool     | Set rolling window keep_last                |
| `voice_assistant_set_max_tokens`     | Tool     | Set max generation tokens                   |
| `voice_assistant_clear_conversation` | Tool     | Reset conversation between test cases       |
| `voice_assistant_training_start`     | Tool     | Begin trace recording                       |
| `voice_assistant_training_end`       | Tool     | Finalize trace                              |
| `voice_assistant_training_get`       | Tool     | Retrieve recorded trace(s)                  |
| `voice_assistant_submit_text`        | Tool     | Inject test query                           |
| `voice_assistant_get_system_prompt`  | Tool     | Inspect current system prompt               |
| `voice_assistant_set_system_prompt`  | Tool     | Override system prompt (for prompt testing) |

---

## 3. Test Case Module System

### 3.1 Test Case Structure

Each test case is a self-contained module with the following fields:

| Field            | Type           | Description                                                      |
|------------------|----------------|------------------------------------------------------------------|
| `id`             | `String`       | Unique identifier (e.g. `weather_query`)                         |
| `query`          | `String`       | The text submitted to the voice assistant                        |
| `label_prefix`   | `String`       | Prefix for training trace labels (e.g. `weather`)                |
| `expected_tools` | `Vec<String>`  | Tools expected to be called (e.g. `["weather_get_forecast"]`)    |
| `min_iterations` | `usize`        | Minimum expected ReAct iterations (1 = direct answer)            |
| `max_iterations` | `usize`        | Maximum expected ReAct iterations before declaring a loop        |
| `language`       | `String`       | Expected response language (`de`, `en`, etc.)                    |
| `description`    | `String`       | Human-readable description of what the test validates            |
| `category`       | `TestCategory` | Category tag for grouping (`Weather`, `Time`, `AppLaunch`, etc.) |

### 3.2 Built-in Test Cases

The skill defines a catalog of test cases covering different capabilities:

#### Weather

| ID                  | Query                                    | Expected Tools                                         | Description                             |
|---------------------|------------------------------------------|--------------------------------------------------------|-----------------------------------------|
| `weather_by_city`   | `"Wie ist das Wetter heute in Bregenz?"` | `weather_lookup_coordinates`, `weather_get_forecast`   | Weather query with city name            |
| `weather_by_coords` | `"What's the weather at 47.5, 9.7?"`     | `weather_get_forecast`                                 | Weather query with coordinates          |
| `weather_generic`   | `"Wie ist das Wetter?"`                  | `weather_lookup_coordinates` or `weather_get_forecast` | Weather query without specific location |

#### Time

| ID              | Query                         | Expected Tools     | Description                  |
|-----------------|-------------------------------|--------------------|------------------------------|
| `time_current`  | `"Wie spät ist es?"`          | `get_current_time` | Current time query in German |
| `time_timezone` | `"What time is it in Tokyo?"` | `get_current_time` | Time query with timezone     |

#### App Launch

| ID                | Query                             | Expected Tools        | Description         |
|-------------------|-----------------------------------|-----------------------|---------------------|
| `app_launch_name` | `"Öffne Firefox"`                 | `app_launcher_launch` | Launch app by name  |
| `app_launch_list` | `"Welche Apps sind installiert?"` | `app_launcher_list`   | List installed apps |

#### Area Config

| ID                  | Query                                | Expected Tools    | Description              |
|---------------------|--------------------------------------|-------------------|--------------------------|
| `area_config_query` | `"Welche Widgets sind im Macropad?"` | `get_area_config` | Query area configuration |

#### Memory

| ID              | Query                                    | Expected Tools  | Description                     |
|-----------------|------------------------------------------|-----------------|---------------------------------|
| `memory_store`  | `"Merke dir, dass mein Lieblingslied...` | `memory_store`  | Store a fact in semantic memory |
| `memory_recall` | `"Was weiß du über mich?"`               | `memory_recall` | Recall stored facts             |

#### Clarification

| ID                  | Query            | Expected Tools | Description                               |
|---------------------|------------------|----------------|-------------------------------------------|
| `clarify_ambiguous` | `"Spiele Musik"` | (varies)       | Ambiguous request, expect clarify or tool |

### 3.3 Custom Test Cases

The skill supports adding custom test cases at runtime. The user can ask Cascade to run a specific query:

> "Test the query 'Stelle die Lautstärke auf 50%' with the current model."

Cascade will construct a test case from the query, run it through the standard flow, and analyze the trace.

### 3.4 Test Case Selection

The skill accepts parameters to select which test cases to run:

| Parameter      | Default   | Description                                           |
|----------------|-----------|-------------------------------------------------------|
| `categories`   | `all`     | Comma-separated categories (e.g. `Weather,Time`)      |
| `test_ids`     | `all`     | Comma-separated test IDs                              |
| `custom_query` | (none)    | A custom query string to test                         |
| `models`       | `current` | Comma-separated model filenames or `all` or `current` |

---

## 4. Parameter Matrix

### 4.1 Default Matrix

For each test case, the skill can run a parameter matrix to evaluate how different settings affect the result:

| Parameter        | Default Values | Description              |
|------------------|----------------|--------------------------|
| `threshold`      | `[0.3, 0.5]`   | Tool selection threshold |
| `rolling_window` | `[4, 6]`       | Rolling window keep_last |
| `max_tokens`     | `[512, 1024]`  | Max generation tokens    |

### 4.2 Matrix Expansion

The full matrix for one test case with one model is:

```
threshold × rolling_window × max_tokens
2 × 2 × 2 = 8 runs per test case per model
```

The skill supports a `quick` mode that uses only the default parameter set (no matrix):

```
threshold=0.3, rolling_window=6, max_tokens=1024
1 run per test case per model
```

### 4.3 Post-Model-Switch Reconfiguration

**Critical**: After `voice_assistant_switch_model`, the rolling window and threshold are reset to config defaults. The skill must reapply the desired parameters
after each model switch before running test cases.

---

## 5. Execution Flow

### 5.1 Per-Test-Case Flow

For each test case × parameter combination × model:

1. **Clear conversation**: Call `voice_assistant_clear_conversation` to prevent context contamination.
2. **Set parameters**: Call `voice_assistant_set_threshold`, `voice_assistant_set_rolling_window`, `voice_assistant_set_max_tokens` with the current matrix
   values.
3. **Verify parameters**: Read `voice_assistant://llm` to confirm parameters were applied.
4. **Start training**: Call `voice_assistant_training_start` with label `"{model_short}_{test_id}_{t}{threshold}_rw{rolling_window}_mt{max_tokens}"`.
5. **Submit query**: Call `voice_assistant_submit_text` with the test case query.
6. **Poll status**: Read `voice_assistant://status` repeatedly until `state` is not `ThinkingLlm` (i.e. `Idle`, `Error`, or `Speaking`). Wait at least 2 seconds
   between polls. Maximum wait: 120 seconds.
7. **End training**: Call `voice_assistant_training_end` to finalize the trace.
8. **Retrieve trace**: Call `voice_assistant_training_get` with `limit=1` to get the trace.
9. **Analyze trace**: Evaluate success, iteration count, tool calls, errors, and response quality.

### 5.2 Trace Analysis Criteria

For each trace, the skill evaluates:

| Criterion               | Pass Condition                                             |
|-------------------------|------------------------------------------------------------|
| `success`               | Trace `success` field is `true`                            |
| `iterations`            | `steps.len()` is within `[min_iterations, max_iterations]` |
| `expected_tools_called` | All `expected_tools` appear in `steps[].action`            |
| `no_generation_loop`    | No single `thought` exceeds 50% of `max_tokens`            |
| `no_parse_errors`       | No `action` field contains `parse_error` or `unknown`      |
| `response_language`     | Final answer language matches `language` field             |
| `response_relevance`    | Final answer is relevant to the query (LLM-judged)         |

### 5.3 Model Discovery and Selection

1. Read `voice_assistant://models` resource.
2. Parse the `available_models` array from the JSON response.
3. Each model entry contains: `filename`, `path`, `size_mb`, `metadata` (architecture, context_length, etc.).
4. Filter models by the `models` parameter:
    - `current`: Use only the currently loaded model (from `current_model` field).
    - `all`: Test all available models.
    - `model1.gguf,model2.gguf`: Test specific models.
5. For each selected model:
   a. Call `voice_assistant_switch_model` with `model_path`. b. Poll `voice_assistant://llm` until `model_path` matches the requested model. c. Wait 5 seconds
   after model match for KV cache initialization. d. Reapply parameter settings (threshold, rolling_window, max_tokens).

---

## 6. Report Generation

### 6.1 Summary Table

After all tests complete, the skill generates a markdown summary table:

```markdown
## Evaluation Results — 2026-08-08

### Model: gemma-4-E4B-it-heretic-ara.Q4_K_M.gguf

| Test ID           | T    | RW  | MT   | Result   | Iterations | Tools Called                    | Trace ID                    |
|-------------------|------|-----|------|----------|------------|---------------------------------|----------------------------|
| weather_by_city   | 0.3  | 6   | 1024 | ✅ Pass  | 3          | weather_lookup_coordinates, weather_get_forecast | trace-2026-08-08T...-abc123 |
| weather_by_city   | 0.5  | 6   | 1024 | ✅ Pass  | 2          | weather_get_forecast            | trace-2026-08-08T...-def456 |
| time_current      | 0.3  | 6   | 1024 | ✅ Pass  | 1          | get_current_time                | trace-2026-08-08T...-789abc |
| app_launch_name   | 0.3  | 6   | 1024 | ❌ Fail  | 4          | (loop)                          | trace-2026-08-08T...-fail01 |
```

### 6.2 Per-Test Detail

For each failed test, the skill generates a detailed analysis:

```markdown
### Failure: app_launch_name (T=0.3, RW=6, MT=1024)

**Query**: "Öffne Firefox"
**Trace ID**: trace-2026-08-08T15-30-00Z-fail01 **Failure reason**: Generation loop — iteration 2 thought exceeds 512 tokens (50% of max_tokens)

**Steps**:

1. Iteration 0: action=`tool:app_launcher_list`, params=`{}`, observation=`[...]`
2. Iteration 1: action=`tool:app_launcher_launch`, params=`{"name":"firefox"}`, observation=`App launched`
3. Iteration 2: action=`parse_error`, thought=`HINT: The tool result likely...` (loop detected)

**Recommendation**: The model enters a generation loop after successful tool call. Consider increasing max_tokens or adjusting the system prompt's final-answer
hint.
```

### 6.3 Model Comparison

When multiple models are tested, the skill generates a comparison table:

```markdown
### Model Comparison

| Test ID           | Model A (Gemma)     | Model B (Qwen)      | Winner  |
|-------------------|---------------------|---------------------|---------|
| weather_by_city   | ✅ 3 iterations     | ❌ Generation loop  | Model A |
| time_current      | ✅ 1 iteration      | ✅ 1 iteration      | Tie     |
| app_launch_name   | ❌ Generation loop  | ✅ 2 iterations     | Model B |
| **Overall**       | **2/3 passed**     | **2/3 passed**      | **Tie** |
```

### 6.4 Baseline Comparison

If a previous evaluation result is available (stored as a markdown file), the skill can compare:

```markdown
### Regression Check vs Baseline (2026-07-15)

| Test ID           | Baseline Result   | Current Result    | Regression? |
|-------------------|-------------------|-------------------|-------------|
| weather_by_city   | ✅ Pass            | ✅ Pass            | No          |
| time_current      | ✅ Pass            | ❌ Fail (loop)     | **Yes**     |
```

---

## 7. Skill Parameters

The skill accepts the following parameters from the user:

| Parameter         | Type     | Default    | Description                                            |
|-------------------|----------|------------|--------------------------------------------------------|
| `models`          | `String` | `current`  | `current`, `all`, or comma-separated model filenames   |
| `categories`      | `String` | `all`      | Comma-separated test categories                        |
| `test_ids`        | `String` | `all`      | Comma-separated test IDs                               |
| `custom_query`    | `String` | (none)     | Custom query to test                                   |
| `matrix`          | `Bool`   | `false`    | Run full parameter matrix (true) or quick mode (false) |
| `thresholds`      | `String` | `0.3,0.5`  | Comma-separated threshold values (matrix mode)         |
| `rolling_windows` | `String` | `4,6`      | Comma-separated rolling window values (matrix mode)    |
| `max_tokens_list` | `String` | `512,1024` | Comma-separated max_tokens values (matrix mode)        |
| `report_path`     | `String` | (console)  | File path to save the report                           |
| `baseline_path`   | `String` | (none)     | Path to a previous report for regression comparison    |
| `clear_between`   | `Bool`   | `true`     | Clear conversation between test cases                  |

---

## 8. Workflow File

### 8.1 File Location

The workflow file is created at `.devin/workflows/automatic_evaluation.md` (or `.windsurf/workflows/automatic_evaluation.md`).

### 8.2 Workflow Structure

```yaml
---
description: Automatic Voice Assistant Evaluation — run modular test suites, compare models, generate reports
---

## Automatic Evaluation Skill

### Prerequisites
- Smearor Swipe Launcher is running with Voice Assistant Service loaded
- MCP Server is connected and voice assistant tools are available
- At least one GGUF model is available in the models directory

### Step 1: Discover Available Models
  Read the `voice_assistant://models` resource to get a list of available models.
  Parse the JSON response to extract `current_model` and `available_models[]`.
  Filter models based on the `models` parameter.

  ### Step 2: For Each Selected Model
  Call `voice_assistant_switch_model` with the model path.
  Poll `voice_assistant://llm` until `model_path` matches the requested model.
  Wait 5 seconds for KV cache initialization.
  Reapply parameter settings (threshold, rolling_window, max_tokens) if in matrix mode.

### Step 3: For Each Test Case
  [ ... detailed steps as described in Section 5.1 ... ]

### Step 4: Analyze Traces
  [ ... analysis criteria as described in Section 5.2 ... ]

### Step 5: Generate Report
  [ ... report format as described in Section 6 ... ]

### Step 6: Baseline Comparison (optional)
  [ ... if baseline_path is provided ... ]
```

### 8.3 Skill Invocation Examples

The user can invoke the skill with natural language:

- **"Run automatic evaluation with all models on weather tests"**
  → `models=all, categories=Weather`

- **"Quick test the current model with a custom query: 'Stelle einen Wecker für 7 Uhr'"**
  → `models=current, custom_query="Stelle einen Wecker für 7 Uhr", matrix=false`

- **"Run full matrix evaluation on the current model"**
  → `models=current, matrix=true`

- **"Compare all models on all test cases and save the report"**
  → `models=all, categories=all, report_path=eval_results/2026-08-08.md`

- **"Check for regressions against the last baseline"**
  → `models=current, baseline_path=eval_results/2026-07-15.md`

---

## 9. Implementation Phases

### Phase 1: Create Workflow File

**Order:** First — no dependencies.

**Tasks:**

- Create `.devin/workflows/automatic_evaluation.md` (or `.windsurf/workflows/automatic_evaluation.md`)
- Define the full workflow procedure with all steps from Section 5
- Include the test case catalog from Section 3.2
- Include the parameter matrix expansion logic from Section 4
- Include the report generation templates from Section 6
- Include the model discovery and selection logic from Section 5.3

**Exit Criteria:** The workflow file exists and Cascade can load it as a skill.

### Phase 2: Validate with Current Model

**Order:** Second — depends on Phase 1.

**Tasks:**

- Load the skill in Cascade
- Run a quick evaluation (single model, no matrix) on 2-3 test cases
- Verify that traces are correctly recorded and retrieved
- Verify that the report format is correct

**Exit Criteria:** At least one successful evaluation run produces a valid report.

### Phase 3: Multi-Model Evaluation

**Order:** Third — depends on Phase 2.

**Tasks:**

- Run evaluation with `models=all`
- Verify model switching works mid-evaluation
- Verify parameters are reapplied after model switch
- Verify the model comparison table is generated correctly

**Exit Criteria:** At least two models are evaluated and compared in a single report.

### Phase 4: Matrix Evaluation

**Order:** Fourth — depends on Phase 3.

**Tasks:**

- Run evaluation with `matrix=true`
- Verify parameter matrix is correctly expanded
- Verify all parameter combinations are tested
- Verify the report includes all matrix combinations

**Exit Criteria:** A full matrix run completes for at least one test case across one model.

### Phase 5: Baseline Comparison

**Order:** Fifth — depends on Phase 4.

**Tasks:**

- Save a baseline report from a full evaluation run
- Make a code change (e.g. system prompt adjustment)
- Run evaluation again with `baseline_path` pointing to the baseline
- Verify regression table is generated correctly

**Exit Criteria:** A regression comparison report is generated showing pass/fail changes.

### Phase 6: Custom Test Cases

**Order:** Sixth — depends on Phase 5.

**Tasks:**

- Test the `custom_query` parameter with various queries
- Verify the skill constructs a valid test case from a custom query
- Verify the trace analysis works for unexpected tool calls

**Exit Criteria:** At least 3 custom queries are tested and produce valid analysis.

---

## 10. Dependencies

No new crate dependencies. The skill relies entirely on existing MCP tools and resources.

| Dependency                 | Type     | Status         |
|----------------------------|----------|----------------|
| Voice Assistant Service    | Service  | ✅ Implemented |
| MCP Server                 | Service  | ✅ Implemented |
| Training Mode              | Feature  | ✅ Implemented |
| Model Switching            | Feature  | ✅ Implemented |
| Parameter Tuning Tools     | Feature  | ✅ Implemented |
| `voice_assistant://models` | Resource | ✅ Implemented |
| Cascade / Windsurf         | Agent    | Required       |

---

## 11. Testing & Verification

### 11.1 Skill Loading

- [ ] Workflow file is recognized by Cascade as a valid skill
- [ ] Skill description appears in the workflow list
- [ ] Skill can be invoked via natural language

### 11.2 Single Test Case

- [ ] Weather query produces a trace with correct tool calls
- [ ] Time query produces a trace with `get_current_time`
- [ ] App launch query produces a trace with `app_launcher_launch`
- [ ] Clarification query produces a trace with `clarify` or appropriate tool

### 11.3 Parameter Matrix

- [ ] All threshold values are applied and verified via `voice_assistant://llm`
- [ ] All rolling window values are applied and verified
- [ ] All max_tokens values are applied and verified
- [ ] Matrix expansion produces the correct number of runs

### 11.4 Model Comparison

- [ ] Model switch is detected by polling `voice_assistant://llm`
- [ ] Parameters are reapplied after model switch
- [ ] Comparison table shows correct pass/fail counts
- [ ] Per-model results are correctly attributed

### 11.5 Report Generation

- [ ] Summary table includes all test cases and parameter combinations
- [ ] Failed tests include detailed step-by-step analysis
- [ ] Model comparison table is generated when multiple models are tested
- [ ] Report can be saved to a file path

### 11.6 Edge Cases

- [ ] Graceful handling when a model fails to load
- [ ] Graceful handling when a test case times out (120s)
- [ ] Graceful handling when no models are available
- [ ] Graceful handling when training mode is already active
- [ ] Conversation is cleared between test cases (no context contamination)
- [ ] Generation loops are detected and reported

---

## 12. Future Enhancements

- **Automated prompt optimization**: After identifying failure patterns, the skill could propose system prompt adjustments and re-run affected test cases.
- **JSONL export**: Export all traces as JSONL for external fine-tuning pipelines.
- **Trace diff visualization**: Side-by-side comparison of two traces for the same query.
- **Performance metrics**: Track and report inference time per iteration (from trace timestamps).
- **Tool catalog awareness**: Automatically discover available tools from `voice_assistant://tool_catalog` and generate test cases for each.
- **Multi-language test cases**: Test the same query in different languages to validate multilingual support.
- **Memory integration**: Test memory store/recall workflows with multi-turn conversations.
- **Wake word testing**: Validate wake word detection and activation flow.
- **Scheduled evaluations**: Run the skill automatically after code changes (CI integration).
- **A/B prompt testing**: Compare two system prompts on the same test suite.
