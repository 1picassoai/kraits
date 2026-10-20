# Kraits 0.1.0 — what is in bin/ and where it came from

| file | bytes | sha256 |
|---|---|---|
| bin/libggml-kraits.dylib (the llama.cpp add-in) | 73,392 | 6e1a521f5a5d977b4b7a3a66049246d7e5bf971570735e2ba54d328419d1ce8e |
| bin/libkraits.dylib (the engine behind it) | 9,313,776 | 5986e43ee7dbf4df685952bf24e7c5ddda953c6392fdd400e352bf41ad8210ab |

Built on a Mac mini M4 (16 GB), macOS 15 or later required, Apple silicon only. Ad-hoc signed. The plugin finds the engine beside itself; ggml-base beside llama-server or in /opt/homebrew/lib. Built against llama.cpp b11715.

Engine source: Coachwhip, branch `lab/kraits-2010`, commit `e0078a1` (plugin source and release build script; `9021b83` engine library and kernels). Not included here.

Tested (20 Oct 2026, same Mac): stock `llama-server` + stock Unsloth `Qwen3.6-35B-A3B-UD-Q4_K_M.gguf`, `--device KRAITS`: 24.20–25.18 words/s on the LRU-cache task, judge 8/8, against 2.09 words/s for llama.cpp alone on the same file, same task, same API (`-ngl 99 -ncmoe 40 -t 4 -c 8448`), judge 8/8. Full four-agent crew run: 196 s, 8/8.
