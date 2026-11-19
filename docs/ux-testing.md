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

The command needs the Python dev dependencies, Linux, Bubblewrap, an actual
installed Lean/Lake distribution (`MEDEN_ATLAS_LEAN`) and the host public toolchain used by the existing integration tests. A missing prerequisite,
skipped journey, failed assertion or empty selection fails the required gate.
Fixtures use temporary projects, synthetic replies and an inert project-checker
manifest. The required Lake journey additionally uses the actual installed
Lean/Lake package loader and compiler. These are installed prerequisites, not
paid-model or cold-install validation. CI downloads the same pinned public Lean
distribution used by the native GUI journeys.

CI runs these same marked cases once within the existing Python adapter job.
The public runner starts its test process in a fresh session so each journey
uses its configured PTY size instead of inheriting the launching terminal.
`test_ux_gate.py` also exercises that runner inside a real 80×24 controlling
terminal; its inner journeys retain their own sizes and decoded evidence.
It uploads `meden-ux-evidence`, including the HTML viewer, on success or failure.
Other platform-dependent tests may still skip; required `ux` cases must pass.

## Experience contract

| Developer goal | Observable acceptance | Executable coverage in `python/tests/` |
| --- | --- | --- |
| Start without connection flags | Run `meden` or `meden tui --workspace …`, configure in the TUI, see inline errors, keep the key masked and choose whether to save it | `test_tui_setup.py` |
| Work in the chosen repository | Files change in that repository; commands run there; original tests remain intact | `test_tui_integration.py`, `test_tui_smoke.py` |
| Continue across GUI/TUI | A native GUI turn, a public TUI continuation and a reopened GUI retain the same files/conversation, with fresh processor review | `test_gui_tui_resume.py` |
| Repair a candidate Finding | Reject a diff without edits, refuse stale sources and clipped headings, apply once, reanalyze the original requirement, retain source artifacts/history and restart without an old grant | `test_tui_repair.py`; native browser repair cases |
| Read Lake projects | Run `/atlas` without enumerating files, follow package/library source roots and cached dependencies, refuse stale configuration and changed glob inventories, recover and restart with fresh approval | `test_tui_lake.py`, actual installed Lean/Lake in the required gate; native Lake browser case |
| Collect local Lean imports | Select one root with `/atlas App.lean`, collect transitive local modules, ignore comment text, stop on missing imports, refuse an older graph and recover by selecting a valid root | `test_tui_atlas.py`, separate installed-Lean job; native browser Atlas case |
| Compare through a separate model | Review both fixed model/destination bindings, refuse clipped review, stop on judge failure without fallback, retain translation/provenance and require fresh approval after restart | `test_tui_judge.py`; native separate-judge browser case |
| Inspect candidate meaning from the TUI | Read selected sources or Atlas declarations, run blind translation and comparison, stop between passes, recover from provider failure, reject concurrent edits, retain failure and Finding history, select a past assessment after restart without restoring a grant | `test_tui_analysis.py` |
| Reopen and continue | Files, connection preferences and conversation persist; HTTP requires a fresh review and no old grant returns | `test_tui_setup.py`, `test_tui_smoke.py`, `test_workspace_resume.py` |
| Understand silent waiting | Before the first response, activity and elapsed time stay visible and the spinner advances | `test_streaming.py` (server held behind an event) |
| Read a response as it arrives | Partial text appears before response completion; command output appears while the command is running | `test_streaming.py` |
| Recover from a tool failure | A completed failing command is explained and repaired under the existing budget; scripts shared across Bash calls survive in the workspace while `/tmp` remains per-invocation | `test_tui_recovery.py` |
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

The separate reviewed-repair smoke uses the public TUI, preserves the original
requirement and runs independent Rust boundary tests in an offline copy:

```bash
uv run --project python --group dev python -m meden_checks.meaning_smoke \
  --binary target/release/meden --provider YOUR_PROVIDER --model YOUR_MODEL \
  --verifier-bundle /absolute/path/to/installed/bundle.json \
  --out /tmp/meden-live-repair.json
```

The bundle is an explicit installed prerequisite. This driver creates an empty
conversation/analysis store; it does not count that as a cold verifier install.
It records real model assessments separately from independent program behavior.
`test_meaning_smoke.py` exercises the driver with long synthetic explanations,
including a model that claims a match after writing the wrong boundary. The
actual Rust check must still reject that run. TUI host outcomes remain at the
end of long analysis/repair messages so they cannot scroll out of the completed
viewport. Scripted replies are not live quality evidence.

