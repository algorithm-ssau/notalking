from __future__ import annotations

import json
import logging
import re
import uuid
from dataclasses import dataclass
from typing import Annotated, Any, AsyncIterator, Literal

import httpx
from fastapi import APIRouter, Depends, HTTPException, Request
from pydantic import BaseModel, Field
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession
from starlette.responses import StreamingResponse

from notalking_intelligence.adapters.openai_stream import stream_chat_completion
from notalking_intelligence.chat.registry import registry
from notalking_intelligence.db.models import ProviderRecord
from notalking_intelligence.deps import IdentityDep
from notalking_intelligence.github_models import inference_headers, inference_url
from notalking_intelligence.http_utils import attach_core_cookies, json_response
from notalking_intelligence.provider_kinds import is_github_family
from notalking_intelligence.routes.providers import get_session

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/chat", tags=["chat"])

SessionDep = Annotated[AsyncSession, Depends(get_session)]

OLLAMA_CHAT_PATH = "/v1/chat/completions"
OPENAI_CHAT_PATH = "/chat/completions"
NOTE_CONTEXT_SEARCH_LIMIT = 6
NOTE_CONTEXT_FETCH_LIMIT = 4
NOTE_CONTEXT_TOTAL_CHARS = 16_000
NOTE_CREATE_MAX_TITLE_CHARS = 120
NOTE_CREATE_MAX_BODY_CHARS = 24_000
SUPER_PROMPT_MAX_CHARS = 12_000

CORE_TOOL_METHODS = {
    "search_notes": {
        "method": "CoreBridge.SearchNotes",
        "mcp_method": "find_note_by_title / semantic_search",
    },
    "get_note_content": {
        "method": "CoreBridge.GetNoteContent",
        "mcp_method": "get_note_content",
    },
    "create_note": {
        "method": "CoreBridge.CreateNote",
        "mcp_method": "create_note",
    },
    "update_note": {
        "method": "CoreBridge.UpdateNote",
        "mcp_method": "update_note",
    },
}


class ChatMessage(BaseModel):
    role: Literal["system", "user", "assistant"]
    content: str


class ChatStreamRequest(BaseModel):
    messages: list[ChatMessage] = Field(min_length=1)
    note_id: str | None = None
    provider_id: uuid.UUID | None = None
    super_prompt: str | None = Field(default=None, max_length=SUPER_PROMPT_MAX_CHARS)


@dataclass(frozen=True)
class NoteWriteIntent:
    kind: Literal["create", "append", "replace", "rename"]
    source: str
    body: str = ""
    next_title: str | None = None
    target_mode: Literal["current", "title"] | None = None
    target_query: str | None = None


class NoteWritePreview(BaseModel):
    kind: Literal["create", "append", "replace", "rename"]
    source: str
    message: str
    target_note_id: str | None = None
    current_title: str | None = None
    next_title: str
    current_body: str | None = None
    next_body: str | None = None


class NoteActionApplyRequest(BaseModel):
    preview: NoteWritePreview


class NoteActionApplyResponse(BaseModel):
    action: Literal["note_write_applied"]
    kind: Literal["create", "append", "replace", "rename"]
    note_id: str
    source: str
    title: str
    head_block_id: str | None = None


def _resolve_default_ollama(settings: Any) -> tuple[str, str, dict[str, str] | None]:
    url = _openai_compatible_chat_url(settings.ollama_base_url)
    return url, settings.ollama_model, None


def _openai_compatible_chat_url(base_url: str) -> str:
    base = base_url.rstrip("/")
    if base.endswith("/v1"):
        return base + OPENAI_CHAT_PATH
    return base + OLLAMA_CHAT_PATH


def _latest_user_text(messages: list[dict[str, Any]]) -> str:
    for message in reversed(messages):
        if message.get("role") == "user" and isinstance(message.get("content"), str):
            return str(message["content"]).strip()
    return ""


def _previous_assistant_text(messages: list[dict[str, Any]]) -> str:
    for message in reversed(messages[:-1]):
        if message.get("role") == "assistant" and isinstance(message.get("content"), str):
            text = str(message["content"]).strip()
            if text:
                return text
    return ""


def _looks_like_note_request(text: str) -> bool:
    lower = text.lower()
    return any(
        marker in lower
        for marker in (
            "note",
            "notes",
            "notalking",
            "knowledge",
            "remember",
            "search",
            "find",
            "look up",
            "document",
        )
    )


def _trim(value: str, max_chars: int) -> str:
    value = value.strip()
    if len(value) <= max_chars:
        return value
    return value[: max_chars - 3].rstrip() + "..."


def _sse(payload: dict[str, Any]) -> str:
    return f"data: {json.dumps(payload)}\n\n"


