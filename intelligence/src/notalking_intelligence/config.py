"""Runtime configuration: CLI overrides environment, environment overrides defaults (SPEC §8)."""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any

from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict


def _default_sqlite_url() -> str:
    root = Path(__file__).resolve().parents[2]
    data_dir = root / "data"
    data_dir.mkdir(parents=True, exist_ok=True)
    return f"sqlite+aiosqlite:///{data_dir / 'intelligence.db'}"


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_prefix="INTELLIGENCE_", env_file=None, extra="ignore")

    environment: str = "dev"

    http_bind: str = "0.0.0.0:41000"

    database_url: str = Field(default_factory=_default_sqlite_url)

    core_http_url: str = "http://127.0.0.1:40000"
    core_grpc_url: str | None = None

    nats_url: str | None = None

    cors_origins: str = ""

    log_level: str = "info"

    ollama_base_url: str = "http://127.0.0.1:11434"
    # Default when no provider row is selected; override with INTELLIGENCE_OLLAMA_MODEL.
    ollama_model: str = "deepseek-r1:8b"


def _parse_bind(bind: str) -> tuple[str, int]:
    host, _, port_s = bind.rpartition(":")
    if not host:
        host = "0.0.0.0"
    return host, int(port_s)


def load_settings(argv: list[str] | None = None) -> Settings:
    """Merge CLI (when provided) over environment over pydantic defaults."""
    if argv is None:
        argv = os.environ.get("INTELLIGENCE_ARGV", "")
        argv_list = [x for x in argv.split("\x1e") if x] if argv else []
    else:
        argv_list = argv

    overrides: dict[str, Any] = {}
    i = 0
    while i < len(argv_list):
        arg = argv_list[i]
        if arg == "--http-bind" and i + 1 < len(argv_list):
            overrides["INTELLIGENCE_HTTP_BIND"] = argv_list[i + 1]
            i += 2
            continue
        if arg == "--database-url" and i + 1 < len(argv_list):
            overrides["INTELLIGENCE_DATABASE_URL"] = argv_list[i + 1]
            i += 2
            continue
        if arg == "--core-http-url" and i + 1 < len(argv_list):
            overrides["INTELLIGENCE_CORE_HTTP_URL"] = argv_list[i + 1]
            i += 2
            continue
        if arg == "--core-grpc-url" and i + 1 < len(argv_list):
            overrides["INTELLIGENCE_CORE_GRPC_URL"] = argv_list[i + 1]
            i += 2
            continue
        if arg == "--nats-url" and i + 1 < len(argv_list):
            overrides["INTELLIGENCE_NATS_URL"] = argv_list[i + 1]
            i += 2
            continue
        if arg == "--cors-origins" and i + 1 < len(argv_list):
            overrides["INTELLIGENCE_CORS_ORIGINS"] = argv_list[i + 1]
            i += 2
            continue
        if arg == "--log-level" and i + 1 < len(argv_list):
            overrides["INTELLIGENCE_LOG_LEVEL"] = argv_list[i + 1]
            i += 2
            continue
        if arg == "--environment" and i + 1 < len(argv_list):
            overrides["INTELLIGENCE_ENVIRONMENT"] = argv_list[i + 1]
            i += 2
            continue
        if arg == "--ollama-base-url" and i + 1 < len(argv_list):
            overrides["INTELLIGENCE_OLLAMA_BASE_URL"] = argv_list[i + 1]
            i += 2
            continue
        if arg == "--ollama-model" and i + 1 < len(argv_list):
            overrides["INTELLIGENCE_OLLAMA_MODEL"] = argv_list[i + 1]
            i += 2
            continue
        i += 1

    previous = {k: os.environ[k] for k in overrides if k in os.environ}
    try:
        for key, value in overrides.items():
            os.environ[key] = value
        return Settings()
    finally:
        for key in overrides:
            if key in previous:
                os.environ[key] = previous[key]
            else:
                os.environ.pop(key, None)


def parse_bind_host_port(bind: str) -> tuple[str, int]:
    return _parse_bind(bind)
