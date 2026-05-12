"""In-flight stream cancellation (SPEC §5.3)."""

from __future__ import annotations

import asyncio
import uuid
from dataclasses import dataclass, field


@dataclass
class StreamRegistry:
    events: dict[uuid.UUID, asyncio.Event] = field(default_factory=dict)

    def register(self) -> tuple[uuid.UUID, asyncio.Event]:
        sid = uuid.uuid4()
        ev = asyncio.Event()
        self.events[sid] = ev
        return sid, ev

    def cancel(self, sid: uuid.UUID) -> bool:
        ev = self.events.pop(sid, None)
        if ev is None:
            return False
        ev.set()
        return True

    def release(self, sid: uuid.UUID) -> None:
        self.events.pop(sid, None)


registry = StreamRegistry()
