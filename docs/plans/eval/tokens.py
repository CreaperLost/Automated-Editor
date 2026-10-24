"""How much an agent read in an EVAL-1 turn (AI-1: about 100k tokens or less for a whole edit).

Usage: python tokens.py <turn.jsonl> [<turn.jsonl> ...]  (Claude Code's `--output-format
stream-json --verbose` logs, as run.sh writes them; give an edit's turns in order).

For each turn: the context at its first and last model call (from the API's own usage: what the
model was sent, cached or not), and what our tools answered in it, by tool, text and images. The
context at the last call of the last turn is what the agent read in the whole edit. Text is
counted at about 4 characters a token and an image at width x height / 750 (Anthropic's rule for
images); the totals from usage are exact.
"""

import base64
import json
import struct
import sys
from collections import defaultdict

OURS = "mcp__aeroedits__"
TARGET = 100_000


def image_size(data: str):
    """Width and height of a base64 PNG or JPEG (Claude Code passes some images on as JPEG)."""
    raw = base64.b64decode(data + "=" * (-len(data) % 4))
    if raw[:8] == b"\x89PNG\r\n\x1a\n":
        return struct.unpack(">II", raw[16:24])
    if raw[:2] == b"\xff\xd8":
        at = 2
        while at + 9 < len(raw):
            if raw[at] != 0xFF:
                return None
            marker, length = raw[at + 1], struct.unpack(">H", raw[at + 2:at + 4])[0]
            # A start-of-frame segment holds the size (not DHT C4, JPG C8 or DAC CC).
            if 0xC0 <= marker <= 0xCF and marker not in (0xC4, 0xC8, 0xCC):
                height, width = struct.unpack(">HH", raw[at + 5:at + 9])
                return width, height
            at += 2 + length
    return None


def answer_tokens(content):
    """Text and image tokens in one tool result."""
    if isinstance(content, str):
        return len(content) / 4, 0, 0
    text = image = images = 0
    for block in content or []:
        if block.get("type") == "text":
            text += len(block.get("text", "")) / 4
        elif block.get("type") == "image":
            size = image_size(block.get("source", {}).get("data", ""))
            if size:
                image += size[0] * size[1] / 750
            images += 1
    return text, image, images


def turn(path):
    calls = {}  # message id -> prompt tokens at that call
    names = {}  # tool_use id -> tool name
    by_tool = defaultdict(lambda: [0, 0.0, 0.0, 0])  # calls, text, image tokens, images
    for line in open(path, encoding="utf-8"):
        try:
            m = json.loads(line)
        except ValueError:
            continue
        if m.get("type") == "assistant":
            message = m["message"]
            usage = message.get("usage", {})
            calls[message.get("id")] = (
                usage.get("input_tokens", 0)
                + usage.get("cache_read_input_tokens", 0)
                + usage.get("cache_creation_input_tokens", 0)
            )
            for c in message.get("content", []):
                if c.get("type") == "tool_use":
                    names[c["id"]] = c["name"]
        elif m.get("type") == "user":
            content = m.get("message", {}).get("content", [])
            for c in content if isinstance(content, list) else []:
                name = names.get(c.get("tool_use_id"), "")
                if c.get("type") != "tool_result" or not name.startswith(OURS):
                    continue
                text, image, images = answer_tokens(c.get("content"))
                row = by_tool[name[len(OURS):]]
                row[0] += 1
                row[1] += text
                row[2] += image
                row[3] += images
    prompts = list(calls.values())
    return prompts, by_tool


def main(paths):
    end = 0
    for path in paths:
        prompts, by_tool = turn(path)
        if not prompts:
            print(f"{path}: no model calls")
            continue
        end = prompts[-1]
        text = sum(r[1] for r in by_tool.values())
        image = sum(r[2] for r in by_tool.values())
        images = sum(r[3] for r in by_tool.values())
        print(f"== {path}")
        print(f"context: {prompts[0]:,} tokens at the first call, {prompts[-1]:,} at the last "
              f"({len(prompts)} calls)")
        print(f"our tool answers: about {text + image:,.0f} tokens "
              f"({text:,.0f} text, {image:,.0f} in {images} images)")
        for name, (n, t, i, k) in sorted(by_tool.items(), key=lambda kv: -(kv[1][1] + kv[1][2]))[:10]:
            extra = f", {k} images" if k else ""
            print(f"  {name}: {n} calls, about {t + i:,.0f} tokens{extra}")
    verdict = "within" if end <= TARGET else "over"
    print(f"\nThe edit's context at its last call: {end:,} tokens, {verdict} the {TARGET:,} aim "
          "(Claude Code's own prompt included).")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    main(sys.argv[1:])
