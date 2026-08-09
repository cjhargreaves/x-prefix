"""Sustained load, live dashboard: N users per arm, each looping prompts for a
duration, direct vs. gateway side by side with bars moving as the rolling
window updates.

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

from rich.console import Console, Group
from rich.live import Live
from rich.table import Table
from rich.text import Text

import requests

MODEL = "grok-4"
WINDOW_SECONDS = 15
REFRESH_PER_SECOND = 4
BAR_WIDTH = 24

PHRASING_TEMPLATES = [
    "what's going on with this trend? (user {u}, msg {i})",
    "can someone explain why this is trending right now -- user {u}, msg {i}",
    "give me the short version of what happened, user {u} msg {i}",
    "why is everyone posting about this today? user {u}, msg {i}",
    "catch me up, I've been offline all day (user {u}, msg {i})",
]


class Arm:
    def __init__(self, name, client, style, build_messages=None):
        self.name = name
        self.client = client
        self.style = style
        self.build_messages = build_messages or (lambda u, phrasing: [{"role": "user", "content": phrasing}])
        self.samples = []  # (completed_at, Sample)
        self.inflight = 0
        self.done = 0
        self.errors = 0

    def window(self):
        cutoff = time.monotonic() - WINDOW_SECONDS
        return [s for t, s in self.samples if t >= cutoff]

    def metrics(self):
        w = self.window()
        ttfts = sorted(s.ttft for s in w)
        return {
            "req/s": len(w) / WINDOW_SECONDS,
            "ttft p50 ms": ttfts[len(ttfts) // 2] * 1000 if ttfts else 0.0,
            "ttft p95 ms": ttfts[int(len(ttfts) * 0.95)] * 1000 if ttfts else 0.0,
            "cached": statistics.mean(s.cached_tokens for s in w) if w else 0.0,
            "prompt": statistics.mean(s.prompt_tokens for s in w) if w else 0.0,
        }


async def _user_loop(arm, user_id, deadline):
    i = 0
    while time.monotonic() < deadline:
        phrasing = PHRASING_TEMPLATES[(user_id + i) % len(PHRASING_TEMPLATES)].format(
            u=user_id, i=i
        )
        arm.inflight += 1
        try:
            s = await asyncio.to_thread(
                requests.timed, arm.client, MODEL, arm.build_messages(user_id, phrasing)
            )
            arm.samples.append((time.monotonic(), s))
            arm.done += 1
        except Exception:
            arm.errors += 1
        finally:
            arm.inflight -= 1
        i += 1
        await asyncio.sleep(random.uniform(0.2, 1.0))


def _bar(value, peak, style):
    frac = 0.0 if peak <= 0 else min(value / peak, 1.0)
    filled = round(frac * BAR_WIDTH)
    return Text("█" * filled + "░" * (BAR_WIDTH - filled), style=style)


def _dashboard(arms, peaks, deadline):
    remaining = max(0.0, deadline - time.monotonic())

    table = Table(title=f"{remaining:>4.0f}s left", title_justify="left")
    table.add_column("metric")
    for arm in arms:
        table.add_column(arm.name, min_width=BAR_WIDTH + 10)

    rows = {}
    for arm in arms:
        for metric, value in arm.metrics().items():
            peaks[metric] = max(peaks.get(metric, 0.0), value)
            rows.setdefault(metric, []).append(value)

    for metric, values in rows.items():
        cells = []
        for arm, value in zip(arms, values):
            cells.append(
                Group(_bar(value, peaks[metric], arm.style), Text(f"{value:,.1f}", style="dim"))
            )
        table.add_row(metric, *cells)

    status = Text()
    for arm in arms:
        status.append(f"  {arm.name}: {arm.done} done, {arm.inflight} in flight", style=arm.style)
        if arm.errors:
            status.append(f", {arm.errors} errors", style="red")
    return Group(table, status)


async def _run(users, duration, trend):
    asyncio.get_running_loop().set_default_executor(ThreadPoolExecutor(max_workers=users * 3 + 4))

    prefix = requests.trend_prefix(trend)

    def unshared_messages(user_id, phrasing):
        return [
            {"role": "system", "content": f"SESSION: user-{user_id}\n{prefix}"},
            {"role": "user", "content": phrasing},
        ]

    arms = [
        Arm("direct", requests.direct(), "cyan"),
        Arm("unshared", requests.direct(), "yellow", unshared_messages),
        Arm("gateway", requests.gateway(trend), "magenta"),
    ]
    deadline = time.monotonic() + duration
    peaks = {}

    loops = [
        asyncio.ensure_future(_user_loop(arm, u, deadline)) for arm in arms for u in range(users)
    ]

    with Live(_dashboard(arms, peaks, deadline), refresh_per_second=REFRESH_PER_SECOND) as live:
        while not all(f.done() for f in loops):
            live.update(_dashboard(arms, peaks, deadline))
            await asyncio.sleep(1 / REFRESH_PER_SECOND)
        await asyncio.gather(*loops)
        live.update(_dashboard(arms, peaks, deadline))

    return arms


def _summary(console, arms, duration, users):
    console.print()
    for arm in arms:
        samples = [s for _, s in arm.samples]
        if not samples:
            console.print(f"  [bold]{arm.name}[/]  no completed requests")
            continue
        ttfts = sorted(s.ttft for s in samples)
        console.print(f"  [bold]{arm.name}[/]  ({users} users, {duration}s, {len(samples)} requests)")
        console.print(f"    throughput       {len(samples) / duration:>8.2f} req/s")
        console.print(f"    median ttft      {ttfts[len(ttfts) // 2] * 1000:>8.0f} ms")
        console.print(f"    p95 ttft         {ttfts[int(len(ttfts) * 0.95)] * 1000:>8.0f} ms")
        console.print(f"    p99 ttft         {ttfts[int(len(ttfts) * 0.99)] * 1000:>8.0f} ms")
        console.print(f"    mean prompt      {statistics.mean(s.prompt_tokens for s in samples):>8.0f} tokens")
        console.print(f"    mean cached      {statistics.mean(s.cached_tokens for s in samples):>8.0f} tokens")
        console.print(f"    mean cost        {statistics.mean(s.cost_ticks for s in samples):>8.0f} ticks")
        if arm.errors:
            console.print(f"    errors           [red]{arm.errors}[/]")
        console.print()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--users", type=int, default=20, help="concurrent users per arm")
    parser.add_argument("--duration", type=int, default=60, help="seconds to run")
    args = parser.parse_args()

    console = Console()
    trend = requests.trends()[0]
    console.print(f"  trend: [bold]{trend}[/]\n")

    arms = asyncio.run(_run(args.users, args.duration, trend))
    _summary(console, arms, args.duration, args.users)


if __name__ == "__main__":
    main()
