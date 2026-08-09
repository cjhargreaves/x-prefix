"""N users per arm loop varied prompts for a duration, three cases side by
side: direct, unshared context, shared context through the gateway.

Usage:
    python benchmark.py                        # 20 users, 60s
    python benchmark.py --users 50 --duration 120
"""

import argparse
import asyncio
import random
import statistics
import time
from concurrent.futures import ThreadPoolExecutor

from rich.console import Console
from rich.live import Live
from rich.table import Table

import requests

MODEL = "grok-4"
WINDOW_SECONDS = 15
REFRESH_PER_SECOND = 2

PHRASING_TEMPLATES = [
    "what's going on with this trend? (user {u}, msg {i})",
    "can someone explain why this is trending right now -- user {u}, msg {i}",
    "give me the short version of what happened, user {u} msg {i}",
    "why is everyone posting about this today? user {u}, msg {i}",
    "catch me up, I've been offline all day (user {u}, msg {i})",
]


class Arm:
    def __init__(self, name, client, build_messages=None):
        self.name = name
        self.client = client
        self.build_messages = build_messages or (lambda u, phrasing: [{"role": "user", "content": phrasing}])
        self.samples = []  # (completed_at, Sample)
        self.errors = 0

    def window(self):
        cutoff = time.monotonic() - WINDOW_SECONDS
        return [s for t, s in self.samples if t >= cutoff]

    def all_samples(self):
        return [s for _, s in self.samples]


async def _user_loop(arm, user_id, deadline):
    i = 0
    while time.monotonic() < deadline:
        phrasing = PHRASING_TEMPLATES[(user_id + i) % len(PHRASING_TEMPLATES)].format(
            u=user_id, i=i
        )
        try:
            s = await asyncio.to_thread(
                requests.timed, arm.client, MODEL, arm.build_messages(user_id, phrasing)
            )
            arm.samples.append((time.monotonic(), s))
        except Exception:
            arm.errors += 1
        i += 1
        await asyncio.sleep(random.uniform(0.2, 1.0))


def _dashboard(arms, deadline):
    remaining = max(0.0, deadline - time.monotonic())

    table = Table(title=f"{remaining:>3.0f}s", title_justify="left", padding=(0, 2))
    table.add_column("")
    for arm in arms:
        table.add_column(arm.name, justify="right")

    def row(label, fmt, metric):
        values = []
        for arm in arms:
            w = arm.window()
            values.append(fmt.format(metric(w)) if w else "-")
        table.add_row(label, *values)

    row("requests", "{}", lambda w: sum(1 for _ in w))
    row("req/s", "{:.2f}", lambda w: len(w) / WINDOW_SECONDS)
    row("ttft p50", "{:.1f}s", lambda w: sorted(s.ttft for s in w)[len(w) // 2])
    row("total p50", "{:.1f}s", lambda w: sorted(s.total for s in w)[len(w) // 2])
    row("prompt tok", "{:.0f}", lambda w: statistics.mean(s.prompt_tokens for s in w))
    row("cached tok", "{:.0f}", lambda w: statistics.mean(s.cached_tokens for s in w))
    row("cached", "{:.0%}", lambda w: statistics.mean(s.cached_tokens for s in w)
        / max(statistics.mean(s.prompt_tokens for s in w), 1))

    return table


async def _run(users, duration, trend):
    asyncio.get_running_loop().set_default_executor(ThreadPoolExecutor(max_workers=users * 3 + 4))

    prefix = requests.trend_prefix(trend)

    def unshared_messages(user_id, phrasing):
        return [
            {"role": "system", "content": f"SESSION: user-{user_id}\n{prefix}"},
            {"role": "user", "content": phrasing},
        ]

    arms = [
        Arm("direct", requests.direct()),
        Arm("unshared", requests.direct(), unshared_messages),
        Arm("gateway", requests.gateway(trend)),
    ]
    deadline = time.monotonic() + duration

    loops = [
        asyncio.ensure_future(_user_loop(arm, u, deadline)) for arm in arms for u in range(users)
    ]

    with Live(_dashboard(arms, deadline), refresh_per_second=REFRESH_PER_SECOND) as live:
        while not all(f.done() for f in loops):
            live.update(_dashboard(arms, deadline))
            await asyncio.sleep(1 / REFRESH_PER_SECOND)
        await asyncio.gather(*loops)
        live.update(_dashboard(arms, deadline))

    return arms


def _summary(console, arms, duration):
    table = Table(padding=(0, 2))
    table.add_column("")
    for arm in arms:
        table.add_column(arm.name, justify="right")

    def row(label, fmt, metric):
        table.add_row(label, *[fmt.format(metric(arm.all_samples())) for arm in arms])

    unshared_cost = statistics.mean(s.cost_ticks for s in
                                    next(a for a in arms if a.name == "unshared").all_samples())

    row("requests", "{}", lambda s: len(s))
    row("throughput", "{:.2f} req/s", lambda s: len(s) / duration)
    row("ttft p50", "{:.1f}s", lambda s: sorted(x.ttft for x in s)[len(s) // 2])
    row("ttft p95", "{:.1f}s", lambda s: sorted(x.ttft for x in s)[int(len(s) * 0.95)])
    row("ttft p99", "{:.1f}s", lambda s: sorted(x.ttft for x in s)[int(len(s) * 0.99)])
    row("total p50", "{:.1f}s", lambda s: sorted(x.total for x in s)[len(s) // 2])
    row("gen tok/s", "{:.0f}", lambda s: statistics.mean(
        x.completion_tokens / max(x.total - x.ttft, 0.001) for x in s))
    row("prompt tok", "{:.0f}", lambda s: statistics.mean(x.prompt_tokens for x in s))
    row("cached tok", "{:.0f}", lambda s: statistics.mean(x.cached_tokens for x in s))
    row("cached", "{:.0%}", lambda s: statistics.mean(x.cached_tokens for x in s)
        / statistics.mean(x.prompt_tokens for x in s))
    row("completion tok", "{:.0f}", lambda s: statistics.mean(x.completion_tokens for x in s))
    row("billed cost", "{:.2f}x", lambda s: statistics.mean(x.cost_ticks for x in s) / unshared_cost)

    console.print()
    console.print(table)
    console.print("  billed cost is relative to unshared")

    errors = {arm.name: arm.errors for arm in arms if arm.errors}
    if errors:
        console.print(f"  errors: {errors}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--users", type=int, default=20, help="concurrent users per arm")
    parser.add_argument("--duration", type=int, default=60, help="seconds to run")
    args = parser.parse_args()

    console = Console()
    trend = requests.trends()[0]
    console.print(f"  trend: [bold]{trend}[/]  {args.users} users per arm, {args.duration}s\n")

    arms = asyncio.run(_run(args.users, args.duration, trend))
    _summary(console, arms, args.duration)


if __name__ == "__main__":
    main()
