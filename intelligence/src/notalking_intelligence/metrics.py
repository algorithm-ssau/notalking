"""Prometheus metrics registry (SPEC §9)."""

from __future__ import annotations

from prometheus_client import CollectorRegistry

_registry = CollectorRegistry()


def registry() -> CollectorRegistry:
    return _registry
