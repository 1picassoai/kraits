# Kraits

A plugin that makes stock llama.cpp run the stock Unsloth Qwen3.6-35B-A3B 4-bit file faster on a 16 GB Mac. It changes nothing in llama.cpp and nothing in the model file.

| Same Mac, same file, same task | Writing speed | Judge |
|---|---|---|
| llama.cpp | (tester's number, pending) | (pending) |
| llama.cpp + Kraits | (tester's number, pending) | (pending) |

The numbers in this table are measured by the release tester, not by the developers. The judge is eight executed checks on the TypeScript LRU cache the model writes (temperature 0, thinking off).

Both rows were measured on the same Mac mini M4 (16 GB), the same file, the same task over `/v1/chat/completions`, temperature 0, thinking off. The llama.cpp row ran `llama-server -m Qwen3.6-35B-A3B-UD-Q4_K_M.gguf -ngl 99 -ncmoe 40 -t 4 -c 8448` (experts on the CPU side); the Kraits row ran `llama-server -m Qwen3.6-35B-A3B-UD-Q4_K_M.gguf --device KRAITS -ngl 99 -ot "token_embd=KRAITS" -c 8448 -np 1` with `GGML_BACKEND_PATH` set as in the quick start.

## Quick start

```sh
cd kraits/bin
LLAMA_SERVER=/path/to/llama-server ./kraits-server -m Qwen3.6-35B-A3B-UD-Q4_K_M.gguf -c 8448 -np 1
```

`kraits-server` checks that llama.cpp really loaded the plugin and refuses to start if it did not, so you cannot end up with an ordinary llama.cpp wearing the name. It prints `Kraits loaded` and then runs `llama-server --device KRAITS -ngl 99 -ot "token_embd=KRAITS"` with your flags. Keep the three files in `bin/` together. The by-hand commands, and the exact llama.cpp build this is tested with, are in `BUILD.md`.

## How it works

llama.cpp keeps its server, its tokenizer and its sampling. Kraits runs the model's layers itself. The model is 22 GB and the Mac has 16 GB, so Kraits keeps the experts each word is most likely to need on the GPU and streams the rest from the SSD as it writes.

## Tested setup

- Mac mini M4, 16 GB, macOS 26.6. Needs macOS 15 or later on Apple Silicon.
- llama.cpp b11715 (commit `9d1878a`, 17 Oct 2026). Other builds are not tested and may not load the plugin.
- Qwen3.6-35B-A3B UD-Q4_K_M from Unsloth (22.13 GB).

## Licence

Apache 2.0. See `LICENSE` and `NOTICE`.
