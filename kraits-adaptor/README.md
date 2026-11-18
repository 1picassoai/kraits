# Kraits Repo Adaptor (`kraits-adaptor`)

Zero-dependency AST indexer, KV-prefix lock proxy, loop-guarded coding-agent loop, and minimalist web UI (`/`).

## Build & Test

Requires Rust (`cargo`). Zero external crates are used (`Cargo.toml`).

```sh
cd kraits-adaptor
cargo test --release
cargo build --release
```

The compiled binary is written to `target/release/kraits-adaptor`.

## Quick Start (`serve`)

1. Start `llama-server --jinja` on `http://127.0.0.1:8099`.
2. Point `kraits-adaptor` at your local Git repository and open `http://127.0.0.1:8100`:

```sh
kraits-adaptor serve --repo <dir> [--listen 127.0.0.1:8100] [--upstream http://127.0.0.1:8099]
```

| Flag | Default | Description |
| --- | --- | --- |
| `--repo <dir>` | `.` | Path to the single locked target Git repository |
| `--listen <host:port>` | `127.0.0.1:8100` | Local HTTP bind address for UI, `/kraits/*`, and `/v1/chat/completions` |
| `--upstream <url>` | `http://127.0.0.1:8099` | `llama-server` URL |

### Other CLI Subcommands

```sh
kraits-adaptor index  --repo <dir>
kraits-adaptor purge  --repo <dir>
kraits-adaptor ast    --repo <dir> <rel-file>
kraits-adaptor symbol --repo <dir> <rel-file> <symbol>
kraits-adaptor query  --repo <dir> <query> [--budget <chars>]
kraits-adaptor learn  --repo <dir> --kind <kind> --sig <sig> --detail <detail> [--file <rel>]
```

## Tested Configuration (`v0.6.28`)

Tested with:

| Model Tested | Hardware / Context | Planted Single-Function Bugs (`pallets/flask`) | Multi-File Features / Open Issues | Decode Speed |
| --- | --- | --- | --- | --- |
| `Qwen3.8-Flash-Next-IQ4_XS` | 16 GB Mac (`-c 10240`) | **`4/4`** signed (`73–189 s`, `4–9` turns); **`23/26`** across `v0.6.21–v0.6.28` | **`0/7`** (`#6123` / `feat1`) | `16.5–17.7 tok/s` |

## Endpoints & Web UI

| Method & Path | Purpose |
| --- | --- |
| `GET /` or `GET /ui` | Single-column chat & diff web UI |
| `POST /v1/chat/completions` | Chat completions proxy with repo context injection |
| `GET /kraits/repo` | Repo version, indexed AST files, active task ID, and served request count |
| `GET /kraits/file?path=<rel>` | Line count and AST symbol skeleton for an indexed file |
| `GET /kraits/memory` | Persisted `.kraits/` episodic lessons and hot-file counts |
| `GET /kraits/diff` | Current `git diff` in the locked repository |
| `POST /kraits/learn` | Manually record a convention/pitfall lesson into `.kraits/memory.jsonl` |
| `POST /kraits/task` | Start an agent task (`{"prompt": "..."}`) -> returns `{"task_id": "..."}` |
| `GET /kraits/task?id=<task_id>` | Poll task status, turn traces, git diff, and final summary |
| `POST /kraits/demo` | Demo harness (`{"action": "bug1"\|"bug2"\|"feat1"\|"reset"\|"pytest", "target": "..."}`) |

*Note: The `bug1`, `bug2`, and `feat1` demo actions assume a `pallets/flask` checkout.*
