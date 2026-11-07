"""Artifact frontends: turn a package format into a requirement set.

A frontend knows how to read one kind of artifact (an Agent Skills folder,
an MCP manifest, an h5i kernel, ...) and reports the capabilities the artifact
would need. Frontends are *readers*, not verifiers: what they extract is a
claim about the artifact that the admission gate then checks against policy
and against attestations.
"""