def _tool_event(
    name: str,
    phase: Literal["start", "done", "error"],
    *,
    call_id: str | None = None,
    message: str,
    **payload: Any,
) -> dict[str, Any]:
    methods = CORE_TOOL_METHODS.get(name, {})
    event = {
        "type": "tool",
        "source": "core_bridge",
        "name": name,
        "phase": phase,
        "call_id": call_id or uuid.uuid4().hex,
        "message": message,
        **methods,
    }
    event.update({k: v for k, v in payload.items() if v is not None})
    return event


def _note_hit_payload(hit: Any) -> dict[str, Any]:
    return {
        "note_id": hit.note_id,
        "title": hit.title,
        "matched_by": hit.matched_by,
        "score": round(float(hit.score), 3),
        "excerpt": _trim(hit.excerpt, 180) if hit.excerpt else "",
        "block_id": hit.block_id or None,
    }


def _note_content_minimal_response(note: Any) -> dict[str, Any]:
    blocks = [
        {
            "block_id": block.block_id,
            "order": block.order,
            "chars": len(block.plain_text),
        }
        for block in note.blocks[:5]
    ]
    remaining = max(0, len(note.blocks) - len(blocks))
    return {
        "note_id": note.note_id,
        "title": note.title,
        "head_block_id": note.head_block_id or None,
        "block_count": len(note.blocks),
        "blocks": blocks,
        "truncated_blocks": remaining,
    }


def _format_super_prompt(value: str | None) -> str | None:
    prompt = (value or "").strip()
    if not prompt:
        return None
    return (
        "USER SUPER PROMPT\n"
        "These hidden user instructions apply to every visible message in this chat. "
        "They are configured in Notalking settings and are not part of the visible user query.\n\n"
        f"{_trim(prompt, SUPER_PROMPT_MAX_CHARS)}"
    )


def _clean_note_title(value: str) -> str:
    title = re.sub(r"\s+", " ", value.strip().strip("\"'`“”‘’")).strip(" .:-")
    return _trim(title, NOTE_CREATE_MAX_TITLE_CHARS)


def _title_from_body(body: str) -> str:
    for raw_line in body.splitlines():
        line = raw_line.strip()
        if not line:
            continue
        line = re.sub(r"^#{1,6}\s+", "", line)
        line = re.sub(r"^[-*]\s+", "", line)
        return _clean_note_title(line)
    return "Agent note"


def _split_title_body(value: str) -> tuple[str, str]:
    text = value.strip()
    if "\n" in text:
        title, body = text.split("\n", 1)
        return _clean_note_title(title), body.strip()

    for sep in (" with content ", " with body ", " with text ", " containing "):
        idx = text.lower().find(sep)
        if idx >= 0:
            return _clean_note_title(text[:idx]), text[idx + len(sep):].strip(" :")

    if ": " in text:
        title, body = text.split(": ", 1)
        return _clean_note_title(title), body.strip()

    return _clean_note_title(text), ""


def _detect_create_note_action(messages: list[dict[str, Any]]) -> NoteWriteIntent | None:
    latest = _latest_user_text(messages)
    if not latest:
        return None

    lower = latest.lower()
    if any(marker in lower for marker in ("don't create", "do not create", "don't save", "do not save")):
        return None

    if re.search(r"\b(create|add|make|write|save)\b.*\b(note|document)\b", lower):
        title_match = re.search(
            r"\b(?:called|named|titled|with title|title)\s+[\"'“”‘’`]?(.+?)[\"'“”‘’`]?(?:\s+(?:with|containing|about)\b|[:\n]|$)",
            latest,
            flags=re.IGNORECASE | re.DOTALL,
        )
        if title_match:
            remainder = latest[title_match.end():].strip()
            body = ""
            body_match = re.search(
                r"\b(?:with\s+)?(?:content|body|text)\b\s*:?\s*(.+)$",
                remainder,
                flags=re.IGNORECASE | re.DOTALL,
            )
            if body_match:
                body = body_match.group(1).strip()
            elif remainder.lower().startswith("about "):
                body = remainder[6:].strip()
            return NoteWriteIntent(
                kind="create",
                next_title=_clean_note_title(title_match.group(1)),
                body=_trim(body, NOTE_CREATE_MAX_BODY_CHARS),
                source="explicit",
            )

        after_note_match = re.search(r"\bnote\b\s*[:\-]?\s*(.+)$", latest, flags=re.IGNORECASE | re.DOTALL)
        if after_note_match:
            after_note = after_note_match.group(1).strip()
            after_note_lower = after_note.lower()
            if after_note_lower.startswith("about ") or after_note_lower.startswith("for "):
                title = _clean_note_title(after_note.split(" ", 1)[1])
                body = after_note.split(" ", 1)[1].strip()
            else:
                title, body = _split_title_body(after_note)
            if title:
                return NoteWriteIntent(
                    kind="create",
                    next_title=title,
                    body=_trim(body, NOTE_CREATE_MAX_BODY_CHARS),
                    source="explicit",
                )

        about_match = re.search(r"\b(?:about|for)\s+(.+)$", latest, flags=re.IGNORECASE | re.DOTALL)
        if about_match:
            title = _clean_note_title(about_match.group(1))
            if title:
                return NoteWriteIntent(
                    kind="create",
                    next_title=title,
                    body=_trim(about_match.group(1).strip(), NOTE_CREATE_MAX_BODY_CHARS),
                    source="explicit",
                )

    if re.search(r"\b(create|save|add)\s+(it|this)\s+(as\s+)?(a\s+)?note\b", lower) or re.fullmatch(
        r"\s*(create|save|add)\s+(it|this)\s*\.?\s*",
        lower,
    ):
        previous = _previous_assistant_text(messages)
        if previous:
            return NoteWriteIntent(
                kind="create",
                next_title=_title_from_body(previous),
                body=_trim(previous, NOTE_CREATE_MAX_BODY_CHARS),
                source="previous_assistant",
            )

    return None


