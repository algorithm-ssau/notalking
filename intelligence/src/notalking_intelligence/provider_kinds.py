"""Supported LLM provider kinds and catalog metadata for clients."""

from __future__ import annotations

from typing import Any

ALLOWED_PROVIDER_KINDS = frozenset({"ollama", "github_models", "github_copilot"})


def is_github_family(kind: str) -> bool:
    return kind in ("github_models", "github_copilot")


def catalog_entries() -> list[dict[str, Any]]:
    """Static catalog for Settings / Agent UI (no secrets)."""
    return [
        {
            "kind": "ollama",
            "label": "Ollama",
            "description": "Local OpenAI-compatible endpoint (root or /v1 base accepted).",
            "config_fields": {
                "base_url": {"required": False, "example": "http://127.0.0.1:11434"},
                "model": {"required": False, "example": "llama3.2"},
            },
        },
        {
            "kind": "github_models",
            "label": "GitHub Models",
            "description": "Hosted inference at models.github.ai (fine-grained PAT with models: read).",
            "config_fields": {
                "token": {"required": True, "secret": True},
                "base_url": {"required": False, "example": "https://models.github.ai"},
                "model": {"required": False, "example": "openai/gpt-4.1"},
                "api_version": {"required": False, "example": "2022-11-28"},
            },
        },
    ]
