#!/usr/bin/env bash
set -euo pipefail
# Kraits 0.6.29 (async_iq4) launcher for llama.cpp b11936 (macOS Apple Silicon, 16 GB+)
# Usage: ./kraits-server.sh /path/to/Qwen3.8-Flash-Next-IQ4_XS-00001-of-00003.gguf [port] [ctx]

D="$(cd "$(dirname "$0")" && pwd)"
M="${1:?usage: ./kraits-server.sh /path/to/model-00001-of-00003.gguf [port] [ctx]}"
P="${2:-8099}"
CTX="${3:-16384}"

if [ -x "$D/llama-server" ]; then
  SERVER="$D/llama-server"
elif command -v llama-server >/dev/null 2>&1; then
  SERVER="$(command -v llama-server)"
else
  echo "error: llama-server not found in $D or PATH (install release tarball or llama.cpp b11936)" >&2
  exit 1
fi

export DYLD_LIBRARY_PATH="$D${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
export GGML_BACKEND_PATH="$D/libggml-kraits.dylib"
export KRAITS_GGUF="$M"
export KRAITS_EXPERTS=1 KRAITS_READ_METAL=1 KRAITS_READ_COMPACT=1
export KRAITS_BANK_READ=1 KRAITS_GPU_BANK=0 KRAITS_WORD_SPLIT=0 KRAITS_KBT_METAL=1
export KRAITS_BANK_BUDGET="${KRAITS_BANK_BUDGET:-7.2}"
export KRAITS_BANK_SLOTS="${KRAITS_BANK_SLOTS:-56}"
export KRAITS_NOCACHE=1 KRAITS_SPLIT=0 KRAITS_SPLIT_MIN_LAYER=99
export KRAITS_ASYNC_IQ4=1 KRAITS_ASYNC_RATIO=130 KRAITS_ASYNC_MIN_MISS=2

exec "$SERVER" -m "$M" \
  --override-kv qwen4exp.expert_used_count=int:6 \
  -ngl 99 -nr -fit off --device KRAITS,MTL0 --main-gpu 1 -sm layer -ts 0,1 \
  -ot exps.weight=KRAITS,per_layer_token_embd.weight=CPU,token_embd.weight=CPU \
  -t 4 -c "$CTX" -ctk q4_0 -ctv q4_0 -fa on -np 1 -ub 2048 -b 2048 \
  --ctx-checkpoints 10 --cache-ram 0 -sps 1.0 --jinja \
  --temp 0.6 --top-p 0.95 --min-p 0.05 --repeat-penalty 1.05 \
  --host 127.0.0.1 --port "$P" --reasoning auto --reasoning-budget 512 --poll 100
