"""CLI entry: `python -m notalking_intelligence` or `notalking-intelligence`."""

from __future__ import annotations

import sys

import uvicorn

from notalking_intelligence.app import create_app
from notalking_intelligence.config import load_settings, parse_bind_host_port


def main() -> None:
    settings = load_settings(sys.argv[1:])
    app = create_app(settings)
    host, port = parse_bind_host_port(settings.http_bind)
    uvicorn.run(app, host=host, port=port, log_level=settings.log_level)


if __name__ == "__main__":
    main()
