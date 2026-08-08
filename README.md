# X-Prefix

Grok's API caches repeated prompt prefixes — identical leading text = cheaper,
faster requests. We make that happen on purpose instead of by accident.

**How:** when a user asks about a trending topic, we pull real X data on it,
compress it into a dense context block, and prepend it as a system message
*before* the request hits Grok. Every user asking about the same topic gets
the same prefix, so Grok's cache actually reuses it.

**Why X:** it's the source of the live, real-time info Grok already leans on —
we're doing that fetch-and-shape ourselves, upfront, so Grok gets curated
signal instead of raw noise.

**What we're proving:**
- Lower time-to-first-token on cached vs. raw requests
- Lower input token usage (curated context vs. raw dump)
- Real cache hits — `cached_tokens` in Grok's response going from 0 to
  non-zero across "different" users on the same topic

**Demo:** same trending topic, two requests. First one pays full price/latency.
Second rides the shared prefix — faster, cheaper, non-zero cache hit, straight
from the API.
