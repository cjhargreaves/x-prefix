# X-Prefix

Force Grok prefix-cache hits using live X data.

## The problem

Grok caches repeated prompt prefixes. Identical leading text = cheaper, faster.

But it only happens by accident, when two users phrase things the same way.

Different phrasing, same topic:

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

Now both requests start with identical bytes. Cache hits.

## Why X

X is the live signal Grok already leans on.

We fetch and shape it ourselves, upfront, so Grok gets curated data instead of raw noise.

## What we're proving

| Metric | Claim |
|---|---|
| TTFT | Lower on cached vs. raw requests |
| Input tokens | Lower (curated context, not a raw dump) |
| `cached_tokens` | Goes 0 to non-zero across users on the same topic |

That last one is the real proof. It's a number the API hands back, not something we estimate.

## Demo

Two requests, same trending topic, two "users."

- **First:** full price, full latency
- **Second:** rides the shared prefix. Faster, cheaper, non-zero cache hit

## Stack

- Rust gateway (Axum) sitting in front of `api.x.ai`
- X API for trends and posts
- No GPU. xAI hosts the inference.
