from __future__ import annotations

from fastapi import APIRouter, Request
from starlette.responses import StreamingResponse

from notalking_intelligence.adapters.v2t_stub import default_v2t_backend
from notalking_intelligence.deps import IdentityDep
from notalking_intelligence.http_utils import attach_core_cookies

router = APIRouter(prefix="/v2t", tags=["v2t"])


@router.post("/transcribe/stream")
async def transcribe_stream(request: Request, identity: IdentityDep) -> StreamingResponse:
    """Stream partial transcripts (SSE). Body is raw audio bytes (format chosen per deployment)."""
    _ = identity

    async def audio_chunks():
        async for chunk in request.stream():
            yield chunk

    backend = default_v2t_backend()

    async def gen():
        async for line in backend.transcribe_stream(audio_chunks()):
            yield f"data: {line}\n\n".encode()

    resp = StreamingResponse(gen(), media_type="text/event-stream")
    return attach_core_cookies(resp, request)
