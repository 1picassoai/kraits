# Kraits

![A llama and a striped krait sharing sandwiches on a picnic blanket, beside a basket marked SSD and a Mac mini](docs/kraits-picnic.png)

A drop-in plugin for `llama.cpp` that runs large mixture-of-experts models (such as **`Qwen3.8-Flash-Next-IQ4_XS`**, `91 GiB` across 3 shards) completely offline on a **base `16 GB` Apple Silicon Mac** at **`16K` context (`-c 16384`)**, together with **`kraits-adaptor`** — a zero-dependency AST indexer, KV-prefix lock proxy, and loop-guarded coding-agent harness & web UI.

## Quick Start (macOS arm64 — Recommended Release Bundle)

Download **`kraits-v0.6.29-macos-arm64.tar.gz`** from [Releases (`v0.6.29`)](https://github.com/1picassoai/kraits/releases/tag/v0.6.29) (includes `llama-server b11936`, patched `libggml-metal`, `libggml-kraits.dylib`, `libkraits.dylib`, `kraits-server.sh`, and `kraits-adaptor`):

```sh
tar -xzf kraits-v0.6.29-macos-arm64.tar.gz
cd kraits-v0.6.29-macos-arm64

# 1. Start llama-server with Kraits (16K context, port 8099)
./kraits-server.sh /path/to/Qwen3.8-Flash-Next-IQ4_XS-00001-of-00003.gguf 8099 16384

# 2. Point kraits-adaptor at any local Git repository (Web UI at http://127.0.0.1:8100)
./kraits-adaptor serve --repo /path/to/your/repo --listen 127.0.0.1:8100 --upstream http://127.0.0.1:8099
```

Or build `kraits-adaptor` from source (`cargo test --release && cargo build --release` inside [`kraits-adaptor/`](kraits-adaptor/README.md)).

## Measured Performance (`16 GB` Apple M4, `-c 16384`, `Qwen3.8-Flash-Next-IQ4_XS`)

- **Bare decode:** up to **`18.0 tok/s`** (`17.94–18.24 tok/s` across 256-token runs)
- **Inside multi-turn coding loops (`kraits-adaptor`):** **`15.6–17.9 tok/s`** (`13–14 tok/s` after `12K` prompt depth)
- **Prompt prefill:** **`30 tok/s`** on short first cold prompts (`1.6K`), scaling to **`48–67 tok/s`** (`6–12K`)
- **Memory & cache (`vm.swapusage`):** **`7.20 GB`** wired bank (`56–57` slots/layer out of `512` experts), keeping macOS swap around **`230–370 MB`**
- **Quality:** Runs full 4-bit (`IQ4_XS`) weights (`8/8` Rust benchmark; `4/4` planted Flask unit-test fixes on Windows, `2/4` on clean Mac build; accurate at `6K` pasted-code Q&A, recall degrades past `12K`).
- **Adaptor documentation:** [`kraits-adaptor/README.md`](kraits-adaptor/README.md) · [`BUILD.md`](BUILD.md) · [Kraits Wiki](https://github.com/1picassoai/kraits/wiki)

## Credits

- [llama.cpp and ggml](https://github.com/ggml-org/llama.cpp), by Georgi Gerganov and contributors (MIT).
- [Candle](https://github.com/huggingface/candle), by Hugging Face and contributors (MIT or Apache 2.0).

## Licence

Apache 2.0. See `LICENSE` and `NOTICE`. The models keep their own licences.
