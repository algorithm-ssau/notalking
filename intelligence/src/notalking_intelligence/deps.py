"""Authentication via Core session cookie forwarding."""

from __future__ import annotations

import json
import uuid
from typing import Annotated
from urllib.parse import urlparse

import httpx
from fastapi import Depends, HTTPException, Request
from pydantic import BaseModel


DEV_CORE_HTTP_URLS = ("http://127.0.0.1:40000", "http://localhost:40000")
RETRYABLE_CORE_AUTH_STATUS_CODES = {404, 502, 503, 504}


class CoreIdentity(BaseModel):
    user_id: uuid.UUID
    session_id: uuid.UUID


def extract_set_cookie_values(response: httpx.Response) -> list[str]:
    out: list[str] = []
    for key, value in response.headers.multi_items():
        if key.lower() == "set-cookie":
            out.append(value)
    return out


def _core_http_candidates(configured: str, environment: str) -> list[str]:
    candidates: list[str] = []

    def add(url: str) -> None:
        clean = url.rstrip("/")
        if clean and clean not in candidates:
            candidates.append(clean)

    add(configured)

    parsed = urlparse(configured)
    host = parsed.hostname or ""
    looks_like_local_dev = host in {"127.0.0.1", "localhost", "0.0.0.0"} or parsed.port in {3000, 3001, 40000}
    has_proxy_path = parsed.path not in {"", "/"}

    if environment == "dev" and (looks_like_local_dev or has_proxy_path):
        for url in DEV_CORE_HTTP_URLS:
            add(url)

    return candidates


def _attempt_preview(response: httpx.Response) -> str:
    return (response.text or "")[:400]


def _attempts_detail(attempts: list[dict[str, object]]) -> list[dict[str, object]]:
    return attempts[-5:]


def _core_auth_failure(response: httpx.Response, attempts: list[dict[str, object]]) -> HTTPException:
    if response.status_code == 401:
        return HTTPException(status_code=401, detail={"code": "invalid_session", "message": "session invalid or expired"})
    if response.status_code == 429:
        return HTTPException(
            status_code=429,
            detail={"code": "core_rate_limited", "message": "Core rate limit; retry later"},
        )

    return HTTPException(
        status_code=502,
        detail={
            "code": "core_auth_error",
            "message": (
                f"Core auth check failed: {response.url} returned HTTP {response.status_code}. "
                "Check INTELLIGENCE_CORE_HTTP_URL; it must point at Core's HTTP origin, not an unavailable proxy."
            ),
            "core_response_preview": _attempt_preview(response),
            "attempts": _attempts_detail(attempts),
        },
    )


async def require_identity(request: Request) -> CoreIdentity:
    """Validate browser session by forwarding cookies to Core `/auth/me`."""
    settings = request.app.state.settings
    cookie_header = request.headers.get("cookie")
    if not cookie_header:
        raise HTTPException(status_code=401, detail={"code": "missing_session", "message": "missing session cookie"})

    candidates = _core_http_candidates(settings.core_http_url, settings.environment)
    attempts: list[dict[str, object]] = []
    first_response: httpx.Response | None = None
    first_error: Exception | None = None
    successful_response: httpx.Response | None = None

    # Internal Core calls must never be routed through HTTP_PROXY/ALL_PROXY.
    async with httpx.AsyncClient(trust_env=False) as client:
        for base_url in candidates:
            url = f"{base_url}/auth/me"
            try:
                r = await client.get(url, headers={"Cookie": cookie_header}, timeout=30.0)
            except httpx.ConnectError as e:
                attempts.append({"url": url, "error": str(e)})
                if first_error is None:
                    first_error = e
                continue
            except httpx.TimeoutException as e:
                attempts.append({"url": url, "error": "timeout", "cause": str(e)})
                if first_error is None:
                    first_error = e
                continue

            attempts.append({"url": str(r.url), "status": r.status_code, "preview": _attempt_preview(r)})
            if first_response is None:
                first_response = r

            if r.status_code == 200:
                successful_response = r
                break

            # In local dev, a configured /core proxy or wrong path can fail while direct Core is healthy.
            if r.status_code in RETRYABLE_CORE_AUTH_STATUS_CODES:
                continue
            break

    if successful_response is not None:
        r = successful_response
        request.state.core_set_cookies = extract_set_cookie_values(r)
    else:
        r = first_response

    if r is None and first_error is not None:
        status_code = 504 if isinstance(first_error, httpx.TimeoutException) else 503
        raise HTTPException(
            status_code=status_code,
            detail={
                "code": "core_timeout" if status_code == 504 else "core_unreachable",
                "message": (
                    f"Cannot reach Core from Intelligence. Start Core and set INTELLIGENCE_CORE_HTTP_URL "
                    f"to the Core HTTP origin (current value: {settings.core_http_url})."
                ),
                "attempts": _attempts_detail(attempts),
            },
        ) from first_error

    if r is None:
        raise HTTPException(
            status_code=503,
            detail={
                "code": "core_unreachable",
                "message": "No Core HTTP URL candidates were available.",
                "attempts": _attempts_detail(attempts),
            },
        )

    if r.status_code != 200:
        raise _core_auth_failure(r, attempts)

    try:
        data = r.json()
    except json.JSONDecodeError:
        raise HTTPException(
            status_code=502,
            detail={
                "code": "core_auth_parse",
                "message": "Core /auth/me returned non-JSON (wrong URL or Core version?)",
                "core_response_preview": (r.text or "")[:400],
            },
        ) from None

    try:
        return CoreIdentity(user_id=uuid.UUID(data["user_id"]), session_id=uuid.UUID(data["session_id"]))
    except (KeyError, ValueError) as e:
        raise HTTPException(status_code=502, detail={"code": "core_auth_parse", "message": str(e)}) from e


IdentityDep = Annotated[CoreIdentity, Depends(require_identity)]
