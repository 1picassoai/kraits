# Kraits

![A llama and a striped krait sharing sandwiches on a picnic blanket, beside a basket marked SSD and a Mac mini](docs/kraits-picnic.png)

A plugin for llama.cpp that runs mixture-of-experts models bigger than a 16 GB Mac's GPU can hold.

## New in 0.6.17: Ling-3.0-flash-TS128-Q4_K_M, TypeScript edition

**47/48 on six TypeScript tasks, at about 20 tokens/s on a 16 GB Mac. gpt-5-mini: 46/48.**

Kraits 0.6.17 runs the 20.4 GB `Ling-3.0-flash-TS128-Q4_K_M.gguf` (20,395,487,680 bytes, cut from 512 to 128 experts per MoE layer from InclusionAI's [`inclusionAI/Ling-3.0-flash`](https://huggingface.co/inclusionAI/Ling-3.0-flash)) on a 16 GB Mac for TypeScript coding. It ships its own expert pin list inside the release tarball (`release-0617/Ling-3.0-flash-TS128-Q4_K_M.kraits-pins.txt`), locks a 9.6 GB bank of the hottest experts in wired GPU memory at load, refills the bank's tail slots from each prompt's expert picks, and streams cold experts from the SSD.

Measured by the release tester, Mac mini M4 16 GB, Fri 6 Nov 2026, temperature 0, launcher as shipped: writing 19.6 to 20.7 tokens/s (mean 20.4; writing 19 to 20 tokens/s in whole numbers), reading 37 to 45 tokens/s (with a live `100% -> 0%` progress countdown on `stderr`).

| Task | Ling TS128 + Kraits 0.6.17 | gpt-5-mini |
|---|---:|---:|
| Express route | 7 | 8 |
| Types and generics | 8 | 8 |
| Async error handling | 8 | 8 |
| Jest test suite | 8 | 6 (average of 3 runs: 8, 8, 2) |
| JS to strict TypeScript | 8 | 8 |
| React component | 8 | 8 |
| **Total** | **47/48** | **46/48** |

*The Jest task uses a reworded prompt written by the release tester and unseen by the model's tuning. Ling's answers are identical on every run at temperature 0. gpt-5-mini's answers vary between runs, so its Jest score is the average of 3 runs; its other five scores are single runs from 2 Nov.*

### Download and run

This git repository holds the Kraits plugin libraries (`bin/`), not `llama-server`. To run out of the box, download:
1. **The release bundle (`kraits-v0.6.17-macos-arm64.tar.gz`, which includes the 0.6.17 expert pin list `Ling-3.0-flash-TS128-Q4_K_M.kraits-pins.txt`):** https://github.com/1picassoai/kraits/releases/download/v0.6.17/kraits-v0.6.17-macos-arm64.tar.gz (on the [v0.6.17 release page](https://github.com/1picassoai/kraits/releases/tag/v0.6.17))
2. **The 20.4 GB model (`Ling-3.0-flash-TS128-Q4_K_M.gguf`):** https://huggingface.co/vinnj/Ling-3.0-flash-TypeScript-Edition-GGUF

Unpack the release tarball, copy its `Ling-3.0-flash-TS128-Q4_K_M.kraits-pins.txt` next to `Ling-3.0-flash-TS128-Q4_K_M.gguf`, and run:

```sh
curl -LO https://github.com/1picassoai/kraits/releases/download/v0.6.17/kraits-v0.6.17-macos-arm64.tar.gz
curl -LO https://huggingface.co/vinnj/Ling-3.0-flash-TypeScript-Edition-GGUF/resolve/main/Ling-3.0-flash-TS128-Q4_K_M.gguf
tar -xzf kraits-v0.6.17-macos-arm64.tar.gz
cp ./release-0617/Ling-3.0-flash-TS128-Q4_K_M.kraits-pins.txt .
./release-0617/kraits-server.sh ./Ling-3.0-flash-TS128-Q4_K_M.gguf 8080
```

Check the model file with `shasum -a 256 Ling-3.0-flash-TS128-Q4_K_M.gguf`: it must print `e9ae4335a36ed49e8262add1099459a807692edb18c6252ba855f0923c52f602` (20,395,487,680 bytes). Check the pin list with `shasum -a 256 Ling-3.0-flash-TS128-Q4_K_M.kraits-pins.txt`: it must print `b430f9b004cc6ea5da2f84aba52ab90e7be5c378a4985886d9ba67515b7f5a23` (16,192 bytes).

## Before you start

You need an Apple silicon Mac with 16 GB or more on macOS 15 or later, and the `kraits-v0.6.17-macos-arm64.tar.gz` release bundle (built against llama.cpp `b11936`, ggml-base 0.27.0). Kraits 0.6.17 runs only `Ling-3.0-flash-TS128-Q4_K_M.gguf` (20,395,487,680 bytes) and refuses any other file. It worked if it prints `kraits: 0.6.17`, `encoder bank on...`, and then the `100% -> 0% remaining` countdown when reading a prompt. The server is llama-server's usual OpenAI-compatible API on the chosen port (default 8080), one request at a time. Nothing leaves the Mac. More detail is in `BUILD.md`.

## Credits

- [llama.cpp and ggml](https://github.com/ggml-org/llama.cpp), by Georgi Gerganov and contributors (MIT).
- [Candle](https://github.com/huggingface/candle), by Hugging Face and contributors (MIT or Apache 2.0).
- [Ling-3.0-flash](https://huggingface.co/inclusionAI/Ling-3.0-flash), by InclusionAI, for the base model.

## Licence

Apache 2.0. See `LICENSE` and `NOTICE`. The models keep their own licences.
