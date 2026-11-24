# Kraits — Run Bigger MoE LLMs on 8 GB NVIDIA GPUs & 16 GB Macs

> **One engine. Any supported GGUF.** Point Kraits at the MoE model already on your SSD (Ollama, LM Studio, Hugging Face cache, or Downloads) and run it with VRAM/Metal expert banking — **100% lossless**, no quant tricks.

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

---

## Install & Run on Windows (NVIDIA CUDA — 8 GB+ VRAM) — NEW (`cuda-v0.1`)

1. Download [**`kraits-cuda-v0.1-win64.zip`**](https://github.com/1picassoai/kraits/releases/download/cuda-v0.1/kraits-cuda-v0.1-win64.zip) (`600.53 MB`, Windows 10/11 x64, multi-arch **RTX 20 / 30 / 40 / 50 series + A100 / A6000**, SHA-256: `b22d728595764d09a2f7441cd051101cf2372d993974a06ff744b9f1401e2a3c`).
   - **Zero CUDA Toolkit install required** — NVIDIA CUDA 13.4 runtime DLLs (`cudart64_13.dll`, `cublas64_13.dll`, `cublasLt64_13.dll`) are bundled inside the ZIP. Just have a recent NVIDIA GPU driver installed.
2. Unzip to any folder (e.g. `C:\kraits-cuda\`).
3. Run in **PowerShell** (or **Command Prompt**):
   ```powershell
   .\start-kraits.ps1 -Model "C:\path\to\gemma-4-26B-A4B-it-qat-UD-Q4_K_XL.gguf"
   ```
   ```cmd
   start-kraits.bat "C:\path\to\gemma-4-26B-A4B-it-qat-UD-Q4_K_XL.gguf"
   ```
   Open **`http://127.0.0.1:8099`** in your browser (OpenAI-compatible API at `http://127.0.0.1:8099/v1`).

### Verified Windows CUDA Numbers (`8 GB` RTX 4070 Laptop, `32 GB` RAM, 100% Lossless)

Model: `gemma-4-26B-A4B-it-qat-UD-Q4_K_XL.gguf` (`13.26 GiB` / `14.25 GB` on `8187 MiB` VRAM, `-ngl 99 -ot exps.weight=CPU -fa 1`)

| Engine | Prompt (`pp512`) | Decode (`tg128`) | Live Server | Lossless |
|---|---:|---:|---:|:---:|
| Stock `llama.cpp` (`-ot exps.weight=CPU`) | `178–289 tok/s` | `26.2–26.4 tok/s` | — | Yes |
| **Kraits-CUDA `v0.1`** | **`400–538 tok/s` (`1.8–2.6×`)** | **`30.2–30.5 tok/s`** | **`31–33 tok/s`** | **Yes** |

> [!NOTE]
> - **RAM & GPU tips:** `32 GB` system RAM is recommended for 26B–30B MoE models (on `16 GB` PCs, set `$env:KRAITS_RAM_CAP_GB = "10"` before launching). Close other GPU-heavy apps for best decode speed.
> - **Thinking models (Gemma 4 / Qwen3 / DeepSeek-R1):** `start-kraits.ps1` defaults to `-ReasoningBudget 0` so standard OpenAI clients immediately get clean `content`. Pass `-ReasoningBudget -1` to enable `reasoning_content`.
> - **Hardware note:** Multi-arch `sm_75;80;86;89` + PTX is compiled in; physically verified so far on RTX 4070 Laptop (`sm_89`). Linux CUDA 12.8 build coming next.

---

## Install & Run on macOS (Apple Silicon — 16 GB+ Unified Memory)

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
| `gemma4` | **Gemma 4 / Gemma QAT** | Frontier MoE / QAT | `QAT`, `IQ4_XS`, `Q4_K_M`, `Q8_0` | **Verified on 16 GB M4 & 8 GB RTX 4070 (`cuda-v0.1`)** |
| `qwen3moe` / `qwen3next` | **Qwen3-MoE / Qwen3-Next** | 80B–235B MoE | `IQ4_XS`, `Q4_K_M`, `Q3_K_M` | **Engine Built-In** |
| `gpt-oss` | **GPT-OSS MoE** | 120B MoE | `MXFP4`, `Q4_K_M`, `IQ4_XS` | **Engine Built-In** |
| `glm4moe` | **GLM-4.5 / GLM-4.5-Air** | 106B+ MoE | `IQ4_XS`, `Q4_K_M` | **Engine Built-In** |
| Other (`llama`, `mistral`, etc.) | Standard GGUFs | User choice | Any `llama.cpp` GGUF | **Standard `llama.cpp` Fallback** |

**Supported Quants (`libkraits.dylib` / `ggml-cuda.dll`):** `IQ4_XS`, `IQ4_NL`, `IQ3_XXS`, `IQ3_S`, `IQ2_XXS`, `IQ2_S`, `IQ1_S`, `IQ1_M`, `MXFP4`, `Q2_K`–`Q8_K`, `Q4_0`–`Q8_1`, `F16`, `BF16`, `F32`.

---

## License

Built on [`llama.cpp`](https://github.com/ggml-org/llama.cpp) (`MIT`) + Kraits engine (`Apache 2.0`).
