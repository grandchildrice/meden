#!/usr/bin/env bash
# Regenerate from production Rust; never normalize or hand-edit the model.
set -euo pipefail
cd "$(dirname "$0")/.."

charon_bin=${CHARON:-charon}
aeneas_bin=${AENEAS:-aeneas}
case $("$charon_bin" version) in
  *8cae8fd1905946de61147a65ecdd7122cc6b491b*) ;;
  *) echo 'Unsupported Charon revision' >&2; exit 1 ;;
esac
case $("$aeneas_bin" -version) in
  *c5eb4a2*) ;;
  *) echo 'Unsupported Aeneas revision' >&2; exit 1 ;;
esac

units=(evidence approval repair conversation text processor policy)
case "$#:${1:-}:${2:-}" in
  0::|1:--write:) ;;
  2:--write:evidence|2:--write:approval|2:--write:repair|2:--write:conversation|2:--write:text|2:--write:processor|2:--write:policy) units=("$2") ;;
  *) echo 'Usage: tools/extract-transfer.sh [--write [evidence|approval|repair|conversation|text|processor|policy]]' >&2; exit 2 ;;
esac
extract_tmp=$(mktemp -d)
trap 'rm -rf "$extract_tmp"' EXIT
for unit in "${units[@]}"; do
  namespace=MedenImpl
  cargo_args=(--locked --lib -p meden-core)
  extraction_args=()
  case "$unit" in
    policy)
      namespace=MedenPolicyImpl
      rust_roots=(meden_core::glob::glob_match_chars
        meden_core::policy::covers_ref meden_core::policy::violates_ref
        meden_core::policy::allowed_ref meden_core::policy::allowed
        meden_core::policy::violates meden_core::policy::Grant::covers
        meden_core::policy::Policy::denies meden_core::policy::Policy::grants
        meden_core::invocation::Attempt::new meden_core::invocation::Attempt::with_policy
        meden_core::invocation::Call::into_parts meden_core::invocation::Attempt::announce
        meden_core::invocation::Attempt::dispatch)
      # Keep public failure constructors present even when a faulty caller
      # stops using one, so missing definitions cannot masquerade as rejection.
      for method in refused cancelled stopped invalid; do
        rust_roots+=("meden_core::invocation::Error::$method")
      done
      for method in new prepare into_parts bind; do
        rust_roots+=("meden_core::file::Request::$method")
      done
      rust_roots+=(meden_core::file::Resolved::consume)
      extraction_args=(--extract-opaque-bodies --opaque core --opaque alloc --opaque std
        --include core::cmp::impls --include core::option::Option --include core::result::Result
        --include core::clone::impls)
      model=Policy ;;
    evidence)
      namespace=MedenEvidenceImpl
      # Shared core types are generated once for evidence/export composition.
      rust_roots=(meden_core::export_control::step
        meden_core::export_control::FrozenRequest::target
        meden_core::export_control::FrozenRequest::body
        meden_core::export_control::Dispatch::into_request)
      for method in new request phase advance; do
        rust_roots+=("meden_core::export_control::State::$method")
      done
      for method in propose review accept is_accepted refresh record eligible; do
        rust_roots+=("meden_core::evidence::Gate::$method")
      done
      for method in new start invalidate abort finish is_active complete; do
        rust_roots+=("meden_core::evidence::CheckJobs::$method")
      done
      for method in new propose displayed accepted pending is_accepted review accept reject refresh start invalidate edit abort is_active complete eligible; do
        rust_roots+=("meden_core::evidence::Contracts::$method")
      done
      rust_roots+=(meden_core::evidence::Contracts::prepare_delivery)
      rust_roots+=(meden_core::evidence::Contracts::inspect
        meden_core::evidence::Inspection::accepted meden_core::evidence::Inspection::consume
        meden_core::evidence::Inspection::matches meden_core::evidence::same_view)
      for method in contract request phase advance; do
        rust_roots+=("meden_core::evidence::Delivery::$method")
      done
      model=Evidence ;;
    approval)
      rust_roots=(meden::tui::approval::approves meden::tui::approval::route)
      namespace=MedenUIImpl
      cargo_args=(--locked --bin meden -p meden-cli)
      # Extract the actual event/bitflag/char comparisons, rather than importing
      # unknown library functions. Keep unrelated standard internals opaque.
      extraction_args=(--extract-opaque-bodies --include crossterm --include bitflags
        --opaque core --opaque alloc --opaque std --include core::cmp::impls
        --include core::option::Option)
      model=Approval ;;
    repair)
      namespace=MedenRepairImpl
      cargo_args=(--locked --lib -p meden-runtime)
      rust_roots=(meden_runtime::verification::turn::Turn::begin
        meden_runtime::verification::turn::Turn::advance
        meden_runtime::verification::failure::failed_worker
        meden_runtime::verification::failure::verification_stops
        meden_runtime::verification::failure::FailureKind::repairable
        meden_runtime::file::complete meden_runtime::file::verification_kind
        meden_runtime::pages::public_v4 meden_runtime::pages::public_v6
        meden_runtime::verification::contract::decimal_words::parse_u64)
      extraction_args=(--extract-opaque-bodies --include meden_agent::tool::ToolError
        --opaque core --opaque alloc --opaque std
        --include core::option::Option --include core::result::Result
        --include std::process --include std::sys --include std::os::unix)
      model=Repair ;;
    conversation)
      namespace=MedenConversationImpl
      cargo_args=(--locked --lib -p meden-agent)
      rust_roots=(meden_agent::flow::Flow::begin
        meden_agent::flow::interrupt
        meden_agent::flow::Flow::advance
        meden_agent::flow::Flow::pending_call
        meden_agent::flow::Flow::reply
        meden_agent::flow::Flow::result
        meden_agent::flow::Flow::into_outcome
        meden_agent::conversation::execution_error
        meden_agent::conversation::tool_error
        meden_agent::conversation::Status::inspection)
      for method in failed refused unavailable inconclusive cancelled parts into_parts; do
        rust_roots+=("meden_agent::tool::ToolError::$method")
      done
      extraction_args=(--extract-opaque-bodies --include meden_core::invocation::Error
        --opaque core --opaque alloc --opaque std --include core::cmp::impls
        --include core::option::Option --include core::result::Result)
      model=Conversation ;;
    text)
      namespace=MedenTextImpl
      cargo_args=(--locked --bin meden -p meden-cli)
      rust_roots=(meden::text::visible_codepoint)
      model=Text ;;
    processor)
      namespace=MedenProcessorImpl
      rust_roots=()
      for method in new binding phase expires approve revoke is_authorized dispatch; do
        rust_roots+=("meden_core::processor::State::$method")
      done
      for method in new engine weights; do
        rust_roots+=("meden_core::processor::LocalParameters::$method")
      done
      for method in new provider endpoint model credential; do
        rust_roots+=("meden_core::processor::HttpParameters::$method")
      done
      rust_roots+=(meden_core::processor::LocalCall::consume_for
        meden_core::processor::LocalCall::into_parameters
        meden_core::processor::HttpCall::into_parameters)
      model=Processor ;;
  esac
  extract_roots=()
  for rust_root in "${rust_roots[@]}"; do
    extract_roots+=(--start-from "$rust_root")
  done
  CARGO_TARGET_DIR="$PWD/target/verification/rust" \
    "$charon_bin" cargo --preset=aeneas --sysroot default \
    "${extraction_args[@]}" "${extract_roots[@]}" \
    --dest-file "$extract_tmp/$unit.llbc" -- "${cargo_args[@]}"
  "$aeneas_bin" -backend lean -namespace "$namespace" -dest "$extract_tmp" \
    -abort-on-error -warnings-as-errors -no-progress-bar "$extract_tmp/$unit.llbc"
  if grep -En '\b(axiom|sorry|admit)\b' "$extract_tmp/$model.lean"; then
    echo 'Extraction left an unknown definition or proof hole' >&2
    exit 1
  fi
  definitions=()
  if [[ $unit == evidence ]]; then
    definitions=(evidence.Contracts.new evidence.Contracts.propose
      evidence.Contracts.displayed evidence.Contracts.impl.accepted
      evidence.Contracts.pending evidence.Contracts.is_accepted
      evidence.Contracts.review evidence.Contracts.accept evidence.Contracts.reject
      evidence.Contracts.refresh evidence.Contracts.start evidence.Contracts.invalidate
      evidence.Contracts.edit evidence.Contracts.abort evidence.Contracts.is_active
      evidence.Contracts.complete evidence.Contracts.eligible
      evidence.Contracts.prepare_delivery evidence.Delivery.impl.contract
      evidence.Contracts.inspect evidence.Inspection.impl.accepted evidence.Inspection.consume
      evidence.Inspection.matches evidence.same_view
      evidence.Delivery.request evidence.Delivery.phase evidence.Delivery.permitted evidence.Delivery.advance
      export_control.step export_control.State.new export_control.State.impl.request
      export_control.State.impl.phase export_control.State.advance
      export_control.FrozenRequest.impl.target export_control.FrozenRequest.impl.body
      export_control.Dispatch.into_request)
  elif [[ $unit == policy ]]; then
    definitions=(glob.glob_match_chars glob.advance_row scope.Scope.matches
      policy.covers_ref policy.Grant.covers policy.grants_ref policy.refusal_ref
      policy.violates_ref policy.evaluate_ref policy.allowed_ref policy.evaluate
      policy.allowed policy.violates policy.Policy.impl.denies policy.Policy.impl.grants
      invocation.Attempt.new invocation.Attempt.with_policy invocation.Call.into_parts
      invocation.Attempt.announce invocation.Attempt.dispatch
      invocation.Error.refused invocation.Error.cancelled invocation.Error.stopped invocation.Error.invalid
      file.Request.new file.Request.prepare file.Request.into_parts file.Request.bind file.Resolved.consume)
  elif [[ $unit == approval ]]; then
    definitions=(tui.approval.approves tui.approval.route)
  elif [[ $unit == text ]]; then
    definitions=(text.visible_codepoint)
  elif [[ $unit == processor ]]; then
    definitions=(processor.State.new processor.State.impl.binding
      processor.State.impl.phase processor.State.impl.expires processor.State.approve
      processor.State.revoke processor.State.is_authorized processor.State.dispatch
      processor.LocalParameters.new processor.LocalParameters.impl.engine
      processor.LocalParameters.impl.weights processor.HttpParameters.new
      processor.HttpParameters.impl.provider processor.HttpParameters.impl.endpoint
      processor.HttpParameters.impl.model processor.HttpParameters.impl.credential
      processor.LocalCall.consume_for processor.LocalCall.into_parameters processor.HttpCall.into_parameters)
  elif [[ $unit == conversation ]]; then
    # External roots can be pruned by the pinned translator. Require the
    # production entry points, rather than accepting helpers alone.
    definitions=(flow.Flow.begin flow.interrupt flow.Flow.advance
      flow.Flow.pending_call flow.Flow.impl.reply flow.Flow.impl.result
      flow.Flow.into_outcome conversation.Status.inspection)
    definitions+=(conversation.execution_error conversation.tool_error)
    definitions+=(tool.ToolError.failed tool.ToolError.refused tool.ToolError.unavailable
      tool.ToolError.inconclusive tool.ToolError.cancelled tool.ToolError.parts tool.ToolError.into_parts)
  elif [[ $unit == repair ]]; then
    definitions=(verification.turn.Turn.begin verification.turn.Turn.advance
      verification.failure.failed_worker verification.failure.verification_stops
      verification.failure.FailureKind.repairable file.complete file.verification_kind
      pages.public_v4 pages.public_v6 verification.contract.decimal_words.parse_u64)
    definitions+=(meden_agent.tool.ToolError.failed meden_agent.tool.ToolError.refused
      meden_agent.tool.ToolError.unavailable meden_agent.tool.ToolError.inconclusive
      meden_agent.tool.ToolError.cancelled meden_agent.tool.ToolError.parts
      meden_agent.tool.ToolError.into_parts)
  fi
  for definition in "${definitions[@]}"; do
    if ! awk -v name="$definition" '$1 == "def" && $2 == name { found = 1 }
      END { exit !found }' "$extract_tmp/$model.lean"; then
      echo "Missing extracted entry point: $definition" >&2
      exit 1
    fi
  done
  case ${1:-} in
    --write) cp "$extract_tmp/$model.lean" "lean/MedenSpec/Generated/$model.lean" ;;
    '') diff -u "lean/MedenSpec/Generated/$model.lean" "$extract_tmp/$model.lean" ;;
  esac
done
