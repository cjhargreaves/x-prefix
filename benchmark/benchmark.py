import argparse
import asyncio
import random
import statistics
import time
from concurrent.futures import ThreadPoolExecutor

import requests

MODEL = "grok-4"

PHRASING_TEMPLATES = [
    "what's going on with this trend? (user {u}, msg {i})",
    "can someone explain why this is trending right now -- user {u}, msg {i}",
    "give me the short version of what happened, user {u} msg {i}",
    "why is everyone posting about this today? user {u}, msg {i}",
    "catch me up, I've been offline all day (user {u}, msg {i})",
]


class Case:
    def __init__(self, name, client, build_messages=None):
        self.name = name
        self.client = client
        self.build_messages = build_messages or (lambda u, phrasing: [{"role": "user", "content": phrasing}])
        self.samples = []
        self.errors = 0


async def user_loop(case, user_id, deadline):
    i = 0
    while time.monotonic() < deadline:
        phrasing = PHRASING_TEMPLATES[(user_id + i) % len(PHRASING_TEMPLATES)].format(u=user_id, i=i)
        try:
            sample = await asyncio.to_thread(
                requests.timed, case.client, MODEL, case.build_messages(user_id, phrasing)
            )
            case.samples.append(sample)
        except Exception:
            case.errors += 1
        i += 1
        await asyncio.sleep(random.uniform(0.2, 1.0))


def percentile(values, p):
    values = sorted(values)
    return values[min(int(len(values) * p), len(values) - 1)]


def report(case, duration, baseline_cost=None):
    s = case.samples
    header = f" {case.name} "
    print(f"{header:=^40}")
    print(f"Completed requests:      {len(s)}")
    print(f"Errors:                  {case.errors}")
    print(f"Request rate (req/s):    {len(s) / duration:.2f}")

    if s:
        print(f"Mean TTFT (ms):          {statistics.mean(x.ttft for x in s) * 1000:.0f}")
        print(f"Median TTFT (ms):        {percentile([x.ttft for x in s], 0.5) * 1000:.0f}")
        print(f"P95 TTFT (ms):           {percentile([x.ttft for x in s], 0.95) * 1000:.0f}")
        print(f"Mean prompt tokens:      {statistics.mean(x.prompt_tokens for x in s):.0f}")
        print(f"Mean cached tokens:      {statistics.mean(x.cached_tokens for x in s):.0f}")
        cached_share = statistics.mean(x.cached_tokens for x in s) / statistics.mean(x.prompt_tokens for x in s)
        print(f"Cached share:            {cached_share:.0%}")
        print(f"Mean completion tokens:  {statistics.mean(x.completion_tokens for x in s):.0f}")
        gen_tps = statistics.mean(x.completion_tokens / max(x.total - x.ttft, 0.001) for x in s)
        print(f"Mean gen tokens/s:       {gen_tps:.0f}")
        mean_cost = statistics.mean(x.cost_ticks for x in s)
        print(f"Mean cost (ticks):       {mean_cost:.0f}")
        if baseline_cost:
            print(f"Cost vs unshared:        {mean_cost / baseline_cost:.2f}x")

    print("=" * 40)
    print()


async def run(users, duration, trend):
    asyncio.get_running_loop().set_default_executor(ThreadPoolExecutor(max_workers=users * 3 + 4))

    prefix = requests.trend_prefix(trend)

    def unshared_messages(user_id, phrasing):
        return [
            {"role": "system", "content": f"SESSION: user-{user_id}\n{prefix}"},
            {"role": "user", "content": phrasing},
        ]

    cases = [
        Case("direct", requests.direct()),
        Case("unshared", requests.direct(), unshared_messages),
        Case("gateway", requests.gateway(trend)),
    ]

    deadline = time.monotonic() + duration
    loops = [asyncio.ensure_future(user_loop(c, u, deadline)) for c in cases for u in range(users)]
    await asyncio.gather(*loops)

    return cases


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--users", type=int, default=20, help="concurrent users per case")
    parser.add_argument("--duration", type=int, default=60, help="seconds to run")
    args = parser.parse_args()

    trend = requests.trends()[0]
    print(f"trend: {trend}  {args.users} users per case, {args.duration}s\n")

    cases = asyncio.run(run(args.users, args.duration, trend))

    unshared = next(c for c in cases if c.name == "unshared")
    baseline_cost = statistics.mean(s.cost_ticks for s in unshared.samples) if unshared.samples else None

    for case in cases:
        report(case, args.duration, baseline_cost if case.name != "unshared" else None)


if __name__ == "__main__":
    main()
