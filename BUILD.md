# Kraits 0.1.0 — what is in bin/ and where it came from

| file | bytes | sha256 |
|---|---|---|
| bin/libggml-kraits.dylib (the llama.cpp add-in) | 73,392 | 6e1a521f5a5d977b4b7a3a66049246d7e5bf971570735e2ba54d328419d1ce8e |
| bin/libkraits.dylib (the engine behind it) | 9,313,776 | 5986e43ee7dbf4df685952bf24e7c5ddda953c6392fdd400e352bf41ad8210ab |

Built on a Mac mini M4 (16 GB), macOS 15 or later required, Apple silicon only. Ad-hoc signed. The plugin finds the engine beside itself; ggml-base beside llama-server or in /opt/homebrew/lib. Built against llama.cpp `b11715` (commit `9d1878a`, 17 Oct 2026; ggml-base 0.27.0).

Engine source: Coachwhip, branch `lab/kraits-2010`, commit `e0078a1` (plugin source and release build script; `9021b83` engine library and kernels). Not included here.

## How to run it (the exact commands measured; nothing else was)

1. Put `libggml-kraits.dylib` and `libkraits.dylib` in one folder. Keep `llama-server` and its own `libggml-*.dylib` files together as they came.
2. Tell llama.cpp where the plugin is, every time, with the environment variable. Without it llama.cpp runs as plain llama.cpp and says nothing:

```sh
export GGML_BACKEND_PATH=/path/to/kraits/bin/libggml-kraits.dylib
llama-server --list-devices      # must list "KRAITS". If it does not, the plugin did not load.
```

3. Run the model through Kraits. Both flags are required; without `-ot "token_embd=KRAITS"` the plugin has nothing to compute and the first request fails:

```sh
llama-server -m Qwen3.6-35B-A3B-UD-Q4_K_M.gguf --device KRAITS -ngl 99 -ot "token_embd=KRAITS" -c 8448 -np 1
```

4. The llama.cpp-alone control, same file, same task, same API, for comparison only:

```sh
llama-server -m Qwen3.6-35B-A3B-UD-Q4_K_M.gguf -ngl 99 -ncmoe 40 -t 4 -c 8448
```

## Which llama.cpp

The plugin links `libggml-base` ABI **0.27** (llama.cpp `b11715` = commit `9d1878a`, 17 Oct 2026, or newer). An older llama.cpp (for example b11220, ggml 0.25) cannot load it: `--list-devices` will not show KRAITS. Check with `llama-server --version`.

Developers' measurements (20 Oct 2026, same Mac; the README carries the release tester's numbers, not these): stock `llama-server` + stock Unsloth `Qwen3.6-35B-A3B-UD-Q4_K_M.gguf`, `--device KRAITS`: 24.20–25.18 words/s on the LRU-cache task, judge 8/8, against 2.09 words/s for llama.cpp alone on the same file, same task, same API (the control command above), judge 8/8. Full four-agent crew run: 196 s, 8/8.
