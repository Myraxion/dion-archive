# Fail-Safe on Malformed descript.ion Data

When parsing existing `descript.ion` files, Dion immediately halts write modifications with exit code `3` if invalid UTF-8 bytes, unclosed quotes, duplicate case-insensitive entries, or corrupted lines are detected. This prevents silent data loss or overwriting of foreign records.
