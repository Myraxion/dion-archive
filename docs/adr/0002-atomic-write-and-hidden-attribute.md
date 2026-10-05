# Atomic Replacement and Hidden Attribute Invariance

Writes to `descript.ion` stage to a sibling temporary file created with `FILE_ATTRIBUTE_HIDDEN` and explicitly flushed via `FlushFileBuffers`, followed by atomic replacement via `ReplaceFileW` / `MoveFileExW`. This eliminates crash-induced half-written state while preserving the hidden attribute throughout the file lifecycle.
