# Kraits — Run Bigger MoE LLMs on 8 GB NVIDIA GPUs & 16 GB Macs

> **One engine. Any supported MoE GGUF.** Point Kraits at the model already on your SSD (Ollama, LM Studio, Hugging Face cache, or Downloads) and run it with VRAM/Metal expert banking — **100% lossless**, no quant tricks.

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

---

## Choose Your Platform

| Platform | Hardware | Verified Headline | Direct Download | Full Guide |
|---|---|---|---|---|
| **Windows (NVIDIA CUDA)** | **8 GB+ VRAM** (RTX 20 / 30 / 40 / 50, zero CUDA Toolkit install) | **Gemma 4 26B (`14.25 GB`)** on `8 GB` RTX 4070: **`400–538 tok/s`** prompt (`1.8–2.6×` stock), **`30–33 tok/s`** decode, 100% lossless | [**`kraits-cuda-v0.1-win64.zip`**](https://github.com/1picassoai/kraits/releases/download/cuda-v0.1/kraits-cuda-v0.1-win64.zip) (`600.53 MB`) | [**Open Windows CUDA Guide ->**](./README-WINDOWS-CUDA.md) |
| **macOS (Apple Silicon)** | **16 GB+ Unified Memory** (M1–M4, macOS 14+) | **Qwen3.8-Flash-Next 80B (`90.97 GiB`)** on `16 GB` M4: **`11–15.4 tok/s`** clean code, `418 MB` swap | [**`Kraits-v0.7.1-macos-arm64.dmg`**](https://github.com/1picassoai/kraits/releases/download/v0.7.1/Kraits-v0.7.1-macos-arm64.dmg) (`16 MB`) | [**Open macOS Guide ->**](./README-MACOS.md) |

---

## Works Across MoE Architectures (Both Windows CUDA & macOS)

Both backends accelerate MoE expert layers generically — supporting **`gemma4`**, **`qwen3moe` / `qwen3next`**, **`gpt-oss`**, **`glm4moe`**, **`bailingmoe` / `laguna`**, **`mixtral`**, **`deepseek`**, and standard `llama.cpp` fallback for dense models (`llama`, `mistral`, etc.).

- **Windows CUDA details, verified numbers & 3-step install:** [**`README-WINDOWS-CUDA.md`**](./README-WINDOWS-CUDA.md)
- **macOS Apple Silicon details, model matrix & 1-step DMG install:** [**`README-MACOS.md`**](./README-MACOS.md)

---

## License

Built on [`llama.cpp`](https://github.com/ggml-org/llama.cpp) (`MIT`) + Kraits engine (`Apache 2.0`).
