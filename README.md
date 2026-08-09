# prefix-x

Inference optimization experimentation by injecting live X trending data and posts as shared, cacheable prompt prefixes.

The question: if you put the same live X data in front of every request about
a trending topic, does Grok's KV cache actually reuse it, and do the requests
that follow the first one come out ahead because of it?

## The idea

When the model reads a prompt, the internal state it builds for each token
only depends on the tokens before it. So if two requests start with the same
text, the state for that part is the same, and a server that already has it
can skip that work and only process the new tokens. xAI reports this in
`cached_tokens` on every response and bills those tokens at a lower rate.

Across different users this basically never happens on its own. Everyone
words their question differently, so no two requests start the same, and
requests get spread across servers that each have their own cache.

So the experiment is to force it. When a topic trends, build one context block
from its live posts, put the exact same bytes at the front of every request
about that topic, send them all to the same server, and see if the cache hits.

## How

```
1. Pull current trending topics and top posts
2. Build one fixed context block per trend, nothing in it that
   changes between requests
3. Insert it as the first system message on requests naming that trend
4. Set x-grok-conv-id to a hash of the topic instead of the user,
   so same-topic requests go to the same server
```

## Measuring it

Every xAI response reports 130 to 190 cached tokens even when nothing is
shared, from text xAI adds to requests on their side. So a nonzero cached
count proves nothing by itself. To isolate the effect, the unshared case
sends the exact same context block but with a different first line per user.
Matching stops at the first difference, so everything after that line gets
processed at full price even though it is identical. Same prompt size, same
work for the model, the only difference is whether the text is shared.

Three cases, 20 concurrent users each, 60 seconds:

| case | sends |
|---|---|
| direct | question only |
| unshared | context block with a per-user first line, plus question |
| gateway | shared context block, plus question |

## What came out

With blocks around 660 tokens the gateway billed 21% less than unshared. At
around 1600 tokens it was 44% less, with 94 to 96% of prompt tokens coming
from cache. The bigger the block, the bigger the gap, because the extra
tokens are exactly the ones the cache covers.

Time to first token did not improve. Processing 1500 tokens of prompt takes
tens of milliseconds inside a response that takes several seconds, and going
through the proxy adds a little time on top. The saved work is real, but
since xAI owns the GPUs, it shows up as their lower cached-token price
instead of faster responses.

Also, the cache takes a moment after the first request on a topic before it
starts hitting. A request that comes in right after the first one can miss.

## Stack

- Rust gateway, Axum on Tokio, in front of api.x.ai
- X API for trends and posts
- No GPU. xAI hosts the inference.

## Layout

```
clients/   X + xAI API wrappers
gateway/   the proxy, allocator, prefix builder, injection
bench/     Python benchmark, runs the three cases against the gateway
```

## Running it

```
cd gateway && cargo run
```

Needs `X_BEARER_TOKEN` and `XAI_API_KEY` in `gateway/.env`.

| env var | default | does |
|---|---|---|
| `TREND_COUNT` | 5 | trends fetched and held |
| `POSTS_PER_TREND` | 8 | posts per block, controls prefix size |
| `REFRESH_INTERVAL_SECS` | 300 | background rebuild interval |

```
GET  /trends                     active trends
GET  /trends/{name}              that trend's context block
POST /v1/chat/completions        OpenAI-shaped chat body
  header x-trend: <name>         exact match, injects the block and pins
                                 the conv-id to the topic. Omit it and the
                                 request passes through untouched.
```

To check it's working: `cargo run --bin run-test -- "prompt" [trend name]`

## Benchmark

```
cd bench && .venv/bin/python benchmark.py --users 20 --duration 60
```

Live dashboard while it runs, then token metrics, latency, and cost for each
case, plus the gateway vs unshared comparison.
