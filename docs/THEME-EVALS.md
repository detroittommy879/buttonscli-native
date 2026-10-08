# Comparable AI theme trials

Start with automatic checks, then human visual comparison. The first trial is
12 palettes: Halloween, Miami beaches/palms, and cozy autumn/Thanksgiving from
the four exact OpenRouter model IDs in `scripts/theme-eval.py`. No v3 collection
or new app generation UI is implemented here.

- Same brief, reference JSON, actual matching native screenshot and request
  settings for each model. Temperature/reasoning omitted so models use defaults,
  as requested. Four calls at a time; no substitute models.
- Strict current-format JSON, complete colors/16 ANSI entries, no invented keys.
  Main text >=4.5:1, dim text >=3:1, dock/status/terminal/white-hover checks.
  Low ANSI foreground contrast is an advisory; black is intentionally exempt.
- One bounded correction with the actual failure, preserving first-pass results.
  Save raw replies, model/provider IDs, token usage, reported cost and latency.
- Import through the real app into an owned temporary profile, assert that every
  color survives native parsing, then capture identical synthetic terminal text.
  Hash-bind JSON and PNG. Installation requires matching native validation and
  uses exclusive file creation; existing personal themes are never replaced.

The key stays in `OPENROUTER_API_KEY`. Requests contain only the creative brief,
synthetic reference image/JSON, and any correction feedback. Run artifacts belong
under ignored `.private/theme-evals/`; they contain no credential headers.

## Repeat a trial (PowerShell, repository root)

Create the reference capture using the real renderer:

```powershell
New-Item -ItemType Directory '.private/theme-evals/reference' -Force | Out-Null
Copy-Item 'scripts/theme-eval-reference.json' '.private/theme-evals/reference/midnight-signal.json'
$env:BUTTONSCLI_THEME_EVAL_INPUT_DIR = "$PWD/.private/theme-evals/reference"
$env:BUTTONSCLI_NATIVE_PROBE_CAPTURE_DIR = "$PWD/.private/theme-evals/reference-captures"
cargo test --locked --lib native_theme_eval_capture -- --ignored --test-threads=1

# Pick a new directory for each run; an existing run cannot be overwritten.
$themeEvalRun = '.private/theme-evals/trial-001'
python scripts/theme-eval.py run --reference-image '.private/theme-evals/reference-captures/midnight-signal.png' --output $themeEvalRun

$env:BUTTONSCLI_THEME_EVAL_INPUT_DIR = "$PWD/$themeEvalRun/themes"
$env:BUTTONSCLI_NATIVE_PROBE_CAPTURE_DIR = "$PWD/$themeEvalRun/captures"
cargo test --locked --lib native_theme_eval_capture -- --ignored --test-threads=1
python scripts/theme-eval.py install --run $themeEvalRun
```

`--models haiku mimo qwen glm` and `--briefs halloween miami autumn` can select a
subset. Maximum first run: 12 calls plus 12 corrections, each requests a 32,000
token output limit. Providers can bill extra reasoning beyond that limit (seen
with Qwen); measure actual usage/cost. Failed themes stay
in the report and are not installed. Restart the app to reload externally added
files; find **AI Haiku**, **AI Mimo**, **AI Qwen**, **AI Glm** in the Personal library.
Applying App + Terminal gives the shared palette; individual tab overrides still
take priority. Choose This terminal or reset its override when comparing.

`--max-tokens 8000` lowers the spend cap, but default reasoning may exhaust it
before producing JSON. The initial 8k trial hit this for GLM/Qwen; extended runs
use 32k without setting temperature or reasoning. Compare budget changes as
separate trials, not as an identical-parameter model ranking.

## Human evaluation

Compare candidates by the same brief, preferably hiding model identity first.
Score 1–5 for brief fit, UI/button readability, distinct useful ANSI colors and
comfort during a long session. Mark keep/reject with one short reason. Automated
checks catch format and defined contrast failures; human judgment checks taste,
visual hierarchy and semantic colors. A screenshot judge can flag issues later,
but should not replace a user's preference or serve as its own sole evaluator.

This initial run has one sample per model/brief. It is exploratory, not a reliable
model ranking. Before choosing an app default, repeat with more briefs and several
samples, include light/dark and adverse layouts, compare first-pass validity,
native import/render success, human acceptance, latency and cost. Freeze the
briefs/prompt/reference and keep failures in the denominator. Use separate briefs
for tuning and the final comparison. Fonts, gradients/effects and mixed-tab
collections need their own native fixtures and eval cases later.

Initial run, 2026-10-07: installed 16 passing palettes (Haiku/MiMo 3 each,
Qwen/GLM 5 each including variants) from defaults-only 8k and 32k trials.
All 16 passed native import/color round-trip and screenshot capture. Failures
remain in their reports. The combined local review is
`.private/theme-evals/review.md`; the two defaults trials reported about $0.20,
excluding earlier calibration/interrupted requests. This is a small palette
experiment, not evidence of a universally best model.

Protocol: [OpenRouter image inputs](https://openrouter.ai/docs/guides/overview/multimodal/image-understanding)
and [chat completions](https://openrouter.ai/docs/api/api-reference/chat/create-a-chat-completion).
Offline failure checks: `python scripts/test-theme-eval.py`.