def _normalize_title_for_match(value: str) -> str:
    return re.sub(r"\s+", " ", value.strip().strip("\"'`“”‘’")).lower()


def _plain_text_from_note(note: Any) -> str:
    return "\n\n".join(block.plain_text for block in note.blocks if block.plain_text).strip()


def _explicit_note_target(text: str) -> str | None:
    patterns = (
        r"\bnote\s+(?:called|named|titled)\s+[\"'“”‘’`]?(.+?)[\"'“”‘’`]?(?:\s+(?:with|to|into|for)\b|[:\n]|$)",
        r"\bnote\s+[\"'“”‘’`]?(.+?)[\"'“”‘’`]?(?:\s+(?:with|to|into|for)\b|[:\n]|$)",
    )
    for pattern in patterns:
        match = re.search(pattern, text, flags=re.IGNORECASE | re.DOTALL)
        if match:
            title = _clean_note_title(match.group(1))
            if title:
                return title
    return None


def _extract_note_write_intent(messages: list[dict[str, Any]], current_note_id: str | None) -> NoteWriteIntent | None:
    latest = _latest_user_text(messages)
    if not latest:
        return None

    lower = latest.lower()
    previous = _previous_assistant_text(messages)
    create = _detect_create_note_action(messages)
    if create is not None:
        return create

    current_markers = ("current note", "this note", "open note")
    target_mode: Literal["current", "title"] | None = None
    target_query: str | None = None
    if any(marker in lower for marker in current_markers):
        target_mode = "current"
    else:
        explicit_target = _explicit_note_target(latest)
        if explicit_target:
            target_mode = "title"
            target_query = explicit_target
        elif current_note_id and "note" in lower:
            target_mode = "current"

    rename_match = re.search(
        r"\b(?:rename|retitle|change the title of|update the title of)\b.+?\bnote\b.+?\b(?:to|as)\b\s+(.+)$",
        latest,
        flags=re.IGNORECASE | re.DOTALL,
    )
    if rename_match and target_mode:
        next_title = _clean_note_title(rename_match.group(1))
        if next_title:
            return NoteWriteIntent(
                kind="rename",
                source="explicit",
                next_title=next_title,
                target_mode=target_mode,
                target_query=target_query,
            )

    replace_match = re.search(
        r"\b(?:replace|overwrite|rewrite)\b.+?\bnote\b.+?(?:with|using|to)\s*:?\s*(.+)$",
        latest,
        flags=re.IGNORECASE | re.DOTALL,
    )
    if replace_match and target_mode:
        body = replace_match.group(1).strip() or previous
        if body:
            return NoteWriteIntent(
                kind="replace",
                source="explicit" if replace_match.group(1).strip() else "previous_assistant",
                body=_trim(body, NOTE_CREATE_MAX_BODY_CHARS),
                target_mode=target_mode,
                target_query=target_query,
            )

    append_prefix_match = re.search(
        r"\b(?:append|add|write)\s+to\s+(?:the\s+)?(?:current|this|open)\s+note\s*:?\s*(.+)$",
        latest,
        flags=re.IGNORECASE | re.DOTALL,
    )
    if append_prefix_match and current_note_id:
        body = append_prefix_match.group(1).strip() or previous
        if body:
            return NoteWriteIntent(
                kind="append",
                source="explicit" if append_prefix_match.group(1).strip() else "previous_assistant",
                body=_trim(body, NOTE_CREATE_MAX_BODY_CHARS),
                target_mode="current",
            )

    append_suffix_match = re.search(
        r"\b(?:append|add|write)\b\s+(.+?)\s+\b(?:to|into)\b.+?\bnote\b",
        latest,
        flags=re.IGNORECASE | re.DOTALL,
    )
    if append_suffix_match and target_mode:
        body = append_suffix_match.group(1).strip() or previous
        if body:
            return NoteWriteIntent(
                kind="append",
                source="explicit" if append_suffix_match.group(1).strip() else "previous_assistant",
                body=_trim(body, NOTE_CREATE_MAX_BODY_CHARS),
                target_mode=target_mode,
                target_query=target_query,
            )

    if previous and target_mode and re.search(r"\b(save|add|append|write)\s+(it|this)\s+(to|into)\b.+?\bnote\b", lower):
        return NoteWriteIntent(
            kind="append",
            source="previous_assistant",
            body=_trim(previous, NOTE_CREATE_MAX_BODY_CHARS),
            target_mode=target_mode,
            target_query=target_query,
        )

    return None


