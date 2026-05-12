"""Attach Core session refresh cookies to outgoing responses."""

from __future__ import annotations

from typing import Any

from fastapi.responses import JSONResponse
from starlette.requests import Request
from starlette.responses import Response


def attach_core_cookies(response: Response, request: Request) -> Response:
    for c in getattr(request.state, "core_set_cookies", []):
        response.headers.append("set-cookie", c)
    return response


def json_response(payload: Any, request: Request) -> JSONResponse:
    return attach_core_cookies(JSONResponse(payload), request)  # type: ignore[arg-type]
