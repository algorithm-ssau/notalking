"""FastAPI application factory."""

from __future__ import annotations

import logging
from contextlib import asynccontextmanager

from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from prometheus_client import Counter

from notalking_intelligence.config import Settings
from notalking_intelligence.core_bridge import client_from_settings
from notalking_intelligence.db.session import lifespan_engine
from notalking_intelligence.logging import configure_logging
from notalking_intelligence.metrics import registry
from notalking_intelligence.routes.chat import router as chat_router
from notalking_intelligence.routes.health import router as health_router
from notalking_intelligence.routes.providers import router as providers_router
from notalking_intelligence.routes.v2t import router as v2t_router

logger = logging.getLogger(__name__)

REQUEST_COUNTER = Counter(
    "intelligence_http_requests_total",
    "HTTP requests",
    ["method"],
    registry=registry(),
)


def create_app(settings: Settings) -> FastAPI:
    configure_logging(settings)

    @asynccontextmanager
    async def lifespan(app: FastAPI):
        async with lifespan_engine(settings) as (_engine, session_factory):
            app.state.settings = settings
            app.state.session_factory = session_factory
            bridge = client_from_settings(settings)
            app.state.core_bridge = bridge
            try:
                yield
            finally:
                if bridge is not None:
                    await bridge.close()

    app = FastAPI(title="Notalking Intelligence", lifespan=lifespan)

    origins = [o.strip() for o in settings.cors_origins.split(",") if o.strip()]
    if origins:
        app.add_middleware(
            CORSMiddleware,
            allow_origins=origins,
            allow_credentials=True,
            allow_methods=["GET", "POST", "PATCH", "DELETE", "OPTIONS"],
            allow_headers=["*"],
        )

    @app.middleware("http")
    async def count_requests(request: Request, call_next):
        response = await call_next(request)
        REQUEST_COUNTER.labels(request.method).inc()
        return response

    app.include_router(health_router)
    app.include_router(providers_router)
    app.include_router(chat_router)
    app.include_router(v2t_router)

    return app
