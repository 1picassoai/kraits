# Kraits

A plugin that makes stock llama.cpp run the stock Unsloth Qwen3.6-35B-A3B 4-bit file 8.7x faster on a 16 GB Mac. It changes nothing in llama.cpp and nothing in the model file.

| Same Mac, same file, same task | Writing speed | Judge |
|---|---|---|
| llama.cpp | 2.91 words/s | 8/8 |
| llama.cpp + Kraits | 25.18 words/s | 8/8 |

The judge is eight executed checks on the TypeScript LRU cache the model writes (temperature 0, thinking off).

## Quick start

```sh
export GGML_BACKEND_PATH=/path/to/kraits/bin/libggml-kraits.dylib
llama-server -m Qwen3.6-35B-A3B-UD-Q4_K_M.gguf \
  --device KRAITS -ngl 99 -ot "token_embd=KRAITS"
```

Keep `libkraits.dylib` in the same folder as `libggml-kraits.dylib`.

## How it works

llama.cpp keeps its server, its tokenizer and its sampling. Kraits runs the model's layers itself. The model is 22 GB and the Mac has 16 GB, so Kraits keeps the experts each word is most likely to need on the GPU and streams the rest from the SSD as it writes.

## Tested setup

- Mac mini M4, 16 GB, macOS 26.6. Needs macOS 15 or later on Apple Silicon.
- llama.cpp b11715.
- Qwen3.6-35B-A3B UD-Q4_K_M from Unsloth (22.13 GB).

## Licence

Apache 2.0. See `LICENSE` and `NOTICE`.
