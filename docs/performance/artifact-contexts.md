# Typed publication contexts

The [validation record](../../results/artifact-contexts-20261125.json) follows the original publication bytes through 13 distinct application-owned view contexts. Actual construction, references, retention tables and query APIs use `ArtifactId<Domain>`, `ArtifactRef<Domain>` and `Publication<Domain>`. The former untyped publication API and caller-selected hash-domain argument are removed. The trusted kernel and its controller operations are unchanged.

## Ownership

Each DTO family declares one domain and keeps the original fixed hash prefix. Borrowed projections share their owning family without retaining a source lifetime in the frozen publication. Construction/update accepts only that family; source and reader IDs cannot be exchanged implicitly, and a directory publication cannot enter a reader retention table. Source retention also uses typed IDs for its current pointer, file bindings and eviction order.

IDs own validated canonical lowercase SHA-256 spellings. They have no implicit string dereference or cross-domain conversion. Explicit wire decoding may label any correctly spelled digest for a route, so it never establishes ownership or authority. Every original owner still checks its exact retained identity, and original source/parent/job/session/freshness checks stay in place. Native HTTP routes decode their own view context and then call that owner. Malformed or foreign reads remain conflicts without starting a job.

## Wire and deployment

All 37 generated browser validators are byte-identical to the preceding revision. JSON Schema now names the shared scalar and context-specific references, while the serialized wire remains ordinary strings and the original envelopes. The 88 tested production assets are byte-identical to the preceding deployment. Actual complete Riemann/EthLib inventory bodies retain their exact original bytes; source/graph/reader/evidence/history and model bindings are checked against the original projects. The compiler's 13 captured sources retain their original hashes and the real file opens reuse their prepared observations.

The Rust nominal types do not survive JSON as browser brands. Decoding and host comparisons, rather than type names or SHA-shaped text, still determine whether a body may be read. This is a view/application boundary improvement, not a new kernel, renderer or I/O proof.

## Validation

All 128 application unit cases, 49 CLI unit cases and 8 documentation compile cases pass. The five new examples include matching query/construction cases and rejected cross-owner query, wrong DTO and wrong retention-table cases. Matching examples exercise the same public APIs, so a missing API cannot stand in for type rejection. Scalar/property checks retain exact digest strings and escaped Unicode/control-text framing, allocation reuse and canonical decoding. An explicitly re-decoded foreign digest still fails the original owner's lookup.

The actual source-publication browser journey issues 54 authenticated reads across six simultaneously retained owners: six own reads succeed, 30 foreign reads and 18 malformed reads fail. The original sources/readers/directories/history/evidence/inventory and chat remain unchanged. The same journey retains transfer failure/retry, bounded conflict retry, old-body refusal, refresh and unapproved restart. Its normal-motion repeat also passes.

The complete 35-journey public TUI and 71-journey browser gates pass without skips, flaky or unexpected cases. Decoded Lake freshness-failure and historical/cancel/restart screens were inspected. The 3,000-case map/reference/update corpora and 15 algorithm performance cases pass. The separate serialized real-Lean retained-tab gate requires no new inspections, preserves source/camera and retains its 250 ms two-rAF p95 bound:

| Motion | Retained-tab p95 |
| --- | ---: |
| reduce | 35.0 ms |
| no-preference | 38.0 ms |

The first normal-motion attempt produced a successful browser verdict, but its runner exited 143 before recording completion. That attempt is retained as interrupted; the independent replacement exited zero and supplies the normal-motion budget above.

Those timings are presentation proxies on the installed fixture, not physical paint, field INP, live-model quality or cold installation. Full interaction/startup speed measurements remain in the [preceding performance audit](inventory-states.md); no new speedup is attributed to nominal types.

The session/backend snapshot/accepted-revision/job identity audit and cone-layout/versioning/retirement work remain open in [the architecture plan](../architecture/typed-core-and-plugins.md).
