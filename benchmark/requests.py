import json
import os
import time
import urllib.parse
import urllib.request

from dotenv import load_dotenv
from openai import OpenAI

load_dotenv()

XAI_URL = "https://api.x.ai/v1"
GATEWAY_URL = "http://127.0.0.1:3000/v1"
TRENDS_URL = "http://127.0.0.1:3000/trends"


def direct():
    return OpenAI(api_key=os.environ["XAI_API_KEY"], base_url=XAI_URL)


def gateway(trend):
    return OpenAI(
        api_key="unused",
        base_url=GATEWAY_URL,
        default_headers={"x-trend": trend},
    )


def trends():
    return json.load(urllib.request.urlopen(TRENDS_URL))


def trend_prefix(name):
    return urllib.request.urlopen(f"{TRENDS_URL}/{urllib.parse.quote(name)}").read().decode()


def timed(client, model, messages, **kwargs):
    started = time.perf_counter()
    ttft = None
    usage = None
    text = []

    stream = client.chat.completions.create(
        model=model,
        messages=messages,
        stream=True,
        stream_options={"include_usage": True},
        **kwargs,
    )

    for chunk in stream:
        if chunk.usage:
            usage = chunk.usage
        if chunk.choices and chunk.choices[0].delta.content:
            if ttft is None:
                ttft = time.perf_counter() - started
            text.append(chunk.choices[0].delta.content)

    total = time.perf_counter() - started
    return Sample(ttft=ttft, total=total, usage=usage, text="".join(text))


class Sample:
    def __init__(self, ttft, total, usage, text):
        self.ttft = ttft
        self.total = total
        self.usage = usage
        self.text = text

    @property
    def prompt_tokens(self):
        return self.usage.prompt_tokens if self.usage else 0

    @property
    def cached_tokens(self):
        d = getattr(self.usage, "prompt_tokens_details", None) if self.usage else None
        return getattr(d, "cached_tokens", 0) or 0

    @property
    def cost_ticks(self):
        return getattr(self.usage, "cost_in_usd_ticks", 0) if self.usage else 0

    @property
    def completion_tokens(self):
        return self.usage.completion_tokens if self.usage else 0
