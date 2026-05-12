"""OpenAI-compatible streaming chat (Ollama, GitHub Models, and similar endpoints)."""

from __future__ import annotations

import asyncio
import json
import logging
from collections.abc import AsyncIterator
from typing import Any

import httpx

logger = logging.getLogger(__name__)


def _delta_content(chunk: dict[str, Any]) -> str | None:
    """Extract assistant delta text from OpenAI-style or GitHub Models streaming JSON."""
    choices = chunk.get("choices")
    if choices is None:
        data = chunk.get("data")
        if isinstance(data, dict):
            choices = data.get("choices")
    if not choices:
        return None
    first = choices[0]
    if not isinstance(first, dict):
        return None
    delta = first.get("delta") or {}
    if not isinstance(delta, dict):
        return None
    content = delta.get("content")
    return content if isinstance(content, str) else None


async def stream_chat_completion(
    *,
    url: str,
    model: str,
    messages: list[dict[str, Any]],
    cancel_event: asyncio.Event | None = None,
    extra_headers: dict[str, str] | None = None,
) -> AsyncIterator[str]:
    """POST JSON {"model","messages","stream":true} and yield assistant token strings from SSE lines."""
    payload: dict[str, Any] = {"model": model, "messages": messages, "stream": True}
    headers = {"Content-Type": "application/json", **(extra_headers or {})}

    async with httpx.AsyncClient(
        timeout=httpx.Timeout(120.0, connect=30.0),
        trust_env=False,
    ) as client:
        async with client.stream("POST", url, json=payload, headers=headers) as response:
            response.raise_for_status()
            async for line in response.aiter_lines():
                if cancel_event is not None and cancel_event.is_set():
                    logger.info("chat stream cancelled")
                    break
                line = line.strip()
                if not line or line == "data: [DONE]":
                    if line == "data: [DONE]":
                        break
                    continue
                if line.startswith("data:"):
                    line = line[5:].strip()
                try:
                    chunk = json.loads(line)
                except json.JSONDecodeError:
                    continue
                text = _delta_content(chunk)
                if text:
                    yield text
