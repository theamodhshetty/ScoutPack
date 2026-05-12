# Design

ScoutPack turns a local repo into compact context packets.

Pipeline:

1. Walk repo with `.gitignore` and `.scoutpackignore`.
2. Skip unsupported, binary, large, and sensitive files.
3. Chunk files into meaningful units.
4. Store files, chunks, symbols, imports, commands, and skipped files in SQLite.
5. Query SQLite FTS5.
6. Render markdown under approximate token budget.

Default installs avoid embeddings and command execution.

Optional semantic mode is gated behind the `semantic` Cargo feature. It can store local fastembed vectors in SQLite and combine FTS score with cosine similarity when users pass `--semantic`.
