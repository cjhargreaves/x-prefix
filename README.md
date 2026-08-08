# X-Prefix: Optimizing Grok Prefix Caching with Live X Data

## The idea

Grok's API automatically caches repeated prompt prefixes — if two requests
start with the identical block of text, the second one hits cache and comes
back cheaper and faster. Right now that only happens by accident, when two
users happen to phrase things the same way.

We're building a layer that makes it happen on purpose.

When a user asks about something that's actively trending or being discussed
on X (a news event, a launch, a live reaction), we pull real, current X data
on that topic *before* the request ever reaches Grok. That data gets
assembled into a dense, consistent block of context and placed at the front
of the prompt as a system message. Every user asking about the same topic
gets the *same* context block prepended — so instead of each person's
question forcing Grok to re-derive what's happening from scratch, the shared
prefix is identical across users and Grok's cache can actually reuse it.

## Why X specifically

X is the source of the thing we're optimizing for: live, fast-moving,
real-time information. Grok already leans on its X integration to know
what's happening right now — we're doing that fetching and shaping
ourselves, upfront, so the prompt Grok receives is already the
highest-signal, de-duplicated version of "what's going on with this topic,"
rather than raw noise it has to sort through per-request.

## What we're trying to prove

- **Latency**: requests that share a pre-fetched X-context prefix get
  measurably faster time-to-first-token than raw, unstructured prompts —
  because Grok is reading a cache hit, not re-processing everything cold.
- **Token cost**: summarizing/curating the X data down to the highest-signal
  tweets before it ever hits the model cuts input token usage compared to
  dumping raw data into the prompt.
- **Real, reported cache hits**: Grok's API reports how many prompt tokens
  were served from cache on each request. We can show that number go from
  zero (first user asking about a topic) to non-zero (every user after that,
  as long as they're asking about the same topic) — not a simulated effect,
  an actual number the API hands back.

## The demo, in one sentence

Two requests, same trending topic, one asked by "different" users: the first
pays full price and full latency: the second — because it's riding the same
pre-assembled X-context prefix — comes back faster, cheaper, and with a
non-zero cache-hit count straight from the API.
