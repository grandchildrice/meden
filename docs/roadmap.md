# Meden product purpose and roadmap

Planning baseline: 2026-11-26. The next milestone is to complete a real EthLib
or Riemann task through Meden, from an explicit requirement to inspected
verification results. The priorities below describe planned work and completion
criteria.

## Purpose and value

Meden connects what a developer intends to build with the evidence that supports
trusting the implementation. The developer describes the task in natural
language, reviews its specification, and follows implementation, verification
and repair in one development environment. Meden tracks which code implements
each requirement and which evidence supports it for the actual revision.

The project map is the visual entry into that relationship. Its value is to
help users understand large projects, follow the consequences of a change and
inspect the grounds for a result:

- **Understand the project.** Navigate theorems, definitions and dependencies
  directly into the relevant source.
- **Track changes.** Identify affected requirements and results that need to be
  checked again after an edit.
- **Inspect evidence.** Distinguish proofs, checks within finite bounds, tests
  and model assessments, with their assumptions and remaining obligations.

Human review connects intent to an accepted formal contract. Model assessments
and graph colors do not establish that connection or grant proof status. The
[design](design.md), [assurance plan](assurance.md) and
[current verification coverage](verification.md) define those boundaries.

## Current foundation

Shared application services and typed plugin ports separate control decisions
from compilers, search, meaning rendering, model connections and presentation.
Lean source analysis and bounded finite Z3 checking exercise those interfaces.
Revision-bound views, cached analysis and the project map support interactive
inspection. The [implementation exit audit](architecture/migration-status.md)
records the shipped paths and their validation.

Initial preparation, deep-zoom rendering and complete Lean input validation
remain latency targets. General SMT and RTL verification, including Yosys
integration, remain future backend work. The supported verification scopes are
listed in [project verification](project-verification.md).

## Priorities and completion criteria

| Priority                              | Work                                                                                                                                                                                           | Completion criterion                                                                                                                                                                                    |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. Complete a real development task   | Select a concrete EthLib or Riemann issue. Connect the requirement, target declarations, edits and verification results, and refine the existing analysis and repair workflow.                 | A user completes understanding, editing, interpreting a verification failure, repairing and checking again through the public UI. The accepted requirement and evidence scope stay explicit throughout. |
| 2. Reduce everyday waiting            | Improve initial preparation, updates after edits and Lean input validation. Increase safe reuse of analysis while retaining current-input checks and usable navigation during background work. | Named operations have measured latency budgets and regression checks. Users can keep inspecting the project during preparation and understand the current phase or failure.                             |
| 3. Make backend additions repeatable  | Document and exercise the implementation, registration and conformance steps using another tool such as Yosys. Begin with one input format and one verification mode.                          | A real example runs through an added adapter, registration and tests without changes to the authority kernel or common viewer. Results retain their actual logic, bounds and evidence kind.             |
| 4. Validate practical local operation | Run representative tasks with the intended capable 100–600B local deployment, measuring quality, latency, tool use and data boundaries.                                                        | Real model/server runs complete the tasks under the intended confinement. Local evidence is distinguished from authorized cloud trials, and confidentiality obligations remain explicit.                |
| 5. Prepare the first release          | Harden installation, dependency preparation, visible recovery, restart and artifact export.                                                                                                    | A user on a fresh environment can start and complete the supported workflow without hidden configuration repairs. Failure and restart paths work through the public interface.                          |

Establish development behavior with explicitly approved Ollama Cloud inference
first, keeping execution and checking local. Validate the large local deployment
separately; comparable performance on roughly 10B models is deferred. Backend
extensions keep the core small, use the shared interfaces and remove replaced
code. Renderer or execution-engine changes require a measured improvement on
the relevant workload.

## Immediate milestone and measures

Choose one real EthLib or Riemann task and complete the requirement-to-result
workflow. Use it to measure how much the project map and linked observations
reduce the effort of understanding, changing and reviewing the project.

Track three task-level measures alongside operation latency:

1. Time to find the relevant source and supporting evidence.
2. Time to recover from a verification failure.
3. Number of manual user interventions needed to complete the task.

Record completion and remaining obligations with each run. Reaching a fast graph
or receiving a successful model reply alone does not complete the task.
