# Kraits Repo Adaptor (`kraits-adaptor`)

Zero-dependency Rust AST indexer, KV-prefix lock proxy, loop-guarded coding-agent loop, and minimalist web UI (`/`). Designed to wrap local OpenAI-compatible inference servers (such as `llama-server` / Kraits) so small quantized local models stop looping on repository tasks.

## Build & Test

Requires Rust (`cargo`). Zero external crates are used (`Cargo.toml`).

```sh
cd kraits-adaptor
cargo test --release
cargo build --release
```

The compiled binary is written to `target/release/kraits-adaptor` (`target\release\kraits-adaptor.exe` on Windows).

## Serve Command & Flags

```sh
kraits-adaptor serve --repo <dir> [--listen 127.0.0.1:8100] [--upstream http://127.0.0.1:8099]
```

| Flag | Default | Description |
| --- | --- | --- |
| `--repo <dir>` | `.` | Path to the single locked target Git repository |
| `--listen <host:port>` | `127.0.0.1:8100` | Local HTTP bind address for UI, `/kraits/*`, and `/v1/chat/completions` |
| `--upstream <url>` | `http://127.0.0.1:8099` | Upstream OpenAI-compatible `/v1/chat/completions` server |

### Other CLI Subcommands

```sh
kraits-adaptor index  --repo <dir>
kraits-adaptor purge  --repo <dir>
kraits-adaptor ast    --repo <dir> <rel-file>
kraits-adaptor symbol --repo <dir> <rel-file> <symbol>
kraits-adaptor query  --repo <dir> <query> [--budget <chars>]
kraits-adaptor learn  --repo <dir> --kind <kind> --sig <sig> --detail <detail> [--file <rel>]
```

## Tested Model & Verified Scope (`v0.6.28`)

- **Model tested:** `Qwen3.8-Flash-Next-IQ4_XS` served locally (`-c 10240` context) on a 16 GB Mac (`16.5–17.7 tok/s` decode, zero cloud APIs).
- **Verified benchmark (`pallets/flask`, 83 indexed files):**
  - Signed `v0.6.28` (`kraits-adaptor.exe`): **`4 of 4`** planted single-function Python bugs fixed and `pytest`-verified in **`73–189 s`** (`4–9` turns), KV prefix `cache_n = 988` on Turn 1.
  - Across `v0.6.21–v0.6.28`: **`23 of 26`** planted single-function bug runs green.
  - **Known limits:** Multi-file feature generation and real open GitHub issues (`#6123`) remain **`0 of 7`** at this model size and 10K context.

## Endpoints & Web UI

| Method & Path | Purpose |
| --- | --- |
| `GET /` or `GET /ui` | Single-column chat & diff web UI |
| `POST /v1/chat/completions` | OpenAI-compatible chat completions proxy with repo context injection |
| `GET /kraits/repo` | Repo version, indexed AST files, active task ID, and served request count |
| `GET /kraits/file?path=<rel>` | Line count and AST symbol skeleton for an indexed file |
| `GET /kraits/memory` | Persisted `.kraits/` episodic lessons and hot-file counts |
| `GET /kraits/diff` | Current `git diff` in the locked repository |
| `POST /kraits/learn` | Manually record a convention/pitfall lesson into `.kraits/memory.jsonl` |
| `POST /kraits/task` | Start an agent task (`{"prompt": "..."}`) -> returns `{"task_id": "..."}` |
| `GET /kraits/task?id=<task_id>` | Poll task status, turn traces, git diff, and final summary |
| `POST /kraits/demo` | Demo harness (`{"action": "bug1"\|"bug2"\|"feat1"\|"reset"\|"pytest", "target": "..."}`) |
