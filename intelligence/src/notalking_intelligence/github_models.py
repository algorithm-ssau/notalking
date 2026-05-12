"""GitHub Models inference API (models.github.ai) request helpers."""

from __future__ import annotations

from typing import Any


DEFAULT_GITHUB_MODELS_BASE = "https://models.github.ai"
DEFAULT_CHAT_PATH = "/inference/chat/completions"
DEFAULT_API_VERSION = "2022-11-28"


def inference_headers(cfg: dict[str, Any]) -> dict[str, str]:
    token = cfg.get("token") or cfg.get("github_token")
    if not token or not str(token).strip():
        raise ValueError("missing_github_token")
    api_version = str(cfg.get("api_version") or DEFAULT_API_VERSION)
    return {
        "Authorization": f"Bearer {token.strip()}",
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": api_version,
    }


def inference_url(cfg: dict[str, Any]) -> str:
    base = str(cfg.get("base_url") or DEFAULT_GITHUB_MODELS_BASE).rstrip("/")
    path = str(cfg.get("chat_path") or DEFAULT_CHAT_PATH)
    if not path.startswith("/"):
        path = "/" + path
    return base + path
