# Kraits

![A llama and a striped krait sharing sandwiches on a picnic blanket, beside a basket marked SSD and a Mac mini](docs/kraits-picnic.png)

A plugin for llama.cpp that runs mixture-of-experts models bigger than a 16 GB Mac's GPU can hold.

## New in 0.6.17: Ling-3.0-flash-TS128-Q4_K_M, TypeScript edition

Kraits 0.6.17 runs the 20.4 GB `Ling-3.0-flash-TS128-Q4_K_M.gguf` (20,395,487,680 bytes, 128 experts per MoE layer cut from InclusionAI's [`inclusionAI/Ling-3.0-flash`](https://huggingface.co/inclusionAI/Ling-3.0-flash)) on a 16 GB Mac for TypeScript coding. It ships an expert pin list (`Ling-3.0-flash-TS128-Q4_K_M.kraits-pins.txt`), locks an 8.8 GB bank of the hottest experts in wired GPU memory at load, and streams cold experts from the SSD.

Measured by the release tester on a Mac mini M4 with 16 GB, 4 Nov 2026, temperature 0, with the launcher as shipped: reading 32 to 44 tokens/s (with a live `100% -> 0%` progress countdown on `stderr`), writing 19 to 22 tokens/s. Six judged TypeScript tasks (strict compile plus tests) scored 45 of 48 (7/8 Express, 6/8 Generics, 8/8 Async, 8/8 Jest, 8/8 JS-to-Strict-TS, 8/8 React), beating Gemma 4's 41 of 48 and 2 behind gpt-5-mini's 47 of 48. Both runs gave identical answers.

### Download and run

This git repository holds the Kraits plugin libraries (`bin/`), not `llama-server`. To run out of the box, download:
1. **The release bundle (`kraits-v0.6.17-macos-arm64.tar.gz`):** https://github.com/1picassoai/kraits/releases/download/v0.6.17/kraits-v0.6.17-macos-arm64.tar.gz (on the [v0.6.17 release page](https://github.com/1picassoai/kraits/releases/tag/v0.6.17))
2. **The 20.4 GB model (`Ling-3.0-flash-TS128-Q4_K_M.gguf`) and pin list (`Ling-3.0-flash-TS128-Q4_K_M.kraits-pins.txt`):** https://huggingface.co/vinnj/Ling-3.0-flash-TypeScript-Edition-GGUF

Keep `Ling-3.0-flash-TS128-Q4_K_M.kraits-pins.txt` next to `Ling-3.0-flash-TS128-Q4_K_M.gguf`, unpack the release tarball, and run from the unpacked folder:

```sh
curl -LO https://github.com/1picassoai/kraits/releases/download/v0.6.17/kraits-v0.6.17-macos-arm64.tar.gz
curl -LO https://huggingface.co/vinnj/Ling-3.0-flash-TypeScript-Edition-GGUF/resolve/main/Ling-3.0-flash-TS128-Q4_K_M.gguf
curl -LO https://huggingface.co/vinnj/Ling-3.0-flash-TypeScript-Edition-GGUF/resolve/main/Ling-3.0-flash-TS128-Q4_K_M.kraits-pins.txt
tar -xzf kraits-v0.6.17-macos-arm64.tar.gz
./release-0617/kraits-server.sh ./Ling-3.0-flash-TS128-Q4_K_M.gguf 8080
```

Check the model file with `shasum -a 256 Ling-3.0-flash-TS128-Q4_K_M.gguf`: it must print `e9ae4335a36ed49e8262add1099459a807692edb18c6252ba855f0923c52f602` (20,395,487,680 bytes).

## Earlier releases (0.3.0): Gemma 4 26B-A4B and Qwen3.6-35B-A3B

In Kraits 0.3.0 (`git checkout v0.3.0`), Kraits ran Google's 14.4 GB `gemma-4-26B_q4_0-it.gguf` (https://huggingface.co/google/gemma-4-26B-A4B-it-qat-q4_0-gguf; reading 105 tokens/s, writing 19 to 22 tokens/s, 33 of 40 on five TypeScript tasks) and Unsloth's 22 GB `Qwen3.6-35B-A3B-UD-Q4_K_M.gguf` (https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF; about 9.5 tokens/s writing in exact mode) against llama.cpp build `b11715`. Each Kraits release is locked to its tested model file.

## Before you start

You need an Apple silicon Mac with 16 GB or more on macOS 15 or later, and the `kraits-v0.6.17-macos-arm64.tar.gz` release bundle (built against llama.cpp `b11936`, ggml-base 0.27.0). Kraits 0.6.17 runs only `Ling-3.0-flash-TS128-Q4_K_M.gguf` (20,395,487,680 bytes) and refuses any other file. It worked if it prints `kraits: 0.6.17`, `encoder bank on...`, and then the `100% -> 0% remaining` countdown when reading a prompt. The server is llama-server's usual OpenAI-compatible API on the chosen port (default 8080), one request at a time. Nothing leaves the Mac. More detail is in `BUILD.md`.

## Credits

- [llama.cpp and ggml](https://github.com/ggml-org/llama.cpp), by Georgi Gerganov and contributors (MIT).
- [Candle](https://github.com/huggingface/candle), by Hugging Face and contributors (MIT or Apache 2.0).
- [Ling-3.0-flash](https://huggingface.co/inclusionAI/Ling-3.0-flash), by InclusionAI, for the base model.
- [Gemma](https://huggingface.co/google/gemma-4-26B-A4B-it-qat-q4_0-gguf), by Google DeepMind, for Gemma 4 26B-A4B.
- [Qwen](https://huggingface.co/Qwen), for Qwen3.6-35B-A3B.
- [Unsloth](https://huggingface.co/unsloth), for the UD-Q4_K_M file.

## Licence

Apache 2.0. See `LICENSE` and `NOTICE`. The models keep their own licences.
