# Kraits

A plugin that makes stock llama.cpp run the stock Unsloth Qwen3.6-35B-A3B 4-bit file faster on a 16 GB Mac. It changes nothing in llama.cpp and nothing in the model file.

Measured on the tested Mac (20 Oct 2026, the shipped files): one prompt of real code plus a coding task, at three sizes. Reading and writing speeds are the server's own counts, in tokens per second; the score is eight executed checks on the code the model writes.

| Prompt size | Reading (tokens/s) | Writing (tokens/s) | Code score |
|---|---|---|---|
| ~1,000 tokens | 45.7 | 22.3 | 8/8 |
| ~8,000 tokens | 48.6 | 19.9 | 8/8 |
| ~16,000 tokens | 47.5 | 17.6 | 8/8 |

In these runs the model left out an `export` keyword the task asked for; the scores are with it added. Temperature 0, thinking off, one run per size. Plain llama.cpp on the same Mac and file writes at about 2 tokens/s (see `BUILD.md` for both commands).

## Requirements

- A Mac with Apple silicon and 16 GB of memory or more, on macOS 15 or later.
- llama.cpp build b11715 (commit `9d1878a`). Newer builds: see "Tested setup" for which have been checked; an untested build may refuse to load Kraits, and the launcher then says so.
- The model file: Unsloth's `Qwen3.6-35B-A3B-UD-Q4_K_M.gguf`, https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF (22,134,528,992 bytes). Kraits runs this one file only.

## Quick start

```sh
cd kraits/bin
LLAMA_SERVER=/path/to/llama-server ./kraits-server -m Qwen3.6-35B-A3B-UD-Q4_K_M.gguf -c 8448
```

`kraits-server` checks that llama.cpp really loaded the plugin and refuses to start if it did not, so you cannot end up with an ordinary llama.cpp wearing the name. It prints `Kraits loaded` and then runs `llama-server --device KRAITS -ngl 99 -ot "token_embd=KRAITS"` with your flags. Keep the three files in `bin/` together. Kraits serves one request at a time (the launcher sets `-np 1`); a second request waits its turn.

**Follow-ups.** A follow-up on the latest answer continues from where that answer stopped. A follow-up on an older answer, or a conversation the server has not seen, is read in first, as plain llama.cpp would read it. The launcher turns the server's own prompt cache off (`--cache-ram 0`) so a follow-up always continues the right conversation; without that flag Kraits refuses to start. The by-hand commands, and the exact llama.cpp build this is tested with, are in `BUILD.md`.

## How it works

llama.cpp keeps its server, its tokenizer and its sampling. Kraits runs the model's layers itself. The model is 22 GB and the Mac has 16 GB, so Kraits keeps the experts each word is most likely to need on the GPU and streams the rest from the SSD as it writes.

## Tested setup

- Mac mini M4, 16 GB, macOS 26.6. Needs macOS 15 or later on Apple Silicon.
- llama.cpp b11715 (commit `9d1878a`, 17 Oct 2026). Other builds are not tested and may not load the plugin.
- The model: Unsloth's Qwen3.6-35B-A3B UD-Q4_K_M, `Qwen3.6-35B-A3B-UD-Q4_K_M.gguf` (22,134,528,992 bytes), https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF. Kraits runs this one file; any other file is refused with a one-line message.

## Licence

Apache 2.0. See `LICENSE` and `NOTICE`.
