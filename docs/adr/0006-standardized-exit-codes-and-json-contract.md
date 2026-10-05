# Standardized Exit Codes and Universal JSON Contract

Dion establishes a rigid four-tier exit code system (`0`: success/idempotent, `1`: not found, `2`: usage error, `3`: operation/data error) to enable deterministic automation. Both `get` and `list` commands provide a first-class `--json` flag to emit machine-readable UTF-8 payloads, decoupling scripts and agents from terminal layout heuristics.