def _format_note_blocks(note: Any, *, heading: str) -> str:
    lines = [
        f"{heading}: {note.title} (note_id={note.note_id}, head_block_id={note.head_block_id or 'none'})",
    ]
    non_empty = 0
    for block in note.blocks:
        text = _trim(block.plain_text, 1_400)
        if not text:
            continue
        non_empty += 1
        lines.append(f"- block {block.block_id}: {text}")
    if non_empty == 0:
        lines.append("- no text blocks")
    return "\n".join(lines)


async def _resolve_note_for_preview(
    *,
    bridge: Any,
    user_id: str,
    current_note_id: str | None,
    intent: NoteWriteIntent,
) -> tuple[Any | None, str | None, list[dict[str, Any]]]:
    tool_events: list[dict[str, Any]] = []
    errors: list[str] = []

    if bridge is None:
        return None, "INTELLIGENCE_CORE_GRPC_URL is not configured.", errors

    if intent.target_mode == "current":
        if not current_note_id:
            return None, "There is no open note to edit.", errors
        call_id = uuid.uuid4().hex
        request_payload = {
            "note_id": current_note_id,
            "reason": "preview_target",
            "max_blocks": 80,
            "max_chars_per_block": 4_000,
        }
        tool_events.append(
            _tool_event(
                "get_note_content",
                "start",
                call_id=call_id,
                note_id=current_note_id,
                reason="preview_target",
                message="Reading the current note for an edit preview.",
                request=request_payload,
                minimal_response={"status": "running"},
            )
        )
        try:
            note = await bridge.get_note_content(
                user_id,
                current_note_id,
                max_blocks=80,
                max_chars_per_block=4_000,
            )
        except Exception as exc:
            tool_events.append(
                _tool_event(
                    "get_note_content",
                    "error",
                    call_id=call_id,
                    note_id=current_note_id,
                    reason="preview_target",
                    message=f"Could not read the current note: {exc}",
                    request=request_payload,
                    minimal_response={"ok": False, "error": str(exc)},
                    error=str(exc),
                )
            )
            return None, f"Could not read the current note: {exc}", tool_events

        tool_events.append(
            _tool_event(
                "get_note_content",
                "done",
                call_id=call_id,
                note_id=note.note_id,
                title=note.title,
                reason="preview_target",
                block_count=len(note.blocks),
                message=f'Read "{note.title}" for the edit preview.',
                request=request_payload,
                minimal_response=_note_content_minimal_response(note),
            )
        )
        return note, None, tool_events

    query = intent.target_query or ""
    call_id = uuid.uuid4().hex
    request_payload = {"query": query, "limit": 5}
    tool_events.append(
        _tool_event(
            "search_notes",
            "start",
            call_id=call_id,
            query=query,
            limit=5,
            message=f'Searching notes for "{_trim(query, 120)}" to resolve the edit target.',
            request=request_payload,
            minimal_response={"status": "running"},
        )
    )
    try:
        hits = await bridge.search_notes(user_id, query, limit=5)
    except Exception as exc:
        tool_events.append(
            _tool_event(
                "search_notes",
                "error",
                call_id=call_id,
                query=query,
                limit=5,
                message=f"Could not resolve the target note: {exc}",
                request=request_payload,
                minimal_response={"ok": False, "error": str(exc)},
                error=str(exc),
            )
        )
        return None, f"Could not resolve the target note: {exc}", tool_events

    hit_payloads = [_note_hit_payload(hit) for hit in hits]
    tool_events.append(
        _tool_event(
            "search_notes",
            "done",
            call_id=call_id,
            query=query,
            limit=5,
            count=len(hits),
            notes=hit_payloads,
            message=f"Resolved {len(hits)} candidate note{'s' if len(hits) != 1 else ''} for the edit target.",
            request=request_payload,
            minimal_response={"count": len(hits), "hits": hit_payloads},
        )
    )
    if not hits:
        return None, f'No note matched "{query}".', tool_events

    normalized_query = _normalize_title_for_match(query)
    best = next(
        (hit for hit in hits if _normalize_title_for_match(hit.title) == normalized_query),
        hits[0],
    )

    read_call_id = uuid.uuid4().hex
    read_payload = {
        "note_id": best.note_id,
        "title": best.title,
        "reason": "preview_target",
        "matched_by": best.matched_by,
        "max_blocks": 80,
        "max_chars_per_block": 4_000,
    }
    tool_events.append(
        _tool_event(
            "get_note_content",
            "start",
            call_id=read_call_id,
            note_id=best.note_id,
            title=best.title,
            reason="preview_target",
            matched_by=best.matched_by,
            message=f'Reading "{best.title}" for the edit preview.',
            request=read_payload,
            minimal_response={"status": "running"},
        )
    )
    try:
        note = await bridge.get_note_content(
            user_id,
            best.note_id,
            max_blocks=80,
            max_chars_per_block=4_000,
        )
    except Exception as exc:
        tool_events.append(
            _tool_event(
                "get_note_content",
                "error",
                call_id=read_call_id,
                note_id=best.note_id,
                title=best.title,
                reason="preview_target",
                matched_by=best.matched_by,
                message=f'Could not read "{best.title}": {exc}',
                request=read_payload,
                minimal_response={"ok": False, "error": str(exc)},
                error=str(exc),
            )
        )
        return None, f'Could not read "{best.title}": {exc}', tool_events

    tool_events.append(
        _tool_event(
            "get_note_content",
            "done",
            call_id=read_call_id,
            note_id=note.note_id,
            title=note.title,
            reason="preview_target",
            matched_by=best.matched_by,
            block_count=len(note.blocks),
            message=f'Read "{note.title}" for the edit preview.',
            request=read_payload,
            minimal_response=_note_content_minimal_response(note),
        )
    )
    return note, None, tool_events


