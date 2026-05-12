"""Pluggable voice-to-text backend (SPEC §6.2) -- stub for development."""

from __future__ import annotations

import asyncio
import json
import logging
from collections.abc import AsyncIterator
from typing import Protocol, runtime_checkable

logger = logging.getLogger(__name__)


@runtime_checkable
class V2tBackend(Protocol):
    async def transcribe_stream(self, audio_chunks: AsyncIterator[bytes]) -> AsyncIterator[str]:
        """Yield partial transcript strings; final chunk may repeat full text."""


class StubV2tBackend:
    """No-op STT: yields a single placeholder line for contract testing."""

    async def transcribe_stream(self, audio_chunks: AsyncIterator[bytes]) -> AsyncIterator[str]:
        _acc = bytearray()
        async for chunk in audio_chunks:
            _acc.extend(chunk)
            await asyncio.sleep(0)
        if _acc:
            yield json.dumps({"partial": f"received {len(_acc)} bytes (stub V2T)"})
        else:
            yield json.dumps({"partial": ""})


def default_v2t_backend() -> V2tBackend:
    return StubV2tBackend()
