# Kraits

![A llama and a striped krait sharing sandwiches on a picnic blanket, beside a basket marked SSD and a Mac mini](docs/kraits-picnic.png)

A plugin for stock llama.cpp that runs Unsloth's Qwen3.6-35B-A3B 4-bit file (22 GB) on a 16 GB Mac. It changes nothing in llama.cpp and nothing in the model file.

## Numbers

Measured by the release tester on a Mac mini M4 with 16 GB, using the shipped files and the official llama.cpp b11715 prebuilt, on 20 Oct 2026. The workload is a four-agent coding crew at three conversation sizes, run at temperature 0 with thinking off. Speeds are llama-server's own counts, as the range across the agents' turns. The score is the crew app's judge running the code as written.

| Conversation size | Reading (tokens/s) | Writing (tokens/s) | Code score |
|---|---|---|---|
| ~1,000 tokens | 30.4 – 49.3 | 18.2 – 23.0 | 6/8 |
| ~8,000 tokens | 36.7 – 48.4 | 15.8 – 22.5 | 8/8 |
| ~16,000 tokens | 37.8 – 47.3 | 16.0 – 23.8 | 9/9 |

## Prerequisites

- A Mac with Apple silicon and 16 GB of memory or more, on macOS 15 or later.
- llama.cpp build **b11715**: `llama-b11715-bin-macos-arm64.tar.gz` from https://github.com/ggml-org/llama.cpp/releases/tag/b11715. Other builds, Homebrew's included, are not tested.
- The model file `Qwen3.6-35B-A3B-UD-Q4_K_M.gguf` (22,134,528,992 bytes) from https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF. Kraits runs this one file and refuses any other.

## Install and run

1. Unpack the llama.cpp archive and keep its files together in one folder.
2. Clone this repo.
3. Start the server:

```sh
cd kraits/bin
LLAMA_SERVER=/path/to/llama-b11715/llama-server ./kraits-server \
  -m /path/to/Qwen3.6-35B-A3B-UD-Q4_K_M.gguf -c 16384 \
  --chat-template-kwargs '{"enable_thinking":false}'
```

It worked if it prints `Kraits loaded`. If the plugin did not load, it stops and says why. The server is llama-server's usual OpenAI-compatible API on port 8080, and it answers one request at a time. More detail is in `BUILD.md`.

## Credits

- [llama.cpp and ggml](https://github.com/ggml-org/llama.cpp), by Georgi Gerganov and contributors (MIT). They supply the server, the tokenizer and the sampling Kraits plugs into.
- [Candle](https://github.com/huggingface/candle), by Hugging Face and contributors (MIT or Apache 2.0). It's the tensor library under the Kraits engine.
- [Qwen](https://huggingface.co/Qwen), for the Qwen3.6-35B-A3B model.
- [Unsloth](https://huggingface.co/unsloth), for the UD-Q4_K_M file Kraits runs.
- The open-source Rust crates the engine is built with.

## Licence

Apache 2.0. See `LICENSE` and `NOTICE`.