async def _build_note_write_preview(
    *,
    bridge: Any,
    user_id: str,
    current_note_id: str | None,
    intent: NoteWriteIntent | None,
) -> tuple[NoteWritePreview | None, list[dict[str, Any]]]:
    if intent is None:
        return None, []

    if intent.kind == "create":
        title = intent.next_title or "Agent note"
        return (
            NoteWritePreview(
                kind="create",
                source=intent.source,
                message=f'Prepared a new note draft "{title}". Review it before applying.',
                next_title=title,
                next_body=intent.body,
            ),
            [],
        )

    target_note, error, tool_events = await _resolve_note_for_preview(
        bridge=bridge,
        user_id=user_id,
        current_note_id=current_note_id,
        intent=intent,
    )
    if target_note is None:
        if error:
            preview = NoteWritePreview(
                kind=intent.kind,
                source=intent.source,
                message=error,
                next_title=intent.next_title or "Untitled note",
                next_body=intent.body or None,
            )
            return preview, tool_events
        return None, tool_events

    current_body = _plain_text_from_note(target_note)
    if intent.kind == "rename":
        next_title = intent.next_title or target_note.title
        message = f'Ready to rename "{target_note.title}" to "{next_title}". Confirm to apply it.'
        preview = NoteWritePreview(
            kind="rename",
            source=intent.source,
            message=message,
            target_note_id=target_note.note_id,
            current_title=target_note.title,
            next_title=next_title,
            current_body=current_body,
        )
        return preview, tool_events

    next_body = intent.body
    if intent.kind == "append":
        next_body = f"{current_body}\n\n{intent.body}".strip() if current_body else intent.body
        message = f'Ready to append content to "{target_note.title}". Confirm to apply the note draft.'
    else:
        message = f'Ready to replace "{target_note.title}" with a rewritten draft. Confirm to apply it.'

    preview = NoteWritePreview(
        kind=intent.kind,
        source=intent.source,
        message=message,
        target_note_id=target_note.note_id,
        current_title=target_note.title,
        next_title=target_note.title,
        current_body=current_body,
        next_body=next_body,
    )
    return preview, tool_events


