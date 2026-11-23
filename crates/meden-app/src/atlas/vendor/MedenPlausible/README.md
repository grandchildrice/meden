# Plausible core

Upstream: https://github.com/leanprover-community/plausible
Revision: b7eb3304aeae834b12dda98993a37f6a41f6f0bb (v4.33.0).
Apache-2.0; original notices retained.

Only the six runtime generator/test modules are bundled. The module and namespace
are renamed to MedenPlausible to avoid selecting or replacing the project's own Plausible library. Generators, shrinking and TestResult behavior are unchanged.
Meden uses an explicit fixed seed; no successful sample run grants proof authority.
