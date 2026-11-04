#!/bin/bash
# Kraits 0.6.16 for llama.cpp b11936 (macOS, Apple silicon).
# Usage: ./kraits-server.sh /path/to/model.gguf [port]
# The model's expert list, <model>.kraits-pins.txt, must sit next to the .gguf.
D="$(cd "$(dirname "$0")" && pwd)"
M="${1:?usage: ./kraits-server.sh /path/to/model.gguf [port]}"
P="${2:-8080}"
export GGML_BACKEND_PATH="$D/libggml-kraits.dylib"
export KRAITS_EXPERTS=1 KRAITS_GPU_BANK=0 KRAITS_WORD_SPLIT=0
export KRAITS_READ_METAL=1 KRAITS_KBT_METAL=1 KRAITS_NO_REFILL=1 KRAITS_READ_COMPACT=1
export KRAITS_WIRE=1 KRAITS_BANK_BUDGET=8.8 KRAITS_WIRE_BANK=1 KRAITS_BANK_READ=1
export KRAITS_SPLIT=1 KRAITS_FETCH1=2 KRAITS_GGUF="$M"
exec "$D/llama-server" -m "$M" -ngl 99 -nr -fit off --no-warmup --device KRAITS,MTL0 --main-gpu 1 -sm layer -ts 0,1 \
    -ot exps.weight=KRAITS -t 4 -c 1536 -fa on -np 1 -ub 256 -b 256 --ctx-checkpoints 0 --cache-ram 0 \
    --temp 0.0 --repeat-penalty 1.0 --dry-multiplier 0.5 --dry-base 1.75 --dry-allowed-length 12 \
    --host 127.0.0.1 --port "$P" --reasoning off
