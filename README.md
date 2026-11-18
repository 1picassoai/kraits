# Kraits

![A llama and a striped krait sharing sandwiches on a picnic blanket, beside a basket marked SSD and a Mac mini](docs/kraits-picnic.png)

A plugin for `llama.cpp` that runs mixture-of-experts models bigger than a 16 GB Mac's GPU can hold, together with **`kraits-adaptor`** — a zero-dependency AST indexer, KV-prefix lock proxy, and loop-guarded coding-agent harness.

## Kraits Repo Adaptor (`v0.6.28` — `kraits-adaptor/`)

Zero-dependency local harness that sits in front of `llama-server` (`llama.cpp`, `--jinja`) to stop coding-agent loops on real repositories:

- **Build & Run:** See [`kraits-adaptor/README.md`](kraits-adaptor/README.md) (`cargo test --release`, `cargo build --release`, `kraits-adaptor serve --repo <dir> --listen 127.0.0.1:8100 --upstream http://127.0.0.1:8099`).
- **Loop & Hallucination Gates:** AST symbol slicing (`ast_symbol`), KV-prefix lock (`cache_n = 988` on Turn 1), pre-tool path/symbol/command validation, `64`-token no-tool stream cutoff, cached repeat replies, auto-`pytest` verification after 2 edits, and summary quality gates.
- **Tested Benchmark (`pallets/flask`, 83 files, `Qwen3.8-Flash-Next-IQ4_XS` on 16 GB Mac `-c 10240`, `16.5–17.7 tok/s`):** **`4/4`** planted single-function Python bugs fixed & `pytest`-verified on signed `v0.6.28` (**`73–189 s`**, `4–9` turns), **`23/26`** across `v0.6.21–v0.6.28`; multi-file features and real open issues (`#6123`) remain **`0/7`**.

## macOS Engine Plugin (`bin/` & `BUILD.md`)

Prebuilt Apple Silicon dylibs and `llama.cpp` build notes live in `bin/` and [`BUILD.md`](BUILD.md) (previous macOS release bundle: [v0.6.17](https://github.com/1picassoai/kraits/releases/tag/v0.6.17)). Runs locally via `llama-server` OpenAI-compatible HTTP API with nothing leaving the machine.

## Credits

- [llama.cpp and ggml](https://github.com/ggml-org/llama.cpp), by Georgi Gerganov and contributors (MIT).
- [Candle](https://github.com/huggingface/candle), by Hugging Face and contributors (MIT or Apache 2.0).

## Licence

Apache 2.0. See `LICENSE` and `NOTICE`. The models keep their own licences.
