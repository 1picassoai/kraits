# Kraits 0.1.0 — what is in bin/ and where it came from

| file | bytes | sha256 |
|---|---|---|
| bin/libggml-kraits.dylib (the llama.cpp add-in) | 73,792 | d70bf412b1cb4573e8e500b4f519746b378de6f068eae8c2fb7293e2756cb10b |
| bin/libkraits.dylib (the Kraits engine) | 9,305,904 | 7433459047b6f381632212c65f2e489c412ed40be6139882a425204401f3688b |
| bin/kraits-server (the launcher, a shell script) | 1,719 | ac41413debd10f012b3a0c2612ffab255ce9c12a255bc667e3ffe382bdaf7e65 |

Built on a Mac mini M4 (16 GB), macOS 15 or later required, Apple silicon only. Ad-hoc signed. The plugin finds the engine library beside itself; ggml-base beside llama-server or in /opt/homebrew/lib. Built against llama.cpp `b11715` (commit `9d1878a`, 17 Oct 2026; ggml-base 0.27.0).

The engine's source is not published; its build records are kept privately and the binaries above are reproducible from them by the maintainers.

## The model file

Kraits needs Unsloth's `Qwen3.6-35B-A3B-UD-Q4_K_M.gguf` (https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF, branch main), exactly 22,134,528,992 bytes. It checks the file's size and a fingerprint of its weights and refuses any other file, including Unsloth's separate "MTP" build, with a one-line message.

## The short way: the launcher

`bin/kraits-server` sits beside the two binaries. It sets the plugin path from its own folder, checks that llama.cpp actually listed KRAITS, stops with exit 1 and the reason if not, and otherwise prints `Kraits loaded` and starts `llama-server` with the required flags:

```sh
cd kraits/bin
LLAMA_SERVER=/path/to/llama-server ./kraits-server -m Qwen3.6-35B-A3B-UD-Q4_K_M.gguf -c 8448
```

(`LLAMA_SERVER` can be left out when `llama-server` is on your PATH. If the script is not executable after cloning, `chmod +x kraits-server`.) The launcher sets `-np 1` and `--cache-ram 0`: Kraits serves **one request at a time** and keeps the conversation itself (the server's prompt cache is off). A follow-up on the latest answer continues from where it stopped; a follow-up on an older answer is re-read first. Started by hand without either flag, the engine refuses to start (exit 1, `kraits: STOPPED. ...`). If it says `STOPPED`, Kraits did not load and nothing is running; the message names the llama-server version and the build Kraits is tested with.

## How to run it by hand (the exact commands measured; nothing else was)

1. Put `libggml-kraits.dylib` and `libkraits.dylib` in one folder. Keep `llama-server` and its own `libggml-*.dylib` files together as they came.
2. Tell llama.cpp where the plugin is, every time, with the environment variable. Without it llama.cpp runs as plain llama.cpp and says nothing:

```sh
export GGML_BACKEND_PATH=/path/to/kraits/bin/libggml-kraits.dylib
llama-server --list-devices      # must list "KRAITS". If it does not, the plugin did not load.
```

3. Run the model through Kraits. Both flags are required; without `-ot "token_embd=KRAITS"` the plugin has nothing to compute and the first request fails:

```sh
llama-server -m Qwen3.6-35B-A3B-UD-Q4_K_M.gguf --device KRAITS -ngl 99 -ot "token_embd=KRAITS" -c 8448 -np 1 --cache-ram 0
```

3b. Starting with `--device KRAITS` and without `-ot "token_embd=KRAITS"`, `-np 1` or `--cache-ram 0` stops at once with `kraits: STOPPED. --device KRAITS also needs: -ot "token_embd=KRAITS"` (exit code 1).

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

Reproduced from this file, by someone who did not write it (20 Oct 2026, fresh folder on the same Mac, the repo's own binaries, `test/task_lru.txt` as the prompt, `test/judge.ts` as the judge): **25.36 tokens/s, judge 8/8** (792 tokens). All speeds here are llama.cpp's own `predicted_per_second`: tokens per second, not words (dense code runs about 2.5 tokens a word). Developers' lab measurements before that, same Mac: 24.20–25.18 tokens/s, 8/8; against 2.09 tokens/s for llama.cpp alone on the same file, same task, same API (the control command above), judge 8/8. Full four-agent crew run: 196 s, 8/8. The README carries the release tester's numbers, not these.
