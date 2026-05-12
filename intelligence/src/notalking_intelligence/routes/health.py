from __future__ import annotations

import logging
from typing import Any

import grpc
from fastapi import APIRouter, Request
from grpc.aio import AioRpcError
from prometheus_client import CONTENT_TYPE_LATEST, generate_latest
from starlette.responses import Response

from notalking_intelligence.metrics import registry

logger = logging.getLogger(__name__)

router = APIRouter(tags=["health"])


@router.get("/health")
async def health(request: Request) -> dict[str, Any]:
    """Liveness for orchestrators and the Web app."""
    bridge = getattr(request.app.state, "core_bridge", None)
    core_grpc = None
    if bridge is not None:
        try:
            status = await bridge.health_check()
            core_grpc = {"status": status}
        except AioRpcError as e:
            if e.code() == grpc.StatusCode.UNAVAILABLE:
                logger.debug("Core gRPC unavailable (start Core with CORE_GRPC_BIND): %s", e.details())
                core_grpc = {"status": "unavailable"}
            else:
                logger.warning("Core gRPC health failed: %s", e.details())
                core_grpc = {"error": e.details()[:400]}
        except Exception as e:
            logger.warning("Core gRPC health failed: %s", e)
            core_grpc = {"error": str(e)[:400]}
    return {"status": "ok", "core_grpc": core_grpc}


@router.get("/metrics")
async def prometheus_metrics() -> Response:
    data = generate_latest(registry())
    return Response(content=data, media_type=CONTENT_TYPE_LATEST)