async def _stream_core_note_context(
    *,
    bridge: Any,
    user_id: str,
    current_note_id: str | None,
    latest_user_text: str,
    context_out: dict[str, str | None],
) -> AsyncIterator[dict[str, Any]]:
    context_out["value"] = None

    if bridge is None:
        if current_note_id:
            logger.debug(
                "Note id present but INTELLIGENCE_CORE_GRPC_URL unset; chat continues without Core note context",
            )
            message = "Could not read the current note because INTELLIGENCE_CORE_GRPC_URL is not configured."
            yield _tool_event(
                "get_note_content",
                "error",
                note_id=current_note_id,
                reason="current_note",
                message=message,
                request={
                    "note_id": current_note_id,
                    "reason": "current_note",
                    "max_blocks": 80,
                    "max_chars_per_block": 2_000,
                },
                minimal_response={"ok": False, "error": message},
                error=message,
            )
        if _looks_like_note_request(latest_user_text):
            message = "Could not search Core notes because INTELLIGENCE_CORE_GRPC_URL is not configured."
            yield _tool_event(
                "search_notes",
                "error",
                query=latest_user_text,
                message=message,
                request={"query": latest_user_text, "limit": NOTE_CONTEXT_SEARCH_LIMIT},
                minimal_response={"ok": False, "error": message},
                error=message,
            )
        return

    fetched_notes: dict[str, Any] = {}
    search_hits: list[Any] = []

    if current_note_id:
        call_id = uuid.uuid4().hex
        request_payload = {
            "note_id": current_note_id,
            "reason": "current_note",
            "max_blocks": 80,
            "max_chars_per_block": 2_000,
        }
        yield _tool_event(
            "get_note_content",
            "start",
            call_id=call_id,
            note_id=current_note_id,
            reason="current_note",
            message="Reading the current Core note.",
            request=request_payload,
            minimal_response={"status": "running"},
        )
        try:
            current = await bridge.get_note_content(
                user_id,
                current_note_id,
                max_blocks=80,
                max_chars_per_block=2_000,
            )
            fetched_notes[current.note_id] = current
            yield _tool_event(
                "get_note_content",
                "done",
                call_id=call_id,
                note_id=current.note_id,
                title=current.title,
                reason="current_note",
                block_count=len(current.blocks),
                message=f'Read current note "{current.title}" ({len(current.blocks)} blocks).',
                request=request_payload,
                minimal_response=_note_content_minimal_response(current),
            )
        except Exception as e:
            logger.warning("Skipping current note content from Core gRPC: %s", e)
            yield _tool_event(
                "get_note_content",
                "error",
                call_id=call_id,
                note_id=current_note_id,
                reason="current_note",
                message=f"Could not read the current Core note: {e}",
                request=request_payload,
                minimal_response={"ok": False, "error": str(e)},
                error=str(e),
            )

    if latest_user_text:
        call_id = uuid.uuid4().hex
        request_payload = {"query": latest_user_text, "limit": NOTE_CONTEXT_SEARCH_LIMIT}
        yield _tool_event(
            "search_notes",
            "start",
            call_id=call_id,
            query=latest_user_text,
            limit=NOTE_CONTEXT_SEARCH_LIMIT,
            message=f'Searching Core notes for "{_trim(latest_user_text, 120)}".',
            request=request_payload,
            minimal_response={"status": "running"},
        )
        try:
            search_hits = await bridge.search_notes(user_id, latest_user_text, limit=NOTE_CONTEXT_SEARCH_LIMIT)
            hit_payloads = [_note_hit_payload(hit) for hit in search_hits]
            yield _tool_event(
                "search_notes",
                "done",
                call_id=call_id,
                query=latest_user_text,
                limit=NOTE_CONTEXT_SEARCH_LIMIT,
                count=len(search_hits),
                notes=hit_payloads,
                message=f"Core note search returned {len(search_hits)} hit{'s' if len(search_hits) != 1 else ''}.",
                request=request_payload,
                minimal_response={"count": len(search_hits), "hits": hit_payloads},
            )
        except Exception as e:
            logger.warning("Skipping Core note search for chat grounding: %s", e)
            yield _tool_event(
                "search_notes",
                "error",
                call_id=call_id,
                query=latest_user_text,
                limit=NOTE_CONTEXT_SEARCH_LIMIT,
                message=f"Could not search Core notes: {e}",
                request=request_payload,
                minimal_response={"ok": False, "error": str(e)},
                error=str(e),
            )

    if not search_hits and _looks_like_note_request(latest_user_text):
        call_id = uuid.uuid4().hex
        limit = min(5, NOTE_CONTEXT_SEARCH_LIMIT)
        request_payload = {"query": "", "limit": limit, "fallback": "recent_notes"}
        yield _tool_event(
            "search_notes",
            "start",
            call_id=call_id,
            query="",
            limit=limit,
            message="No direct hits; listing recent Core notes for fallback context.",
            mcp_method="list_notes",
            request=request_payload,
            minimal_response={"status": "running"},
        )
        try:
            search_hits = await bridge.search_notes(user_id, "", limit=limit)
            hit_payloads = [_note_hit_payload(hit) for hit in search_hits]
            yield _tool_event(
                "search_notes",
                "done",
                call_id=call_id,
                query="",
                limit=limit,
                count=len(search_hits),
                notes=hit_payloads,
                message=f"Recent-note fallback returned {len(search_hits)} note{'s' if len(search_hits) != 1 else ''}.",
                mcp_method="list_notes",
                request=request_payload,
                minimal_response={"count": len(search_hits), "hits": hit_payloads},
            )
        except Exception as e:
            logger.warning("Skipping Core recent-note fallback for chat grounding: %s", e)
            yield _tool_event(
                "search_notes",
                "error",
                call_id=call_id,
                query="",
                limit=limit,
                message=f"Could not list recent Core notes: {e}",
                error=str(e),
                mcp_method="list_notes",
                request=request_payload,
                minimal_response={"ok": False, "error": str(e)},
            )

    for hit in search_hits[:NOTE_CONTEXT_FETCH_LIMIT]:
        if hit.note_id in fetched_notes:
            continue
        call_id = uuid.uuid4().hex
        request_payload = {
            "note_id": hit.note_id,
            "title": hit.title,
            "reason": "search_hit",
            "matched_by": hit.matched_by,
            "max_blocks": 30,
            "max_chars_per_block": 1_600,
        }
        yield _tool_event(
            "get_note_content",
            "start",
            call_id=call_id,
            note_id=hit.note_id,
            title=hit.title,
            reason="search_hit",
            matched_by=hit.matched_by,
            message=f'Reading Core note "{hit.title}".',
            request=request_payload,
            minimal_response={"status": "running"},
        )
        try:
            fetched = await bridge.get_note_content(
                user_id,
                hit.note_id,
                max_blocks=30,
                max_chars_per_block=1_600,
            )
            fetched_notes[hit.note_id] = fetched
            yield _tool_event(
                "get_note_content",
                "done",
                call_id=call_id,
                note_id=fetched.note_id,
                title=fetched.title,
                reason="search_hit",
                matched_by=hit.matched_by,
                block_count=len(fetched.blocks),
                message=f'Read Core note "{fetched.title}" ({len(fetched.blocks)} blocks).',
                request=request_payload,
                minimal_response=_note_content_minimal_response(fetched),
            )
        except Exception as e:
            logger.warning("Skipping Core note %s content for chat grounding: %s", hit.note_id, e)
            yield _tool_event(
                "get_note_content",
                "error",
                call_id=call_id,
                note_id=hit.note_id,
                title=hit.title,
                reason="search_hit",
                matched_by=hit.matched_by,
                message=f'Could not read Core note "{hit.title}": {e}',
                request=request_payload,
                minimal_response={"ok": False, "error": str(e)},
                error=str(e),
            )

    if not fetched_notes and not search_hits:
        return

    sections = [
        "NOTALKING CORE NOTE CONTEXT",
        "Use the authenticated user's Core notes below when they are relevant. Cite note titles naturally when answering from notes. If the notes do not contain the answer, say what is missing instead of inventing it. This chat path can preview note creation or existing-note edits when explicitly asked, and can reference note/block ids.",
    ]

    if search_hits:
        sections.append("Search hits:")
        for hit in search_hits:
            block_part = f", block_id={hit.block_id}" if hit.block_id else ""
            excerpt = f" -- {hit.excerpt}" if hit.excerpt else ""
            sections.append(
                f"- {hit.title} (note_id={hit.note_id}, matched_by={hit.matched_by}, score={hit.score:.3f}{block_part}){excerpt}"
            )

    if fetched_notes:
        sections.append("Fetched note content:")
        for note_id, note in fetched_notes.items():
            heading = "Current note" if current_note_id and note_id == current_note_id else "Relevant note"
            sections.append(_format_note_blocks(note, heading=heading))

    context_out["value"] = _trim("\n\n".join(sections), NOTE_CONTEXT_TOTAL_CHARS)


