# Conformance check

Runs the harness `verify` command against the corpus package. It does not load
the binding, so it works on any machine that can install the workspace.

```sh
pnpm --filter @notedge/panduck-examples conformance
```
