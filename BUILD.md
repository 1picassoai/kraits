# Kraits 0.6.16 - what is in bin/ and where it came from

| file | bytes | sha256 |
|---|---|---|
| bin/libggml-kraits.dylib (the llama.cpp add-in) | 308,656 | 983dd8c851986973d8ab0aa44ee99185f3bac7d0e5d955f6cc58d33dd1457736 |
| bin/libkraits.dylib (the Kraits engine) | 10,512,752 | 66a6374983b9811c355437812df29c3c4b81f31afdbbde8ab1d2d4dc71841347 |
| bin/libggml-metal.0.27.0.dylib (patched ggml-metal for encoder split) | 5,589,312 | afa96da57ba9f7d4935609bb61e7d79f50694ca91c77b8811b4a08d34fa49baf |
| bin/kraits-server.sh (the launcher, a shell script) | 1,028 | e05ac4dc8965acb03622d4e0343a0c3ab6a390b67928dafecb0ecea1aa6e6a9c |
| bin/Ling-3.0-flash-TS128-Q4_K_M.kraits-pins.txt (the expert pin list) | 16,192 | 7a5c3f335ee411b5733eb507b650f9a0f646ce5f5660495442dc4a05d23d0c02 |

Built on a Mac mini M4 (16 GB), macOS 15 or later required, Apple silicon only. Ad-hoc signed. Build paths are remapped, so no folder of the building Mac is inside the binaries. The plugin finds the engine library beside itself (`@rpath` / `@loader_path`). Built against llama.cpp `b11936` (ggml-base 0.27.0).

## The model file

Kraits 0.6.16 runs one file and refuses any other, checking the file's exact byte size and two 1 MiB FNV-1a fingerprints of its weights (`64 MiB` offset = `0x6c4cbc0769e989ac`, `11 GiB` offset = `0x11032d6407ce0257`):

- `Ling-3.0-flash-TS128-Q4_K_M.gguf` (https://huggingface.co/vinnj/Ling-3.0-flash-TypeScript-Edition-GGUF), exactly `20,395,487,680` bytes (`sha256: e9ae4335a36ed49e8262add1099459a807692edb18c6252ba855f0923c52f602`).

## The expert pin list

`Ling-3.0-flash-TS128-Q4_K_M.kraits-pins.txt` has one line per MoE layer (`2` through `40`): the layer number, then that layer's 128 experts ranked hottest first from TypeScript coding prompts. At load Kraits pins an 8.8 GB bank of the hottest experts in wired GPU memory (`mlock`) and streams cold experts from the SSD. Keep `Ling-3.0-flash-TS128-Q4_K_M.kraits-pins.txt` in the same folder as `Ling-3.0-flash-TS128-Q4_K_M.gguf`.

## The short way: the release bundle and launcher

This git repository holds the Kraits plugin libraries and launcher script, not `llama-server`. Download the complete release archive `kraits-v0.6.16-macos-arm64.tar.gz` from https://github.com/1picassoai/kraits/releases/tag/v0.6.16 (which bundles `llama-server`, the `ggml` libraries, the Kraits `0.6.16` plugin, and the pin list) and start it with `kraits-server.sh`:

```sh
tar -xzf kraits-v0.6.16-macos-arm64.tar.gz
./release-0616/kraits-server.sh /path/to/Ling-3.0-flash-TS128-Q4_K_M.gguf 8080
```

The README carries the release tester's numbers.
