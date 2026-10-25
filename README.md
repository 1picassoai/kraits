# Kraits

![A llama and a striped krait sharing sandwiches on a picnic blanket, beside a basket marked SSD and a Mac mini](docs/kraits-picnic.png)

A plugin for stock llama.cpp that runs mixture-of-experts models bigger than a 16 GB Mac's GPU can hold. It changes nothing in llama.cpp and nothing in the model file.

## New in 0.3.0: Gemma 4 26B-A4B, TypeScript edition

Kraits runs Google's own 14.4 GB Gemma 4 file on a 16 GB Mac, for TypeScript coding. It ships a heat map of the experts each layer uses most on TypeScript work, and puts the hottest 80 per layer on the GPU at load, so nothing is read from the SSD while it writes.

Measured by the release tester on a Mac mini M4 with 16 GB, 25 Oct 2026, thinking off, temperature 0: reading 105 tokens/s, writing 31 to 33 tokens/s. Five judged TypeScript tasks (strict compile plus tests) scored 33 of 40; the weakest, a Jest test suite, scored 2 of 8. The same prompt twice gave byte-identical answers.

```sh
cd kraits/bin
./kraits-server -m /path/to/gemma-4-26B_q4_0-it.gguf -c 8192 --reasoning off
```

The model is `gemma-4-26B_q4_0-it.gguf` (14,439,363,584 bytes) from https://huggingface.co/google/gemma-4-26B-A4B-it-qat-q4_0-gguf. For chats past 8K tokens, start with `KRAITS_HOT_N=64` and `-c 24576`: it fits more chat memory but keeps fewer experts, and quality drops on long coding tasks.

## Qwen3.6-35B-A3B

Kraits runs Unsloth's 22 GB `Qwen3.6-35B-A3B-UD-Q4_K_M.gguf` (22,134,528,992 bytes, https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF) by streaming experts from the SSD. By default it waits for every expert, so the same prompt gives the same answer every time: about 9.5 tokens/s writing, and 3 of 3 runs of an LRU-cache coding prompt passed every check (release tester, 21 Oct 2026). `KRAITS_FAST=1` writes about twice as fast but skips late experts, so answers vary and about 1 in 4 or 5 has a one-token slip.

```sh
cd kraits/bin
./kraits-server -m /path/to/Qwen3.6-35B-A3B-UD-Q4_K_M.gguf -c 16384 \
  --chat-template-kwargs '{"enable_thinking":false}'
```

## Before you start

You need an Apple silicon Mac with 16 GB or more on macOS 15 or later, and llama.cpp build b11715 (`llama-b11715-bin-macos-arm64.tar.gz` from https://github.com/ggml-org/llama.cpp/releases/tag/b11715; set `LLAMA_SERVER=/path/to/llama-server`). Other builds are not tested with this release. Kraits runs only the two files above and refuses any other. It worked if it prints `Kraits loaded`. The server is llama-server's usual OpenAI-compatible API on port 8080, one request at a time. Nothing leaves the Mac. More detail is in `BUILD.md`.

## Credits

- [llama.cpp and ggml](https://github.com/ggml-org/llama.cpp), by Georgi Gerganov and contributors (MIT).
- [Candle](https://github.com/huggingface/candle), by Hugging Face and contributors (MIT or Apache 2.0).
- [Gemma](https://huggingface.co/google/gemma-4-26B-A4B-it-qat-q4_0-gguf), by Google DeepMind, for Gemma 4 26B-A4B and its 4-bit file.
- [Qwen](https://huggingface.co/Qwen), for Qwen3.6-35B-A3B.
- [Unsloth](https://huggingface.co/unsloth), for the UD-Q4_K_M file.

## Licence

Apache 2.0. See `LICENSE` and `NOTICE`. The models keep their own licences.