## GUI journeys

The TypeScript/Vite GUI has its own public browser gate: `npm run test:gui`.
It covers instruction/send/automatic titles, separate searchable conversations,
desktop and mobile home logo layout, independent conversation and artifact
sidebar controls, conversation artifact preview and proof status after restart,
original meden-demo messages and 25 tool inputs/results, unsent input/restart,
preview purchase/refund, three-tab proof-graph/statement/source navigation, stable node targeting in both graph views,
meaning-progress cancellation, the ¥150 NG boundary and matching examples and the algorithm info popover,
continuous consolidation of legacy v1/v2 demo conversations and selection/input restore,
exact supplied opening text, expanded graph/source independent scrolling,
requirement-change freshness, local
review export, invalid import recovery and storage failure/retry. Compact layout,
keyboard entry and automated WCAG AA checks are included. The gate builds and
serves the production GUI on port 4321. Screenshots, traces and HTML reports are
under `target/gui-checks/`; inspect them when changing layout or copy.

Native GUI journeys in `native.spec.ts` start the actual `meden gui` binary and
execute offline file workers against a temporary project. They cover provider
failure/restart, conversation and unsent-input retention, fresh approval, terminal
revocation, cancellation before dispatch and gateway boundary checks. The
native browser cases also repeat warm-asset host restarts six times and require
visible controls with fresh, unapproved sessions; browser failures are attached
to the test evidence. Atlas recovery waits use the compiler deadline rather than
the short ordinary element expectation.
Build the native binary before `npm run test:gui`; Linux/Bubblewrap are required, and missing
prerequisites fail rather than skip these browser tests. Native screenshots include
`native-restart.png`, `native-cancel.png`, `native-live-graph.png` and
`native-source-graph.png`. The source-graph journey covers local files, exact-line
navigation, rereading after real edits, failed observation recovery and reload;
source-only declarations must never acquire archived proof status.
Native Atlas cases require a real installed Lean distribution (4.35.0 in CI;
`MEDEN_ATLAS_LEAN` selects its host path). They prepare the actual pinned library
from an empty analysis cache, compile selected modules, inspect cross-module type/value
edges and `sorry`, and compare an elaborated scope without source comments or
theorem proof-only dependencies. Compilation failure, GUI repair and restart are
covered. This is installed-Lean analysis preparation, not a cold verifier download.
The separate isolation case runs real Lean `#eval` code to probe hidden host data,
unselected files, cleared environment, read-only source/library mounts and offline
networking; corrupted prepared resources recover via the visible preparation button.
Inference in these browser cases remains scripted and does not prove model quality.
Native application-preview cases develop actual HTML/JS/CSS/JSON through file tools,
exercise module imports and interactive controls on desktop/mobile, retain the
document across tabs, reread after edits and recover from a missing entry/restart.
They check old preview revocation, missing-asset diagnostics, size limits, folder
aliases resolving outside the selected root, browser parent/storage/API separation,
working client-side form handlers, blocked cross-origin fetch/image/script/form loads
and exact HTTP host/method/path handling.
These browser policies do not establish OS egress isolation or whole R1.
Native delivery cases review original binary snapshots, preserve the fixed target
and hash while paging bytes, refuse clipped headings and stale selections, reject
proposals and send once to a real local receiver. Redirects are not followed;
uncertain results cannot be retried, and restarting discards the old queue.
Native meaning-analysis cases also cover request separation, actual back-translation
and discrepancy rendering, unchanged proof status, target edits, historical restart,
cancellation between passes, concurrent source changes, provider limits and refusal
of model-returned executable tools. These cases use scripted provider replies and
real host/worker execution. The separate live browser driver in [gui.md](gui.md)
uses an explicitly selected actual provider and independent original Rust tests.

The standalone GUI remains a local recorded demo with archived verification
evidence. Neither browser gate replaces the public TUI gate or establishes
live-model quality, cold installation or new prover validation. See
[gui.md](gui.md) for provenance and boundaries.

For a separate-model repair trial, add `--judge-model gpt-oss:120b` to the
`meaning_smoke` command. Review both model/destination bindings in the actual TUI.
The driver retains failures as well as successful trials; invalid categories stop
instead of being normalized. `test_tui_judge.py` checks both HTTP and invalid-category
failure, retained blind translation, explicit recovery, clipped review and restart.
