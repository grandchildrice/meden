# Local tiny_http 0.12.0 patch

Source: the crates.io `tiny_http` 0.12.0 package, checksum
`389915df6413a2e74fb181895f933386023c71110878cd0825588928e64cdc82`.
Upstream: <https://github.com/tiny-http/tiny-http>. The original MIT/Apache-2.0
notices are retained. This is the active dependency selected by the root Cargo
patch; the excluded package has its own locked unit-test build.

The connection pool now counts queued jobs as reservations of idle threads.
Previously a burst could reserve the same idle capacity repeatedly; remaining
connections then waited indefinitely while earlier keep-alive connections stayed
open. This stranded a module-preload request and left native GUI restarts blank.
The private scheduler regression reserves all actual idle workers and requires
another job to start before the earlier connections are released. It failed on
the original condition and passes with the capacity correction. Browser journeys
exercise real HTTP and repeated restart separately.

Only library sources, its manifest, README and license files are retained.
Unused example-only development dependencies and a private unused reexport were
removed; the original compile-time Send/Sync assertion has a targeted dead-code
annotation so path-dependency builds work with Meden's warning gate.
