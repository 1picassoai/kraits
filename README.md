# Kraits

![A llama and a striped krait sharing sandwiches on a picnic blanket, beside a basket marked SSD and a Mac mini](docs/kraits-picnic.png)

A plugin that makes stock llama.cpp run the stock Unsloth Qwen3.6-35B-A3B 4-bit file faster on a 16 GB Mac. It changes nothing in llama.cpp and nothing in the model file.

Measured by the release tester on the tested Mac (20 Oct 2026, the shipped files, the official llama.cpp b11715 prebuilt). The workload is a four-agent coding crew whose conversation grows to three sizes. Reading and writing speeds are llama-server's own counts in tokens per second, given as the range across the agents' turns. The score is the crew app's own judge running the code the model wrote, scored as written.

| Conversation size | Reading (tokens/s) | Writing (tokens/s) | Code score |
|---|---|---|---|
| ~1,000 tokens | 30.4 – 49.3 | 18.2 – 23.0 | 6/8 |
| ~8,000 tokens | 36.7 – 48.4 | 15.8 – 22.5 | 8/8 |
| ~16,000 tokens | 37.8 – 47.3 | 16.0 – 23.8 | 9/9 |

Reading does not slow down as the conversation grows: 8,641 tokens read at 48.4 tokens/s, and 12,330 tokens at 47.3. Settings: `-c 16384`, temperature 0, thinking off (`--chat-template-kwargs '{"enable_thinking":false}'`, passed to `kraits-server` like any llama-server flag), one run per size.

## Requirements

- A Mac with Apple silicon and 16 GB of memory or more, on macOS 15 or later.
- llama.cpp build b11715 (commit `9d1878a`). The official prebuilt for Mac works as it comes: `llama-b11715-bin-macos-arm64.tar.gz` from https://github.com/ggml-org/llama.cpp/releases/tag/b11715. Keep `llama-server` and its `libggml-*.dylib` files together in the folder it unpacks to. Homebrew's llama.cpp is a different build and is not tested. An older build (b11220) was refused with a message naming the build it needs.
- The model file: Unsloth's `Qwen3.6-35B-A3B-UD-Q4_K_M.gguf`, https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF (22,134,528,992 bytes). Kraits runs this one file only.

## Quick start

```sh
cd kraits/bin
LLAMA_SERVER=/path/to/llama-b11715/llama-server ./kraits-server -m Qwen3.6-35B-A3B-UD-Q4_K_M.gguf -c 8448
```

`kraits-server` checks that llama.cpp really loaded the plugin and refuses to start if it did not, so you cannot end up with an ordinary llama.cpp wearing the name. It prints `Kraits loaded` and then runs `llama-server --device KRAITS -ngl 99 -ot "token_embd=KRAITS"` with your flags. Keep the three files in `bin/` together. Kraits serves one request at a time (the launcher sets `-np 1`); a second request waits its turn.

**Follow-ups.** A follow-up on the latest answer continues from where that answer stopped. A follow-up on an older answer, or a conversation the server has not seen, is read in first, as plain llama.cpp would read it. The launcher turns the server's own prompt cache off (`--cache-ram 0`) so a follow-up always continues the right conversation; without that flag Kraits refuses to start. The by-hand commands, and the exact llama.cpp build this is tested with, are in `BUILD.md`.

## How it works

llama.cpp keeps its server, its tokenizer and its sampling. Kraits runs the model's layers itself. The model is 22 GB and the Mac has 16 GB, so Kraits keeps the experts each word is most likely to need on the GPU and streams the rest from the SSD as it writes.

## Tested setup

- Mac mini M4, 16 GB, macOS 26.6. Needs macOS 15 or later on Apple Silicon.
- llama.cpp b11715 (commit `9d1878a`, 17 Oct 2026): our own build, and the official macOS arm64 prebuilt (tested by the release tester, 20 Oct). Other builds are not tested and may not load the plugin.
- The model: Unsloth's Qwen3.6-35B-A3B UD-Q4_K_M, `Qwen3.6-35B-A3B-UD-Q4_K_M.gguf` (22,134,528,992 bytes), https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF. Kraits runs this one file; any other file is refused with a one-line message.

## Licence

Apache 2.0. See `LICENSE` and `NOTICE`.
