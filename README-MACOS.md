# Kraits — macOS Apple Silicon (16 GB+ Unified Memory)

[<- Back to Main README](./README.md) · [Windows NVIDIA CUDA Guide ->](./README-WINDOWS-CUDA.md)

**Bring Your Own 80B–177B+ LLM to a 16 GB Mac** with NVMe/Metal expert streaming.

---

## Install & Run (1 Step)

1. Download [**`Kraits-v0.7.1-macos-arm64.dmg`**](https://github.com/1picassoai/kraits/releases/download/v0.7.1/Kraits-v0.7.1-macos-arm64.dmg) (`16 MB`, Apple Silicon `arm64`, macOS 14+, SHA-256: `fa65a5c0a9f4d829f2e3bd6bf545ede23c0194b42facf4c4ff2811d7c84e0a0e`).
2. Drag **`Kraits.app`** into **Applications**.
3. Double-click **Kraits** -> pick your `.gguf` (`shard 1` for split models) -> browser opens at `http://127.0.0.1:8105` (with OpenAI API on `:8099`).

> [!NOTE]
> **First launch (ad-hoc build on macOS 15+):** Double-click once to dismiss the initial block, then open **System Settings -> Privacy & Security -> Open Anyway** (for Terminal/CLI use, run `xattr -dr com.apple.quarantine /Applications/Kraits.app`).

---

## Supported Models (`16 GB` Apple Silicon Matrix)

| Architecture (`general.architecture`) | Example Models | Scale | Quants | Status |
|---|---|---|---|---|
| `qwen4exp` | **Qwen3.8-Flash-Next** | ~80B MoE (`90.97 GiB`, native `K=10`) | `IQ4_XS`, `Q4_K_M` | **Verified on 16 GB M4 (`v0.7.1` DMG)** (`11–15.4 tok/s` clean code, `418 MB` swap) |
| `laguna` / `bailingmoe` | **Ling-3.0-Flash / Laguna** | `20.4 GB` TS128 – `72 GB` MoE | `Q4_K_M`, `IQ4_XS` | **Verified on 16 GB M4 (`0.6.17`)** (`19.6–20.7 tok/s` on TS128) |
| `gemma4` | **Gemma 4 / Gemma QAT** | Frontier MoE / QAT | `QAT`, `IQ4_XS`, `Q4_K_M`, `Q8_0` | **Verified on 16 GB M4** (prior engine pass) |
| `qwen3moe` / `qwen3next` | **Qwen3-MoE / Qwen3-Next** | 80B–235B MoE | `IQ4_XS`, `Q4_K_M`, `Q3_K_M` | **Engine Built-In** |
| `gpt-oss` | **GPT-OSS MoE** | 120B MoE | `MXFP4`, `Q4_K_M`, `IQ4_XS` | **Engine Built-In** |
| `glm4moe` | **GLM-4.5 / GLM-4.5-Air** | 106B+ MoE | `IQ4_XS`, `Q4_K_M` | **Engine Built-In** |
| Other (`llama`, `mistral`, etc.) | Standard GGUFs | User choice | Any `llama.cpp` GGUF | **Standard `llama.cpp` Fallback** |

**Supported Quants (`libkraits.dylib`):** `IQ4_XS`, `IQ4_NL`, `IQ3_XXS`, `IQ3_S`, `IQ2_XXS`, `IQ2_S`, `IQ1_S`, `IQ1_M`, `MXFP4`, `Q2_K`–`Q8_K`, `Q4_0`–`Q8_1`, `F16`, `BF16`, `F32`.
