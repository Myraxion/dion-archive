# Unified Input Channels for Comment Modification

The `set` command unifies comment modification through three mutually exclusive channels: positional CLI argument, standard input (`-` or `--stdin`), and editor invocation (`-e`). The editor mode is strictly gated behind an interactive TTY check, while standard input allows agents and pipelines to transmit arbitrary text without shell escape degradation.
