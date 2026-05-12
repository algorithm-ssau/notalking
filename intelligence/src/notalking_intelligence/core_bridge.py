"""gRPC client for Core `CoreBridge` (notalking.v1)."""

from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import TYPE_CHECKING

import grpc.aio
from notalking.v1 import core_pb2, core_pb2_grpc

if TYPE_CHECKING:
    from notalking_intelligence.config import Settings

logger = logging.getLogger(__name__)


@dataclass(frozen=True)
class CoreNoteSearchHit:
    note_id: str
    title: str
    matched_by: str
    score: float
    excerpt: str
    block_id: str


@dataclass(frozen=True)
class CoreNoteBlock:
    block_id: str
    order: int
    plain_text: str


@dataclass(frozen=True)
class CoreNoteContent:
    note_id: str
    title: str
    head_block_id: str
    blocks: list[CoreNoteBlock]


@dataclass(frozen=True)
class CoreCreatedNote:
    note_id: str
    title: str
    head_block_id: str


class CoreBridgeClient:
    def __init__(self, target: str) -> None:
        self._target = target
        self._channel: grpc.aio.Channel | None = None

    async def connect(self) -> None:
        if self._channel is None:
            self._channel = grpc.aio.insecure_channel(self._target)

    async def close(self) -> None:
        if self._channel is not None:
            await self._channel.close()
            self._channel = None

    def _stub(self) -> core_pb2_grpc.CoreBridgeStub:
        if self._channel is None:
            raise RuntimeError("CoreBridgeClient not connected")
        return core_pb2_grpc.CoreBridgeStub(self._channel)

    async def health_check(self) -> str:
        await self.connect()
        stub = self._stub()
        resp = await stub.HealthCheck(core_pb2.HealthCheckRequest(), timeout=10.0)
        return resp.status

    async def get_note_context(self, user_id: str, note_id: str) -> tuple[str, str]:
        await self.connect()
        stub = self._stub()
        req = core_pb2.GetNoteContextRequest(user_id=user_id, note_id=note_id)
        resp = await stub.GetNoteContext(req, timeout=30.0)
        return resp.title, resp.head_block_id

    async def search_notes(self, user_id: str, query: str, limit: int = 8) -> list[CoreNoteSearchHit]:
        await self.connect()
        stub = self._stub()
        req = core_pb2.SearchNotesRequest(user_id=user_id, query=query, limit=limit)
        resp = await stub.SearchNotes(req, timeout=30.0)
        return [
            CoreNoteSearchHit(
                note_id=hit.note_id,
                title=hit.title,
                matched_by=hit.matched_by,
                score=hit.score,
                excerpt=hit.excerpt,
                block_id=hit.block_id,
            )
            for hit in resp.hits
        ]

    async def get_note_content(
        self,
        user_id: str,
        note_id: str,
        *,
        max_blocks: int = 40,
        max_chars_per_block: int = 2000,
    ) -> CoreNoteContent:
        await self.connect()
        stub = self._stub()
        req = core_pb2.GetNoteContentRequest(
            user_id=user_id,
            note_id=note_id,
            max_blocks=max_blocks,
            max_chars_per_block=max_chars_per_block,
        )
        resp = await stub.GetNoteContent(req, timeout=30.0)
        return CoreNoteContent(
            note_id=resp.note_id,
            title=resp.title,
            head_block_id=resp.head_block_id,
            blocks=[
                CoreNoteBlock(
                    block_id=block.block_id,
                    order=block.order,
                    plain_text=block.plain_text,
                )
                for block in resp.blocks
            ],
        )

    async def create_note(self, user_id: str, title: str, initial_text: str = "") -> CoreCreatedNote:
        await self.connect()
        stub = self._stub()
        req = core_pb2.CreateNoteRequest(user_id=user_id, title=title, initial_text=initial_text)
        resp = await stub.CreateNote(req, timeout=30.0)
        return CoreCreatedNote(
            note_id=resp.note_id,
            title=resp.title,
            head_block_id=resp.head_block_id,
        )


def client_from_settings(settings: Settings) -> CoreBridgeClient | None:
    url = settings.core_grpc_url
    if not url or not url.strip():
        return None
    return CoreBridgeClient(url.strip())
