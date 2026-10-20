# Kraits 0.1.0 — what is in bin/ and where it came from

| file | bytes | sha256 |
|---|---|---|
| bin/libggml-kraits.dylib (the llama.cpp add-in) | 73,600 | 40ad900ac6bedb76cc46ff79709123e582d84f9dd30550f4e8360430425af009 |
| bin/libkraits.dylib (the engine behind it) | 9,313,776 | 5986e43ee7dbf4df685952bf24e7c5ddda953c6392fdd400e352bf41ad8210ab |

Built on a Mac mini M4 (16 GB), macOS 15 or later required, Apple silicon only. Ad-hoc signed. The plugin finds the engine beside itself; ggml-base beside llama-server or in /opt/homebrew/lib. Built against llama.cpp `b11715` (commit `9d1878a`, 17 Oct 2026; ggml-base 0.27.0).

Engine source: Coachwhip, branch `lab/kraits-2010`, commit `fcd8c03` (plugin refuses to start without `-ot "token_embd=KRAITS"`; earlier `e0078a1` plugin + build script, `9021b83` engine library and kernels). Not included here.

## The model file

`Qwen3.6-35B-A3B-UD-Q4_K_M.gguf` from https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF (branch main), 22,134,528,992 bytes. No other file was measured. The separate "MTP" build from Unsloth's other repo is not this file.

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

3b. Starting with `--device KRAITS` and without `-ot "token_embd=KRAITS"` now stops at once with `kraits: STOPPED. --device KRAITS also needs: -ot "token_embd=KRAITS"` (exit code 1).

3c. The measured task is `test/task_lru.txt` (the judge in `test/judge.ts` was written for it; see `test/README.md`), sent over `/v1/chat/completions` with thinking switched off in the request body, which is where that switch lives for this model:

```json
{"messages":[{"role":"user","content":"<the task>"}],"max_tokens":3000,"temperature":0,"chat_template_kwargs":{"enable_thinking":false}}
```

4. The llama.cpp-alone control, same file, same task, same API, for comparison only:

```sh
llama-server -m Qwen3.6-35B-A3B-UD-Q4_K_M.gguf -ngl 99 -ncmoe 40 -t 4 -c 8448
```

## Which llama.cpp

The plugin links `libggml-base` ABI 0.27 and is tested only with llama.cpp **b11715** (commit `9d1878a`, 17 Oct 2026). Other builds are not tested and may not load it: plugins use llama.cpp's internal backend structures, which can change in any build. An older llama.cpp (for example b11220, ggml 0.25) does not load it; `--list-devices` will not show KRAITS. Check with `llama-server --version`.

Reproduced from this file, by someone who did not write it (20 Oct 2026, fresh folder on the same Mac, the repo's own binaries, `test/task_lru.txt` as the prompt, `test/judge.ts` as the judge): **25.36 words/s, judge 8/8** (792 words). Developers' lab measurements before that, same Mac: 24.20–25.18 words/s, 8/8; against 2.09 words/s for llama.cpp alone on the same file, same task, same API (the control command above), judge 8/8. Full four-agent crew run: 196 s, 8/8. The README carries the release tester's numbers, not these.
