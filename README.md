# prefix-x

Inference optimization experiment. Injects live X trending data as a shared,
cacheable prompt prefix so that xAI's KV cache can reuse it across requests.

## The question

When the model reads a prompt, the state it builds for each token depends only
on the tokens before it. Two requests that start with the same text share the
state for that text, and a server that already has it can skip that work and
only process the new tokens. xAI reports this in `cached_tokens` and bills
cached tokens at a lower rate.

Across different users this never happens on its own. Everyone words their
question differently, and requests get spread across servers that each keep
their own cache.

So the experiment forces it. When a topic trends, build one context block from
its live posts, put the exact same bytes at the front of every request about
that topic, send them to the same server, and see whether the cache hits.

## How it works

1. Pull current trending topics and the top posts for each.
2. Build one fixed context block per trend. Nothing in it changes between
   requests.
3. Insert it as the first system message on requests that name the trend.
4. Set `x-grok-conv-id` to a hash of the topic instead of the user, so
   same-topic requests are routed to the same server.

The context block lists the trend, the post text with like and repost counts,
and guidance to answer from the block only, in a few sentences.

## Measuring it

xAI reports cached tokens on every response, including a baseline even when
nothing is shared, from text xAI adds on its side. So a nonzero cached count
proves nothing by itself. The control for this is the unshared case, which
sends the exact same context block but with a different first line per user.
Matching stops at the first difference, so everything after that line is
processed at full price even though it is identical. Same prompt size, same
work, the only difference is whether the text is shared.

Three cases, 20 concurrent users each, 60 seconds:

| case | sends |
|---|---|
| direct | question only |
| unshared | context block with a per-user first line, plus question |
| gateway | shared context block, plus question |

## Findings

Sharing the prefix gets cache hits. The shared case billed less prompt cost
than the control, and most of the prompt tokens came from cache. The gap grows
with prefix size, since the cache covers exactly those tokens.

Caveats. Single runs against a live service, not a study. The savings are on
xAI's side, not the caller's. Time to first token did not improve, and the
cache needs a moment to warm up, so a request right after the first one can
still miss.

## Stack

- Rust gateway, Axum on Tokio, in front of api.x.ai
- X API for trends and posts
- No local GPU, xAI hosts the inference

## Layout

```
clients/    X and xAI API wrappers
gateway/    the proxy, prefix allocator, prefix builder
benchmark/  Python benchmark, runs the three cases against the gateway
```

## Running it

```
cd gateway && cargo run
```

Needs `X_BEARER_TOKEN` and `XAI_API_KEY` in `gateway/.env`.

| env var | default | what it does |
|---|---|---|
| `TREND_COUNT` | 5 | trends fetched and held |
| `POSTS_PER_TREND` | 8 | posts per block, controls prefix size |
| `REFRESH_INTERVAL_SECS` | 300 | background rebuild interval |

```
GET  /trends                    active trends
GET  /trends/{name}             that trend's context block
POST /v1/chat/completions       OpenAI-shaped chat body
  header x-trend: <name>        exact match, injects the block and pins the
                                conv-id to the topic. Omit it and the request
                                passes through untouched.
```

Smoke test: `cargo run --bin run-test -- "prompt" [trend name]`

## Benchmark

```
cd benchmark && .venv/bin/python benchmark.py --users 20 --duration 60
```

Picks the first active trend, runs the three cases, and prints token, latency,
and cost metrics per case plus the gateway versus unshared comparison. It has
a live dashboard while it runs.
