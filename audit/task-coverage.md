# Task coverage matrix

Generated from `/api/surface` and `/api/describe` against the canonical CLI.

**Coverage:** 82/82 leaf commands expose a generated form; every form includes required positionals, valued options, and global flags.

| Command | Workspace | Class | Required inputs | Options |
|---|---|---|---:|---:|
| `aether discover commands` | project | read | 0 | 5 |
| `aether discover project-context` | project | read | 0 | 6 |
| `aether commands` | schema | read | 0 | 5 |
| `aether init` | project | read | 1 | 6 |
| `aether doctor` | overview | read | 0 | 6 |
| `aether wingbox config snapshot` | manufacturing | read | 1 | 6 |
| `aether wingbox config verify` | manufacturing | read | 2 | 5 |
| `aether wingbox release-gate` | manufacturing | read | 1 | 5 |
| `aether wingbox manufacturing verify` | manufacturing | read | 1 | 5 |
| `aether wingbox manufacturing as-built` | manufacturing | read | 1 | 6 |
| `aether wingbox manufacturing impact` | manufacturing | read | 1 | 7 |
| `aether wingbox model verify` | manufacturing | read | 1 | 6 |
| `aether wingbox model promote` | manufacturing | read | 1 | 7 |
| `aether wingbox fidelity plan` | manufacturing | read | 1 | 5 |
| `aether demo` | project | read | 1 | 5 |
| `aether run` | project | read | 0 | 7 |
| `aether explain` | project | read | 0 | 6 |
| `aether experiment` | project | read | 1 | 6 |
| `aether evidence export` | evidence | read | 0 | 6 |
| `aether evidence gate` | evidence | approval | 1 | 5 |
| `aether export` | evidence | read | 0 | 7 |
| `aether replay` | evidence | read | 1 | 5 |
| `aether benchmark` | design | long | 1 | 5 |
| `aether project status` | project | read | 1 | 5 |
| `aether graph query` | graph | read | 0 | 9 |
| `aether graph as-of` | graph | read | 1 | 6 |
| `aether graph chain` | graph | read | 0 | 6 |
| `aether plugin list` | schema | read | 0 | 6 |
| `aether package verify` | schema | read | 1 | 6 |
| `aether workflow run` | project | read | 1 | 6 |
| `aether workflow validate` | project | read | 1 | 5 |
| `aether geometry recipe` | design | read | 1 | 6 |
| `aether mesh generate` | design | read | 0 | 8 |
| `aether requirement add` | verification | read | 0 | 10 |
| `aether requirement import-sysml` | verification | read | 1 | 6 |
| `aether requirement report` | verification | read | 0 | 6 |
| `aether simulate compose` | design | read | 1 | 5 |
| `aether uq propagate` | design | read | 1 | 5 |
| `aether uq sobol` | design | long | 1 | 5 |
| `aether optimize` | design | long | 1 | 6 |
| `aether twin estimate` | design | read | 1 | 5 |
| `aether manufacture plan` | manufacturing | read | 1 | 5 |
| `aether inspect acquire` | manufacturing | read | 1 | 5 |
| `aether test run` | manufacturing | read | 1 | 5 |
| `aether deploy plan` | operations | read | 1 | 5 |
| `aether deploy apply` | operations | read | 1 | 5 |
| `aether deploy airgap-check` | operations | read | 0 | 5 |
| `aether envman verify` | operations | read | 1 | 6 |
| `aether envman install` | operations | read | 2 | 6 |
| `aether verify-tuf` | evidence | read | 2 | 5 |
| `aether dark-build-check` | operations | long | 0 | 5 |
| `aether schema list` | schema | read | 0 | 6 |
| `aether schema show` | schema | read | 1 | 6 |
| `aether status` | overview | read | 0 | 6 |
| `aether policy explain` | intelligence | read | 1 | 7 |
| `aether provenance explain` | graph | read | 1 | 6 |
| `aether validation report` | verification | read | 0 | 6 |
| `aether plan explain` | design | read | 1 | 5 |
| `aether config snapshot` | manufacturing | read | 1 | 6 |
| `aether config verify` | manufacturing | read | 2 | 5 |
| `aether manufacturing verify` | manufacturing | read | 1 | 5 |
| `aether manufacturing as-built` | manufacturing | read | 1 | 6 |
| `aether manufacturing impact` | manufacturing | read | 1 | 7 |
| `aether model verify` | intelligence | approval | 1 | 6 |
| `aether model promote` | intelligence | read | 1 | 7 |
| `aether fidelity plan` | design | read | 1 | 5 |
| `aether design evaluate` | design | read | 1 | 5 |
| `aether design lineage` | design | read | 1 | 5 |
| `aether agent check` | intelligence | read | 1 | 5 |
| `aether units check` | manufacturing | read | 1 | 5 |
| `aether sandbox check` | operations | read | 1 | 5 |
| `aether sandbox run` | operations | read | 1 | 6 |
| `aether vv coverage` | verification | read | 1 | 5 |
| `aether vv decisions` | verification | read | 1 | 5 |
| `aether material check` | manufacturing | read | 1 | 5 |
| `aether loads validate` | manufacturing | read | 1 | 5 |
| `aether infer` | intelligence | read | 1 | 5 |
| `aether agent-run` | intelligence | approval | 1 | 6 |
| `aether classify check` | operations | read | 1 | 5 |
| `aether metrics` | overview | read | 0 | 5 |
| `aether token issue` | intelligence | read | 1 | 5 |
| `aether release prepare` | evidence | approval | 1 | 7 |
