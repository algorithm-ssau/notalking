from __future__ import annotations

import json
import uuid
from datetime import datetime, timezone
from typing import Annotated, Any

from fastapi import APIRouter, Depends, HTTPException, Request
from fastapi.responses import JSONResponse
from pydantic import BaseModel, Field
from sqlalchemy import delete, select
from sqlalchemy.ext.asyncio import AsyncSession
from starlette.responses import Response

from notalking_intelligence.db.models import ProviderRecord
from notalking_intelligence.deps import IdentityDep
from notalking_intelligence.http_utils import attach_core_cookies, json_response
from notalking_intelligence.provider_kinds import ALLOWED_PROVIDER_KINDS, catalog_entries

router = APIRouter(prefix="/providers", tags=["providers"])


class ProviderOut(BaseModel):
    id: str
    kind: str
    display_name: str
    config: dict[str, Any]
    created_at: str
    updated_at: str


class ProviderCreate(BaseModel):
    kind: str = Field(min_length=1, max_length=64)
    display_name: str = Field(min_length=1, max_length=256)
    config: dict[str, Any] = Field(default_factory=dict)


class ProviderPatch(BaseModel):
    display_name: str | None = None
    config: dict[str, Any] | None = None


async def get_session(request: Request):
    factory = request.app.state.session_factory
    async with factory() as session:
        yield session


SessionDep = Annotated[AsyncSession, Depends(get_session)]


def _sanitize_config(cfg: dict[str, Any]) -> dict[str, Any]:
    out = dict(cfg)
    for key in ("token", "github_token", "api_key"):
        if key in out and out[key]:
            out[key] = "***"
    return out


def _to_out(rec: ProviderRecord) -> ProviderOut:
    raw = json.loads(rec.config_json or "{}")
    return ProviderOut(
        id=str(rec.id),
        kind=rec.kind,
        display_name=rec.display_name,
        config=_sanitize_config(raw),
        created_at=rec.created_at.isoformat(),
        updated_at=rec.updated_at.isoformat(),
    )


def _validate_kind(kind: str) -> None:
    if kind not in ALLOWED_PROVIDER_KINDS:
        raise HTTPException(
            status_code=422,
            detail={
                "code": "invalid_provider_kind",
                "message": f"kind must be one of: {', '.join(sorted(ALLOWED_PROVIDER_KINDS))}",
            },
        )


@router.get("/catalog")
async def provider_catalog() -> list[dict[str, Any]]:
    """Describe supported provider kinds (no auth)."""
    return catalog_entries()


@router.get("")
async def list_providers(
    request: Request,
    identity: IdentityDep,
    session: SessionDep,
) -> JSONResponse:
    result = await session.execute(select(ProviderRecord).where(ProviderRecord.user_id == str(identity.user_id)))
    rows = result.scalars().all()
    payload = [_to_out(r).model_dump() for r in rows]
    return json_response(payload, request)


@router.post("", status_code=201)
async def create_provider(
    request: Request,
    identity: IdentityDep,
    session: SessionDep,
    body: ProviderCreate,
) -> JSONResponse:
    _validate_kind(body.kind)
    rec = ProviderRecord(
        user_id=str(identity.user_id),
        kind=body.kind,
        display_name=body.display_name,
        config_json=json.dumps(body.config),
    )
    session.add(rec)
    await session.commit()
    await session.refresh(rec)
    return json_response(_to_out(rec).model_dump(), request)


@router.patch("/{provider_id}")
async def patch_provider(
    request: Request,
    identity: IdentityDep,
    session: SessionDep,
    provider_id: uuid.UUID,
    body: ProviderPatch,
) -> JSONResponse:
    result = await session.execute(
        select(ProviderRecord).where(
            ProviderRecord.id == provider_id,
            ProviderRecord.user_id == str(identity.user_id),
        )
    )
    rec = result.scalar_one_or_none()
    if rec is None:
        raise HTTPException(status_code=404, detail={"code": "not_found", "message": "provider not found"})
    if body.display_name is not None:
        rec.display_name = body.display_name
    if body.config is not None:
        rec.config_json = json.dumps(body.config)
    rec.updated_at = datetime.now(timezone.utc)
    await session.commit()
    await session.refresh(rec)
    return json_response(_to_out(rec).model_dump(), request)


@router.delete("/{provider_id}", status_code=204)
async def delete_provider(
    request: Request,
    identity: IdentityDep,
    session: SessionDep,
    provider_id: uuid.UUID,
) -> Response:
    result = await session.execute(
        delete(ProviderRecord).where(
            ProviderRecord.id == provider_id,
            ProviderRecord.user_id == str(identity.user_id),
        )
    )
    await session.commit()
    if result.rowcount == 0:
        raise HTTPException(status_code=404, detail={"code": "not_found", "message": "provider not found"})
    return attach_core_cookies(Response(status_code=204), request)
