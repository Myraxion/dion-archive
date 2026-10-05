# Idempotent Unset and File Entry Order Preservation

`unset` operations are strictly idempotent: removing a non-existent entry returns exit code 0 rather than erroring out. In-place edits strictly preserve the line order of unaffected entries, and newly introduced entries are appended to the end of the file, maintaining Total Commander compatibility and minimizing diff churn.