async def _resolve_target_from_provider(
    *,
    settings: Any,
    session: AsyncSession,
    identity: Any,
    provider_id: uuid.UUID,
) -> tuple[str, str, dict[str, str] | None]:
    result = await session.execute(
        select(ProviderRecord).where(
            ProviderRecord.id == provider_id,
            ProviderRecord.user_id == str(identity.user_id),
        )
    )
    prov = result.scalar_one_or_none()
    if prov is None:
        raise HTTPException(status_code=404, detail={"code": "not_found", "message": "provider not found"})

    cfg = json.loads(prov.config_json or "{}")
    kind = prov.kind

    if kind == "ollama":
        base_url = str(cfg.get("base_url") or settings.ollama_base_url).rstrip("/")
        model = str(cfg.get("model") or settings.ollama_model)
        url = _openai_compatible_chat_url(base_url)
        return url, model, None

    if is_github_family(kind):
        try:
            headers = inference_headers(cfg)
        except ValueError as e:
            raise HTTPException(
                status_code=422,
                detail={"code": "missing_token", "message": "GitHub Models provider needs token in config"},
            ) from e
        url = inference_url(cfg)
        model = str(cfg.get("model") or "openai/gpt-4.1")
        return url, model, headers

    raise HTTPException(status_code=422, detail={"code": "unsupported_provider", "message": kind})


