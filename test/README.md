# The test behind the numbers

`task_lru.txt` is the prompt: a TypeScript LRU cache with a cache-wide default TTL. Send it as the user message over `/v1/chat/completions` with the body in BUILD.md (temperature 0, thinking off, max 3,000 tokens).

`judge.ts` is the judge: eight executed checks, no AI. It needs Node 20 or later (`node --version` to check); without Node the judge cannot run, which says nothing about Kraits. Save the model's TypeScript class as `main.ts` next to `judge.ts` (it must export `LruCache`) and run `npx -y tsx judge.ts`. The last line prints `SCORE n/8`.

The judge was written for this prompt. A different prompt (for example one without the cache-wide TTL) gives a different class and a lower score for reasons that have nothing to do with the engine.
