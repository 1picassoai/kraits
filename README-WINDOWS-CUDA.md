# Kraits-CUDA — Windows x64 (NVIDIA 8 GB+ VRAM)

[<- Back to Main README](./README.md) · [macOS Apple Silicon Guide ->](./README-MACOS.md)

**Run bigger MoE models on 8–16 GB NVIDIA GPUs — 100% lossless, zero quant tricks, zero CUDA Toolkit install.**

---

## Install & Run (3 Steps)

1. Download [**`kraits-cuda-v0.1-win64.zip`**](https://github.com/1picassoai/kraits/releases/download/cuda-v0.1/kraits-cuda-v0.1-win64.zip) (`600.53 MB`, Windows 10/11 x64, SHA-256: `b22d728595764d09a2f7441cd051101cf2372d993974a06ff744b9f1401e2a3c`).
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

---

## Verified Benchmarks (`8 GB` RTX 4070 Laptop, `32 GB` RAM, 100% Lossless)

Model: `gemma-4-26B-A4B-it-qat-UD-Q4_K_XL.gguf` (`13.26 GiB` / `14.25 GB` on `8187 MiB` VRAM, `-ngl 99 -ot exps.weight=CPU -fa 1`)

| Metric | Kraits-CUDA `v0.1` Measured |
|---|---:|
| Prompt Eval (`pp512`) | **`400–538 tok/s`** |
| Generation (`tg128`, default `1050 MB` reserve) | **`30.2–30.5 tok/s`** |
| Live Server Decode (short context) | **`31–33 tok/s`** |
| Precision / Quality | **100% Lossless** (exact weights, exact router picks) |

- Server startup: **`~10s`**
- Stability: **`0` OOMs across `13` launches** with automatic graceful VRAM bank shrink fallback.

---

## VS Code Coding Agent Setup (`Cline` / `Roo Code` / `Continue`) & `32K` Context

Kraits-CUDA exposes a standard OpenAI-compatible API with **verified native tool-calling** (`read_file`, structured `tool_calls` JSON) — zero API cost, 100% offline.

1. Start Kraits-CUDA (default `16K` context, or `-Ctx 32768` for `32K`):
   ```powershell
   .\start-kraits.ps1 -Model "C:\path\to\gemma-4-26B-A4B-it-qat-UD-Q4_K_XL.gguf" -Ctx 32768
   ```
2. In **Cline**, **Roo Code**, or **Continue** (OpenAI Compatible provider):
   - **Base URL:** `http://127.0.0.1:8099/v1`
   - **API Key:** `kraits` (any non-empty string)
   - **Model ID:** `gemma4-26b`

### Verified Multi-Turn `32K` Context & Tool-Calling (`8 GB` RTX 4070 Laptop, `32 GB` RAM)

- **OpenAI Tool-Calling:** **PASS** (`read_file` structured `tool_calls` JSON, `finish=tool_calls`, `21.4–21.8 tok/s`).
- **4-Turn Codebase Fill (`2,977 -> 12,375 -> 24,709 -> 29,801` tokens):** **`4/4 PASS`** needle recall of Turn 1 across `29.8K` tokens, flat **`6,877 / 8,188 MiB`** VRAM (`0` OOM):
  - Prompt eval at long context: **`332–449 tok/s`**
  - Decode at long context (`3K–30K` tokens): **`14–19 tok/s`** *(vs `30–33 tok/s` on short context)*
- **RAM caveat for `32K`:** `32K` context requires **`32 GB` RAM minimum with other apps closed** (free RAM dipped to `90 MB` at `29.8K` tokens); for multitasking with browsers/IDEs open, keep the default **`-Ctx 16384`**.

---

## Supported GPUs & MoE Models

- **NVIDIA GPUs (8 GB+ VRAM):** Multi-arch binary covering **RTX 20-series (`sm_75`)**, **A100 (`sm_80`)**, **RTX 30-series / A6000 (`sm_86`)**, **RTX 40-series (`sm_89`)**, and **RTX 50-series (via PTX JIT)**. *(Physically verified so far on RTX 4070 Laptop `sm_89`; Linux CUDA 12.8 build coming next.)*
- **Supported MoE Architectures:** Hooks `GGML_OP_MUL_MAT_ID` generically — works with **`gemma4`** (verified), **`qwen3moe` / `qwen3next`**, **`gpt-oss`**, **`glm4moe`**, **`mixtral`**, **`deepseek`**, **`bailingmoe`**, and standard `llama.cpp` fallback for dense models.
- **Supported Quants:** `IQ4_XS`, `IQ4_NL`, `IQ3_XXS`, `IQ3_S`, `IQ2_XXS`, `IQ2_S`, `MXFP4`, `Q2_K`–`Q8_K`, `Q4_0`–`Q8_1`, `F16`, `BF16`, `F32`.

---

## Requirements & Tips

- **OS & Driver:** Windows 10/11 x64 + recent NVIDIA GPU driver supporting CUDA 13.x.
- **System RAM:** `32 GB` recommended for 26B–30B MoE models (`Gemma 4 26B` pins ~12–14 GB host RAM; on `16 GB` PCs, set `$env:KRAITS_RAM_CAP_GB = "10"` before launching).
- **Best decode speed:** Close other GPU-heavy apps while serving.
- **Thinking models (Gemma 4 / Qwen3 / DeepSeek-R1):** `start-kraits.ps1` defaults to `-ReasoningBudget 0` so standard OpenAI clients immediately receive clean `content`. Pass `-ReasoningBudget -1` to enable `reasoning_content`.

## Environment Variables (Auto-set by `start-kraits.ps1`)

| Variable | Default | Purpose |
|---|---|---|
| `KRAITS_CUDA` | `1` | Enable Kraits-CUDA MoE VRAM Wrangler Bank |
| `KRAITS_VRAM_RESERVE_MB` | `1050` | Reserve VRAM for Windows/WDDM to prevent spillover |
| `KRAITS_PROMOTE_THRESH` | `2` | Expert hit count before promoting into VRAM |
| `KRAITS_BANK_GB` | auto | Optional hard cap on VRAM bank size |
| `KRAITS_RAM_CAP_GB` | off | Optional Windows hard working-set RAM cap |
