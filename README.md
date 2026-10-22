# Kraits

![A llama and a striped krait sharing sandwiches on a picnic blanket, beside a basket marked SSD and a Mac mini](docs/kraits-picnic.png)

An open-source plugin for stock llama.cpp that runs Unsloth's Qwen3.6-35B-A3B 4-bit file (22 GB) on a 16 GB Mac. It changes nothing in llama.cpp and nothing in the model file. The engine inside it is [Coachwhip](https://github.com/1picassoai/coachwhip), Apache 2.0, compiled here as a llama.cpp plugin.

## Numbers

Measured by the release tester on a Mac mini M4 with 16 GB, using the shipped files and the official llama.cpp b11715 prebuilt, on 20 Oct 2026, in the default fast mode (see Two modes below). The workload is a four-agent coding crew at three conversation sizes, run at temperature 0 with thinking off. Speeds are llama-server's own counts, as the range across the agents' turns. The score is the crew app's judge running the code as written.

| Conversation size | Reading (tokens/s) | Writing (tokens/s) | Code score |
|---|---|---|---|
| ~1,000 tokens | 30.4 – 49.3 | 18.2 – 23.0 | 6/8 |
| ~8,000 tokens | 36.7 – 48.4 | 15.8 – 22.5 | 8/8 |
| ~16,000 tokens | 37.8 – 47.3 | 16.0 – 23.8 | 9/9 |

One run per size, in the fast mode, which varies from run to run (see Two modes). Treat each score as one roll, not an average.

## Two modes

Kraits streams the model's experts from the SSD. The default **fast** mode does not wait for experts that arrive late. The **exact** mode waits for every one, so the same prompt gives the same answer every time. Measured 21 Oct 2026 on the same Mac, one coding prompt (an LRU cache with expiry, temperature 0, thinking off), executed checks:

| Mode | How to run | Writing (tokens/s) | Result |
|---|---|---|---|
| Fast (default in this release) | as below | 19.0 – 19.8 end to end (23.5 generation only) | Not deterministic: three identical runs gave three different answers. 1 of 3 (tester) and 3 of 14 (developer) runs had a one-token slip (for example `tttl_ms` for `ttl_ms`) that broke the code without any error. |
| Exact | `KRAITS_NOWAIT=0` in the environment before `kraits-server` | 9.2 – 9.7 end to end | Every run passed every check, and the answers were byte-identical: 5 of 5 runs (developer), 3 of 3 (release tester), and the same answer again from a different build on a different day. |

If a wrong token is worse than waiting, run the exact mode. The next release makes it the default.

## Prerequisites

