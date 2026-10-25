# Kraits 0.3.0 — what is in bin/ and where it came from

| file | bytes | sha256 |
|---|---|---|
| bin/libggml-kraits.dylib (the llama.cpp add-in) | 74,128 | 8fcc3cae44d8de20e12b7268cc581f2603ff08f0564caa77992336cb7e21380f |
| bin/libkraits.dylib (the Kraits engine) | 9,913,648 | 967085d4fe8cd83993c1bc88a1194316e722912ed31d10d5661f1d569ff5b27c |
| bin/kraits-server (the launcher, a shell script) | 2,494 | ed4ba7173c88511a7b97798dc2683b4a5fe854d22e1ee891fe2109a162b25cdf |
| bin/gemma4-26b-a4b-heat.txt (Gemma 4's heat map) | 7,600 | faf2434846bc3d3f1adcdb90736e2efb8f9af50d37767e78cbbad21b05ca2719 |

Built on a Mac mini M4 (16 GB), macOS 15 or later required, Apple silicon only. Ad-hoc signed. Build paths are remapped, so no folder of the building Mac is inside the binaries. The plugin finds the engine library beside itself; ggml-base beside llama-server or in /opt/homebrew/lib. Built against llama.cpp `b11715` (commit `9d1878a`, 17 Oct 2026; ggml-base 0.27.0).

## The model files

Kraits runs two files and refuses any other, checking each file's size and two fingerprints of its weights:

- Google's `gemma-4-26B_q4_0-it.gguf` (https://huggingface.co/google/gemma-4-26B-A4B-it-qat-q4_0-gguf), exactly 14,439,363,584 bytes.
- Unsloth's `Qwen3.6-35B-A3B-UD-Q4_K_M.gguf` (https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF), exactly 22,134,528,992 bytes.

## The heat map

`gemma4-26b-a4b-heat.txt` has one line per layer: the layer number, then that layer's 80 hottest experts of 128, hottest first, counted from Gemma's own routing on TypeScript coding prompts. At load Kraits puts those 80 on the GPU (`KRAITS_HOT_N` sets a smaller number) and the router chooses among them. The launcher points `KRAITS_HOT` at this file; Gemma refuses to start without it.

## The short way: the launcher

`bin/kraits-server` sets the plugin path from its own folder, checks that llama.cpp listed KRAITS, stops with exit 1 and the reason if not, and otherwise prints `Kraits loaded` and starts `llama-server` with the required flags (`--device KRAITS -ngl 99 -ot "token_embd=KRAITS" -np 1 --cache-ram 0`). Kraits serves one request at a time and keeps the conversation itself.

```sh
cd kraits/bin
LLAMA_SERVER=/path/to/llama-server ./kraits-server -m gemma-4-26B_q4_0-it.gguf -c 8192 --reasoning off
```

`LLAMA_SERVER` can be left out when `llama-server` is on your PATH. If the script is not executable after cloning, `chmod +x kraits-server`.

For Qwen the launcher's default is the exact mode, which waits for every expert streamed from the SSD; `KRAITS_FAST=1` turns on the fast mode, which does not. Gemma does not stream while writing, so the mode does not apply to it.

## Which llama.cpp

The plugin links `libggml-base` ABI 0.27 and is tested with llama.cpp **b11715** (commit `9d1878a`, 17 Oct 2026). Homebrew's llama.cpp 0.7.0 loaded the 0.1 plugin but is not tested with this release. Other builds are not tested and may not load it. An older llama.cpp (for example b11220, ggml 0.25) does not load it; `--list-devices` will not show KRAITS.

The README carries the release tester's numbers.
