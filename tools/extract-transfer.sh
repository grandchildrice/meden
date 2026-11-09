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

units=(control evidence approval repair conversation text processor)
case "$#:${1:-}:${2:-}" in
  0::|1:--write:) ;;
  2:--write:control|2:--write:evidence|2:--write:approval|2:--write:repair|2:--write:conversation|2:--write:text|2:--write:processor) units=("$2") ;;
  *) echo 'Usage: tools/extract-transfer.sh [--write [control|evidence|approval|repair|conversation|text|processor]]' >&2; exit 2 ;;
esac
extract_tmp=$(mktemp -d)
trap 'rm -rf "$extract_tmp"' EXIT
for unit in "${units[@]}"; do
  namespace=MedenImpl
  cargo_args=(--locked --lib -p meden-core)
  extraction_args=()
  case "$unit" in
    control)
      rust_roots=(meden_core::export_control::step
        meden_core::export_control::FrozenRequest::target
        meden_core::export_control::FrozenRequest::body
        meden_core::export_control::Dispatch::into_request)
      for method in new request phase advance; do
        rust_roots+=("meden_core::export_control::State::$method")
      done
      model=Control ;;
    evidence)
      namespace=MedenEvidenceImpl
      rust_roots=()
      for method in propose review accept is_accepted refresh record eligible; do
        rust_roots+=("meden_core::evidence::Gate::$method")
      done
      for method in new start invalidate abort finish is_active complete; do
        rust_roots+=("meden_core::evidence::CheckJobs::$method")
      done
      for method in new propose displayed accepted pending is_accepted review accept reject refresh start invalidate edit abort is_active complete eligible; do
        rust_roots+=("meden_core::evidence::Contracts::$method")
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
        meden_runtime::verification::failure::FailureKind::repairable)
      extraction_args=(--include std::process --include std::sys --include std::os::unix)
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
        meden_agent::conversation::Status::inspection)
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
  if [[ $unit == control ]]; then
    definitions=(export_control.step
      export_control.State.new export_control.State.impl.request
      export_control.State.impl.phase export_control.State.advance
      export_control.FrozenRequest.impl.target export_control.FrozenRequest.impl.body
      export_control.Dispatch.into_request)
  elif [[ $unit == evidence ]]; then
    definitions=(evidence.Contracts.new evidence.Contracts.propose
      evidence.Contracts.displayed evidence.Contracts.impl.accepted
      evidence.Contracts.pending evidence.Contracts.is_accepted
      evidence.Contracts.review evidence.Contracts.accept evidence.Contracts.reject
      evidence.Contracts.refresh evidence.Contracts.start evidence.Contracts.invalidate
      evidence.Contracts.edit evidence.Contracts.abort evidence.Contracts.is_active
      evidence.Contracts.complete evidence.Contracts.eligible)
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
