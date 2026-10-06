# Target Existence Validation for Set Command

`dion set` strictly requires the target file or directory to exist in the parent directory before modifying `descript.ion`, failing immediately with exit code `1` (`target not found: ...`) if missing. Existence is verified via non-traversing `symlink_metadata` to avoid unnecessary I/O or symlink loops, while `get` and `unset` intentionally omit this check to allow reading and idempotently cleaning up historical orphan entries.
