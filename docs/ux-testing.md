# Testing the developer experience

The unit of acceptance is a developer completing a task from the public TUI.
A model saying “done”, a successful tool call, or a screenshot alone does not
establish completion. Keep this checklist current when changing the workflow.

## Run the required journeys

From the repository root, after building the binary:

```bash
cargo build -p meden-cli
MEDEN_BIN="$PWD/target/debug/meden" uv run --project python --group dev \
  python -m meden_checks.ux --out /tmp/meden-ux-review
```

Use a new output directory for each run. Open `/tmp/meden-ux-review/index.html`
to select a journey and move through its decoded terminal screens. `summary.json`
and `junit.xml` contain the verdicts; the summary identifies the binary by path
and SHA-256; individual directories retain timestamped
`screens.jsonl` and `last-screen.txt`. Screen records are sampled after PTY reads,
not a recording of every paint. Per-test traces are capped at 10 MB and indicate
truncation in `trace.json`; the final screen is retained separately.

The command needs the Python dev dependencies, Linux, Bubblewrap and the host
public toolchain used by the existing integration tests. A missing prerequisite,
skipped journey, failed assertion or empty selection fails the required gate.
Fixtures use temporary projects, synthetic replies and an inert checker manifest;
they never need a paid model or prove a real cold checker installation.

CI runs these same marked cases once within the existing Python adapter job.
It uploads `meden-ux-evidence`, including the HTML viewer, on success or failure.
Other platform-dependent tests may still skip; required `ux` cases must pass.

## Experience contract

| Developer goal | Observable acceptance | Executable coverage in `python/tests/` |
| --- | --- | --- |
| Start without connection flags | Run `meden` or `meden tui --workspace …`, configure in the TUI, see inline errors, keep the key masked and choose whether to save it | `test_tui_setup.py` |
| Work in the chosen repository | Files change in that repository; commands run there; original tests remain intact | `test_tui_integration.py`, `test_tui_smoke.py` |
| Reopen and continue | Files, connection preferences and conversation persist; HTTP requires a fresh review and no old grant returns | `test_tui_setup.py`, `test_tui_smoke.py`, `test_workspace_resume.py` |
| Understand silent waiting | Before the first response, activity and elapsed time stay visible and the spinner advances | `test_streaming.py` (server held behind an event) |
| Read a response as it arrives | Partial text appears before response completion; command output appears while the command is running | `test_streaming.py` |
| Recover from a tool failure | A completed failing command is explained and repaired under the existing budget; the repaired file really exists | `test_tui_recovery.py` |
| Understand a terminal stop | Provider limits or incomplete streams explain the situation and next action; work survives restart; `/retry` is an explicit new turn | `test_tui_recovery.py`, `test_streaming.py` |
| Navigate without losing work | Wheel/page scrolling moves transcript content, preserves drafts, separates input history and restores the terminal on exit | `test_tui_keys.py` |
| Trust the completion claim | Independent original tests run in a separate offline `meden exec` copy; intentionally broken generated code makes the smoke fail | `test_tui_smoke.py` |
| Distinguish unproven and verified work | Native contract review, actual Rust/Lean checks, stale source detection and restart without stale coverage | `test_project_verification.py`, in the separate Lean CI job with a real bundle |

The fast gate does not cover large-model quality, verifier bootstrap from an empty
machine, arbitrary new user journeys, subjective layout quality, or complete
renderer authenticity. Real checker tests and the proof scope remain documented
in [verification.md](verification.md) and [project-verification.md](project-verification.md).

## Completion criteria for changes

1. Name the affected goal and observable outcome before editing. Use the README's
   public entry command and the intended starting state, including an empty
   configuration when changing onboarding.
2. Exercise the actual PTY and resulting files. For changed asynchronous behavior,
   hold the server/tool behind an event and assert the intermediate screen before
   releasing it. Include the relevant failure and restart path. Prefer outcomes
   and visible states to raw ANSI history or exact whole-screen snapshots.
3. Run the required journeys and inspect the relevant screens in `index.html`,
   including the failure/stop screen. Do not repair the test project or configuration
   outside the TUI to make a journey pass. Explicit test fixtures are prerequisites,
   not evidence that real installation succeeded.
4. Add regressions for new user-reported friction. Update this table and the README
   when entry commands, controls or expectations change. Report what passed and
   what was not exercised; a skipped case is not completed validation.

## Exploratory review before declaring a new flow usable

Use a fresh temporary project with synthetic data. Give an evaluator (human or
AI) only a goal, the README entry command and the visible TUI, for example:
“Start a project, implement a small calculation, close the app and continue it.”
For a new flow, also try a provider error, a long silent response and a small
terminal. Do not supply internal key sequences, edit private configuration or
read source to rescue the evaluator. Record each place where progress requires
guessing, an unexplained stop, an extra command, lost work or ambiguous proof
status. Save the screens and turn reproducible problems into deterministic tests.

An AI implementation agent should do this screen review as part of its completion
criteria. This is a review procedure, not a separate autonomous evaluator service;
the current automation cannot judge taste or discover every unknown workflow.
The developer should need to review new choices, rather than repeat old regressions.

## Live-model smoke

For changes to prompting, tool use or multi-turn behavior, and before a release,
run the separate driver with an explicitly selected capable Ollama Cloud provider
already configured on the host:

```bash
uv run --project python --group dev python -m meden_checks.tui_smoke \
  --binary target/release/meden --provider YOUR_PROVIDER --model YOUR_MODEL \
  --out /tmp/meden-live-review.json
```

This approves the chosen processor through its real review screen for a synthetic
project, fixes a program, restarts the TUI with fresh review, adds a feature using
the retained conversation and checks the result independently in an offline copy.
It saves source, original-test preservation, exit status and milestone screens.
CI tests the driver against a scripted provider, including a lying completion
with broken code. That protects the driver from stale controls; it is not live
inference evidence. A real cloud run is not local-model or confidentiality evidence.
No live requests or model performance claims are part of the fast gate.