- A Mac with Apple silicon and 16 GB of memory or more, on macOS 15 or later.
- llama.cpp. Either build **b11715** (`llama-b11715-bin-macos-arm64.tar.gz` from https://github.com/ggml-org/llama.cpp/releases/tag/b11715) or Homebrew's llama.cpp (`brew install llama.cpp`, version 0.7.0 at the time of writing). Both loaded the plugin on the tested Mac. Other builds are not tested, and an older one (b11220) was refused with a message naming the build Kraits needs.
- The model file `Qwen3.6-35B-A3B-UD-Q4_K_M.gguf` (22,134,528,992 bytes) from https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF. Kraits runs this one file and refuses any other.

## Install and run

1. Get llama.cpp: either unpack the b11715 archive and keep its files together in one folder, or `brew install llama.cpp`.
2. Clone this repo.
3. Start the server. With Homebrew's llama.cpp:

```sh
cd kraits/bin
./kraits-server -m /path/to/Qwen3.6-35B-A3B-UD-Q4_K_M.gguf -c 16384 \
  --chat-template-kwargs '{"enable_thinking":false}'
```

With the b11715 build, point `kraits-server` at it:

```sh
cd kraits/bin
LLAMA_SERVER=/path/to/llama-b11715/llama-server ./kraits-server \
  -m /path/to/Qwen3.6-35B-A3B-UD-Q4_K_M.gguf -c 16384 \
  --chat-template-kwargs '{"enable_thinking":false}'
```

It worked if it prints `Kraits loaded`. If the plugin did not load, it stops and says why. The server is llama-server's usual OpenAI-compatible API on port 8080, and it answers one request at a time. More detail is in `BUILD.md`.

## Guides

What do you want to do?

- **Run a crew of agents (CrewAI).** Point CrewAI's `LLM` at `http://127.0.0.1:8080/v1` with any API key and the model name the server reports. That is how every number above was measured, including a four-agent crew and a tool-calling agent with twelve tools.

Other apps will be listed here only after they have been run against Kraits.

## FAQ

Every answer here comes from a measurement on the tested Mac mini M4 (16 GB) on 21 Oct 2026, or from the code.

**Is it open source?** Yes. The engine is [Coachwhip](https://github.com/1picassoai/coachwhip) (Rust, Metal kernels, server), Apache 2.0. This repo ships it compiled as a llama.cpp plugin, with the launcher script. The small glue crate that turns the engine into a ggml backend is being added to the Coachwhip repo with its build steps.

**Does it change llama.cpp or the model file?** No. Kraits is a separate library that llama.cpp loads at start-up, because the `kraits-server` launcher tells it where the library is. Your llama.cpp files are not touched, and llama.cpp run without the launcher is plain llama.cpp. The model file is only read: the server holds it open read-only during a run, and Kraits checks its size and a fingerprint of its weights.

**Which llama.cpp builds work?** Tested: build b11715 (our own build and the official macOS prebuilt) and Homebrew's llama.cpp 0.7.0, both on ggml 0.27. An older build (b11220, ggml 0.25) is refused with a message naming what Kraits needs.

**Which model, and why only one?** Unsloth's `Qwen3.6-35B-A3B-UD-Q4_K_M.gguf`. Kraits plans its memory around one file's layout, deciding which parts of the model stay in memory and which are read from the SSD, and every number in this README was measured on that file. A different file would run with numbers nobody has measured, so Kraits refuses it, checking the size and a fingerprint of the weights rather than the name.

**Why does the first answer take a few seconds?** Two waits. Start-up: loading took about 15 to 33 s before the server answered. Then reading your prompt, at 30 to 49 tokens a second: a 230-token prompt takes about 8 s, a 12,330-token conversation about 260 s. A follow-up on the latest answer reads only the new words: 2.3 s for a short reply.

**How much memory does it use?** About 9.3 GB of the Mac's 16 GB as macOS counts it (the server's resident size), the same at idle and at the peak of a run, sampled every 2 s through a full run (developer, 21 Oct).

**Does it work offline?** In a full run, sampled every 2 s with the Mac online, the server opened no network connection other than the local port your app talks to (release tester, 22 Oct, 72 samples over two runs). Not yet tested with the network physically switched off.

**Does anything leave the Mac?** No connection was seen: the socket sampling above (release tester, 22 Oct) and the developer's own sampling on 21 Oct both found only the local port. The release tester's scan of both binaries found no hostnames other than the project's issues page.

## Credits

- [Coachwhip](https://github.com/1picassoai/coachwhip), the engine inside Kraits (Apache 2.0).
- [llama.cpp and ggml](https://github.com/ggml-org/llama.cpp), by Georgi Gerganov and contributors (MIT). They supply the server, the tokenizer and the sampling Kraits plugs into.
- [Candle](https://github.com/huggingface/candle), by Hugging Face and contributors (MIT or Apache 2.0). It's the tensor library under the engine.
- [Qwen](https://huggingface.co/Qwen), for the Qwen3.6-35B-A3B model.
- [Unsloth](https://huggingface.co/unsloth), for the UD-Q4_K_M file Kraits runs.
- The open-source Rust crates the engine is built with.

## Licence

Apache 2.0. See `LICENSE` and `NOTICE`.
