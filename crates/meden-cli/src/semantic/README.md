# Native semantic round-trip adapter

The two-pass prompts and output fields are adapted from MIT-licensed
[metareflection/claimcheck](https://github.com/metareflection/claimcheck), commit
`8eb37fedc0acb5bc5f89efc3933cb3955268a4bd` (`src/prompts.js`, `src/schemas.js`,
`src/roundtrip.js`). See CLAIMCHECK-LICENSE. This is a Meden adapter, not execution
of the upstream CLI, SDK backends or Dafny verifier.

Changes: single selected Rust/Lean declaration with bounded observed project
context; no original requirement/chat in translation; no source/chat in comparison;
strict bounded single-call result decoding; same host-selected model in separate
requests; native processor grant and cancellation; source observations before and
after inference; local host JSON persistence and historical status on restart.
No additional network client, credential discovery, model-selected tool execution,
implicit retry, proof admission or contract acceptance is introduced.

Names/comments in source can expose intent: withholding the separate requirement
field is not a theorem about statistical independence. Lexical graphs do not give
complete elaborated dependency scope. Results remain model-generated candidates;
quality, calibration and broader backend correspondence need independent evaluation.
