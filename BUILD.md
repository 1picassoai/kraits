# Kraits 0.6.29 — Engine Binaries & Launcher (`bin/`)

| File | Bytes | SHA-256 |
|---|---:|---|
| `bin/libggml-kraits.dylib` (Kraits ggml backend plugin) | 322,720 | `541b214fbb38f0ac96c5e618323582e7fa6ee6d56a6f8c3007e662ae0f38acfe` |
| `bin/libkraits.dylib` (Kraits async expert-bank engine) | 10,609,280 | `2af02dbb788be1732305591ea69d02f9cea681af4c5d6af48369818e957a6a8d` |
| `bin/libggml-metal.0.27.0.dylib` (patched Metal encoder hook) | 5,575,072 | `60948277a884b08d51e72ff1d4ce8e08caba29d4b5d6162119522250cd2428df` |
| `bin/kraits-server.sh` (launcher for `Qwen3.8-Flash-Next-IQ4_XS`) | — | — |

Built on Apple Silicon (`macOS` arm64, ad-hoc signed, `@loader_path` clean with no local developer home paths baked in) against `llama.cpp` `b11936` (`ggml-base 0.27.0`).

## Tested Model

- **`Qwen3.8-Flash-Next-IQ4_XS`** (3-shard GGUF, `90.97 GiB` on disk — `176.9 B` total params: `125B` MoE + `51.2B` n-gram table, `48` layers, `512` experts).
- Kraits pins a **`7.20 GB`** wired bank (`56–57` experts/layer) in unified memory and promotes hot misses asynchronously off NVMe (`KRAITS_ASYNC_IQ4=1`, zero static `.kraits-pins.txt` required).

## Quick Launch (Release Archive Recommended)

This git repo stores the Kraits engine libraries and launcher script, not `llama-server` (`b11936`). Download **`kraits-v0.6.29-macos-arm64.tar.gz`** from [Releases (`v0.6.29`)](https://github.com/1picassoai/kraits/releases/tag/v0.6.29) (bundles `llama-server`, `libllama`/`libggml` shared libs, Kraits `0.6.29` dylibs, `kraits-server.sh`, and `kraits-adaptor`):

```sh
tar -xzf kraits-v0.6.29-macos-arm64.tar.gz
cd kraits-v0.6.29-macos-arm64
./kraits-server.sh /path/to/Qwen3.8-Flash-Next-IQ4_XS-00001-of-00003.gguf 8099 16384
```
