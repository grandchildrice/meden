# Examples

```bash
cargo build -p tagent-cli
T=target/debug/tagent

$T extract examples/skills/pdf-helper              # what it asks for
$T check   examples/skills/pdf-helper --policy examples/policies/read-only.json   # exit 2
$T check   examples/skills/csv-summary --policy examples/policies/read-only.json  # exit 0

# Produce a Lean-backed attestation for the benign skill from this repository's own spec
# (any theorem works for the demonstration; a real catalogue ties property ids to statements)
$T attest lean --skill examples/skills/csv-summary --project lean \
   --module TAgentSpec.Policy --theorem TAgentSpec.Policy.deny_wins \
   --property inv.v0.deny-wins --statement lean/TAgentSpec/Policy.lean --out /tmp/att.json
$T gate examples/skills/csv-summary --policy examples/policies/read-only.json --attestation /tmp/att.json
```
