# Kraits — Bring Your Own 80B–177B+ LLM to a 16 GB Mac

> **One app. Any supported GGUF.** Point Kraits at the model already on your Mac SSD (Ollama, LM Studio, Hugging Face cache, or Downloads) and run it with NVMe/Metal expert streaming.

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

---

## Install & Run (1 Step)

1. Download [**`Kraits-v0.7.0-macos-arm64.dmg`**](https://github.com/1picassoai/kraits/releases/download/v0.7.0/Kraits-v0.7.0-macos-arm64.dmg) (`16 MB`, Apple Silicon `arm64`, macOS 14+, SHA-256: `210bae1472a74088b5c57fde6745636882073e8b43ca58debddd072c2565c4bd`).
2. Drag **`Kraits.app`** into **Applications**.
3. Double-click **Kraits** -> pick your `.gguf` (`shard 1` for split models) -> browser opens at `http://127.0.0.1:8105` (with OpenAI API on `:8099`).

> [!NOTE]
> **First launch (ad-hoc build on macOS 15+):** Double-click once to dismiss the initial block, then open **System Settings -> Privacy & Security -> Open Anyway** (for Terminal/CLI use, run `xattr -dr com.apple.quarantine /Applications/Kraits.app`).

---

## Supported Models (`16 GB` Apple Silicon Matrix)

| Architecture (`general.architecture`) | Example Models | Scale | Quants | Status |
|---|---|---|---|---|
| `qwen4exp` | **Qwen3.8-Flash-Next** | ~80B MoE (`90.97 GiB`, native `K=10`) | `IQ4_XS`, `Q4_K_M` | **Verified on 16 GB M4** (`9–14 tok/s` lossless stop-fetch) |
| `qwen3moe` / `qwen3next` | **Qwen3-MoE / Qwen3-Next** | 80B–235B MoE | `IQ4_XS`, `Q4_K_M`, `Q3_K_M` | **Engine Built-In** |
| `gpt-oss` | **GPT-OSS MoE** | 120B MoE | `MXFP4`, `Q4_K_M`, `IQ4_XS` | **Engine Built-In** |
| `gemma4` | **Gemma 4** | Large MoE / dense | `IQ4_XS`, `Q4_K_M`, `Q8_0` | **Engine Built-In** |
| `laguna` / `bailingmoe` | **Ling-3.0-Flash / Laguna** | 100B+ MoE | `Q4_K_M`, `IQ4_XS` | **Engine Built-In** |
| `glm4moe` | **GLM-4.5 / GLM-4.5-Air** | 106B+ MoE | `IQ4_XS`, `Q4_K_M` | **Engine Built-In** |
| Other (`llama`, `mistral`, etc.) | Standard GGUFs | User choice | Any `llama.cpp` GGUF | **Standard `llama.cpp` Fallback** |

**Supported Quants (`libkraits.dylib`):** `IQ4_XS`, `IQ4_NL`, `IQ3_XXS`, `IQ3_S`, `IQ2_XXS`, `IQ2_S`, `IQ1_S`, `IQ1_M`, `MXFP4`, `Q2_K`–`Q8_K`, `Q4_0`–`Q8_1`, `F16`, `BF16`, `F32`.

---

## License

Built on [`llama.cpp`](https://github.com/ggml-org/llama.cpp) (`b11936`, MIT) + Kraits engine (`Apache 2.0`).
