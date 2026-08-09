import client

NAME = "baseline: one plain request, straight to xAI"

MODEL = "grok-4"
PROMPT = "What is prefix caching?"


def run():
    s = client.timed(
        client.direct(),
        MODEL,
        [{"role": "user", "content": PROMPT}],
    )

    print(f"  model   {MODEL}")
    print(f"  ttft    {s.ttft * 1000:.0f} ms")
    print(f"  total   {s.total * 1000:.0f} ms")
    print(f"  prompt  {s.prompt_tokens} tokens ({s.cached_tokens} cached)")