@router.post("/completions/stream")
async def chat_completions_stream(
    request: Request,
    identity: IdentityDep,
    session: SessionDep,
    body: ChatStreamRequest,
) -> StreamingResponse:
    settings = request.app.state.settings
    bridge = getattr(request.app.state, "core_bridge", None)

    if body.provider_id is not None:
        url, model, extra_headers = await _resolve_target_from_provider(
            settings=settings,
            session=session,
            identity=identity,
            provider_id=body.provider_id,
        )
    else:
        url, model, extra_headers = _resolve_default_ollama(settings)

    messages: list[dict[str, Any]] = [m.model_dump() for m in body.messages]

    latest_user_text = _latest_user_text(messages)
    note_write_intent = _extract_note_write_intent(messages, body.note_id)
    super_prompt = _format_super_prompt(body.super_prompt)

    stream_id, cancel_event = registry.register()

    async def event_stream():
        yield _sse({"type": "start", "stream_id": str(stream_id)})
        try:
            preview, preview_tool_events = await _build_note_write_preview(
                bridge=bridge,
                user_id=str(identity.user_id),
                current_note_id=body.note_id,
                intent=note_write_intent,
            )
            for tool_event in preview_tool_events:
                yield _sse(tool_event)

            if preview is not None:
                yield _sse(
                    {
                        "type": "action",
                        "action": "note_write_preview",
                        "preview": preview.model_dump(),
                        "message": preview.message,
                    }
                )
                yield _sse({"type": "done"})
                return

            note_context_out: dict[str, str | None] = {}
            async for tool_event in _stream_core_note_context(
                bridge=bridge,
                user_id=str(identity.user_id),
                current_note_id=body.note_id,
                latest_user_text=latest_user_text,
                context_out=note_context_out,
            ):
                yield _sse(tool_event)

            system_messages: list[dict[str, Any]] = []
            note_context = note_context_out.get("value")
            if super_prompt:
                system_messages.append({"role": "system", "content": super_prompt})
            if note_context:
                system_messages.append({"role": "system", "content": note_context})
            provider_messages = system_messages + messages

            async for token in stream_chat_completion(
                url=url,
                model=model,
                messages=provider_messages,
                cancel_event=cancel_event,
                extra_headers=extra_headers,
            ):
                yield _sse({"type": "token", "text": token})
            yield _sse({"type": "done", "interrupted": cancel_event.is_set()})
        except httpx.HTTPStatusError as e:
            detail = (e.response.text or str(e))[:800]
            yield _sse({"type": "error", "status": e.response.status_code, "message": detail})
        except httpx.RequestError as e:
            yield _sse({"type": "error", "message": str(e)})
        except Exception as e:
            logger.exception("Unhandled chat stream failure")
            yield _sse({"type": "error", "message": str(e)})
        finally:
            registry.release(stream_id)

    resp = StreamingResponse(event_stream(), media_type="text/event-stream")
    return attach_core_cookies(resp, request)


@router.post("/note-actions/apply")
async def apply_note_action(
    request: Request,
    identity: IdentityDep,
    body: NoteActionApplyRequest,
):
    bridge = getattr(request.app.state, "core_bridge", None)
    if bridge is None:
        raise HTTPException(
            status_code=503,
            detail={"code": "core_bridge_unavailable", "message": "INTELLIGENCE_CORE_GRPC_URL is not configured"},
        )

    preview = body.preview
    try:
        if preview.kind == "create":
            created = await bridge.create_note(
                str(identity.user_id),
                preview.next_title,
                preview.next_body or "",
            )
            response = NoteActionApplyResponse(
                action="note_write_applied",
                kind="create",
                note_id=created.note_id,
                source=preview.source,
                title=created.title,
                head_block_id=created.head_block_id or None,
            )
        elif preview.kind == "rename":
            if not preview.target_note_id:
                raise HTTPException(status_code=422, detail={"code": "missing_note_id", "message": "target note is missing"})
            updated = await bridge.update_note(
                str(identity.user_id),
                preview.target_note_id,
                mode="rename",
                title=preview.next_title,
            )
            response = NoteActionApplyResponse(
                action="note_write_applied",
                kind="rename",
                note_id=updated.note_id,
                source=preview.source,
                title=updated.title,
                head_block_id=updated.head_block_id or None,
            )
        else:
            if not preview.target_note_id:
                raise HTTPException(status_code=422, detail={"code": "missing_note_id", "message": "target note is missing"})
            updated = await bridge.update_note(
                str(identity.user_id),
                preview.target_note_id,
                mode=preview.kind,
                body=preview.next_body or "",
            )
            response = NoteActionApplyResponse(
                action="note_write_applied",
                kind=preview.kind,
                note_id=updated.note_id,
                source=preview.source,
                title=updated.title,
                head_block_id=updated.head_block_id or None,
            )
    except HTTPException:
        raise
    except Exception as exc:
        raise HTTPException(
            status_code=502,
            detail={"code": "note_apply_failed", "message": str(exc)},
        ) from exc

    return json_response(response.model_dump(), request)


@router.post("/cancel/{stream_id}")
async def cancel_stream(request: Request, identity: IdentityDep, stream_id: uuid.UUID):
    _ = identity
    ok = registry.cancel(stream_id)
    if not ok:
        raise HTTPException(status_code=404, detail={"code": "unknown_stream", "message": "stream not found"})
    return json_response({"ok": True}, request)
