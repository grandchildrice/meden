# Saved maps across parser updates

The previous deployment measured a 99.833-second Riemann syntax-cache population
inside a 107.278-second startup job. The saved-map filename also depended on the
navigation implementation, so changing a parser made the former presentation
unavailable while that work ran. Faster indexing did not remove this coupling.

Saved navigation now has one host-scoped, versioned presentation frame. It owns
the original lexical indexer and composition identities alongside the complete
metadata. Restoring it preserves those identities in the published inventory;
it does not attribute old headings to the current parser. The frame retains its
64 MiB bound, checksum, strict record shape and atomic replacement. Unsupported
versions, malformed origins and damaged bodies fall back to ordinary discovery.
No old-format decoder or namespace migration is retained.

The existing `Saved map` status remains visible until current observation
completes. A former composition serves only its original retained files, without
querying the current parser's incomplete private cache under an old parent.
Opening a current file independently cannot merge that result into the former
map. Complete refresh replaces the frame and its publication together. Matching
unchanged restarts still reuse the existing publication without rewriting it.

The map's existing Retry action now also covers a failed workspace refresh,
rather than only failed inventory/detail transfers. It sends the same typed
refresh command as the explorer. No parser or prover work is moved into rendering,
and no model request, approval, contract or evidence is restored from this frame.

## Verification scope

The new public journey builds actual old/new parser executables, prepares a
10,002-file project and real Lean/Plausible/GF observations, then changes a theorem
without changing its byte length. A trusted launcher holds the new host's actual
directory worker. The original map, complete original inventory envelope,
indexer, saved bytes and selected chunk remain available before that worker runs.
Zoom uses the retained view, without a command or inference request.

An explicit dispatch failure admits no worker result. After that injected host
fault ends, the journey requires a visible local error, independently opens the
changed source with the real compiler, verifies that the former map was not
relabeled, clicks Retry map and observes the new declaration with the same camera.
Reopening retains the updated frame with fresh model approval required. Both
motion settings exercise this journey. A separate executable that reinstates the
former composition check with the same current frame/schema must fail saved-map
visibility after its real first preparation and held new discovery succeed.

Two initial exploratory runs kept the global launcher fault active while waiting
for the visible map error and failed that assertion. The final controlled journey
ends the fault after observing the actual failed host state. It does not establish
error-display latency during an ongoing global worker outage, and changing this
fixture is not counted as an application performance improvement.

The [numeric record](../../results/saved-map-parser-update-20261126.json) retains
executable/source hashes, browser attachments, operation outcomes and observation
limits. Time from navigation to a visible saved target plus two animation
callbacks measures an automation-visible presentation proxy; it excludes host
startup and is not physical presentation latency. The held-worker condition is
the acceptance criterion, rather than assuming that a fast complete rebuild
establishes saved-view reuse.

The two motion settings observe their saved target at 1.293 / 1.215 seconds while
new discovery is held. These are individual fixture observations, not population
percentiles. Both six-case browser sets, 192 application tests, 64 architecture
checks and the 37 required TUI journeys pass. Nine decoded TUI journeys (21
observations) and rendered map/source screens were inspected. The test driver
terminates after the final normal-motion report is published; its last runner
exit is unobserved. A separate decoder confirms all six final case outcomes with
no skips/flakes. The original interrupted driver record is retained rather than
reporting a clean exit for it.

## Actual hosts

Both hosts use executable `6a3039f6b5c9432f` and the matching 88 browser assets.
One sequential normal-motion restart per project, on retained project/OS caches,
observes:

| Observation                                   | Riemann |   EthLib |
| --------------------------------------------- | ------: | -------: |
| Navigation to scene plus two callbacks        | 3.156 s |  2.587 s |
| Restart request to complete ready observation | 9.029 s | 19.866 s |
| Original inventory file count                 | 175,187 |    2,517 |

Both first scenes display `Saved map` while refresh is running, without model
approval. No test, build or second sampler runs during these captures. They use
the unchanged current parser, so they demonstrate warm startup viewing rather
than an actual-project parser-update timing pair or a general startup speedup.
The explicit method-update/held-worker journey above covers that separate case.

After file inspection, complete source, reader, inventory, navigation and evidence
response bytes match the original five responses per project. All 58 Riemann and
46 EthLib compiler/search/render files and indices retain their exact hashes and
sizes. History/candidates are unchanged, selected files report cache hits, and
both hosts remain idle and unapproved.

This changes first presentation across compatible parser updates. It does not
shorten complete private-cache population, current-input fingerprinting or the
first-detail SVG/Canvas intervals. A missing new-format frame still requires
ordinary observation once. Actual compilation/search/English cache identities
and the lexical parser itself are unchanged by this storage change. Installed
tools and scripted providers are not live-model or cold-install validation.
