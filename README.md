# prefix-x

Inference cost optimization by injecting live X trending data and posts as shared, cacheable prompt prefixes.

## The problem

Grok caches repeated prompt prefixes. Identical leading text bills at the cached rate.

But it only happens by accident, when two users phrase things the same way.

```
User A: "what's going on with the Nvidia crash?"
User B: "explain the NVDA drop"
         ^ different bytes, no shared prefix, no cache hit
```

## The fix

Put the same block in front of both.

```
1. User asks about a trending topic
2. We pull live X data on it
3. Compress into a fixed context block
4. Prepend as a system message
5. Send to Grok
```

Now both requests start with identical bytes. Cache hits. Answers come grounded
in what's actually on X right now instead of the model guessing.

## Results

20 concurrent users, 60s per run. The honest control is `unshared`: the same
context block, perturbed at the first byte per user so it can never share.
Same prompt size, same model work, the only difference is sharing.

| context size | unshared cost | gateway cost | saved | cached share |
|---|---|---|---|---|
| ~660 tokens (8 posts) | 18.3M ticks | 14.5M ticks | **21%** | 96% |
| ~1610 tokens (20 posts) | 29.3M ticks | 16.3M ticks | **44%** | 94% |

The gap widens as context grows: the added tokens are exactly the ones served
from cache. Latency is a wash (prefill is milliseconds inside a multi-second
response). Cost numbers are xAI's own billing fields, not estimates.

## Stack

- Rust gateway (Axum on Tokio) in front of `api.x.ai`
- X API for trends and posts
- No GPU. xAI hosts the inference.

## Layout

```
clients/   X + xAI API wrappers (clients::x, clients::grok)
gateway/   the proxy: allocator, prefix builder, injection
bench/     Python benchmark, targets the running gateway
```

## Running it

```
cd gateway && cargo run
```

Needs `X_BEARER_TOKEN` and `XAI_API_KEY` in `gateway/.env`.

Knobs (env vars):

| var | default | does |
|---|---|---|
| `TREND_COUNT` | 5 | trends fetched and held |
| `POSTS_PER_TREND` | 8 | posts per context block (controls prefix size) |
| `REFRESH_INTERVAL_SECS` | 300 | background rebuild interval |

Endpoints:

```
GET  /trends                     → currently active trends
GET  /trends/{name}              → that trend's prefix block
POST /v1/chat/completions        → normal OpenAI-shaped chat body
  header x-trend: <name>         → exact match against /trends; injects that
                                    trend's prefix + pins conv-id by topic.
                                    Omit for plain passthrough.
```

Smoke test: `cargo run --bin run-test -- "prompt" [trend name]`

## Benchmark

```
cd bench && .venv/bin/python benchmark.py --users 20 --duration 60
```

Three arms side by side: `direct` (no context), `unshared` (context, no
sharing), `gateway` (context, shared). Live dashboard, then a summary with
TTFT percentiles, cached tokens, and billed cost per arm.
