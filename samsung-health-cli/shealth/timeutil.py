"""Time helpers for Samsung Health exports.

Samsung Health stores timestamps in UTC (``2024-03-01 06:12:00.000``) next to a
separate ``time_offset`` column (``UTC+0300``). The local wall-clock time, which
is what you actually experienced, is UTC + offset.
"""

from __future__ import annotations

import re
from datetime import date, datetime, timedelta, timezone

_FORMATS = (
    "%Y-%m-%d %H:%M:%S.%f",
    "%Y-%m-%d %H:%M:%S",
    "%Y-%m-%dT%H:%M:%S.%f",
    "%Y-%m-%dT%H:%M:%S",
    "%Y-%m-%d %H:%M",
    "%Y-%m-%d",
)
_OFFSET_RE = re.compile(r"(?:UTC|GMT)?\s*([+-])(\d{1,2}):?(\d{2})?$", re.I)


def parse_ts(value) -> datetime | None:
    """Parse a UTC timestamp (string or epoch seconds/ms) into a naive UTC datetime."""
    if value is None or value == "":
        return None
    if isinstance(value, (int, float)):
        return _from_epoch(value)
    s = str(value).strip()
    if re.fullmatch(r"-?\d+(\.\d+)?", s):
        return _from_epoch(float(s))
    for fmt in _FORMATS:
        try:
            return datetime.strptime(s, fmt)
        except ValueError:
            continue
    return None


def _from_epoch(v: float) -> datetime:
    if abs(v) > 1e11:  # milliseconds
        v = v / 1000.0
    return datetime.fromtimestamp(v, tz=timezone.utc).replace(tzinfo=None)


def parse_offset(value) -> timedelta:
    """``UTC+0300`` / ``+03:00`` / ``UTC-0530`` -> timedelta. Unknown -> 0."""
    if value is None or value == "":
        return timedelta(0)
    if isinstance(value, (int, float)):  # some versions store offset in ms
        return timedelta(milliseconds=value)
    m = _OFFSET_RE.search(str(value).strip())
    if not m:
        return timedelta(0)
    sign = -1 if m.group(1) == "-" else 1
    return sign * timedelta(hours=int(m.group(2)), minutes=int(m.group(3) or 0))


def to_local(ts, offset) -> datetime | None:
    dt = parse_ts(ts)
    if dt is None:
        return None
    return dt + parse_offset(offset)


def day_from_daytime(value) -> date | None:
    """Daily-summary tables key each day by ``day_time``: epoch ms of 00:00 UTC
    of the *local* calendar date. So the UTC date of that instant is the day."""
    dt = parse_ts(value)
    return dt.date() if dt else None


def parse_date(s: str) -> date:
    return datetime.strptime(s, "%Y-%m-%d").date()


def date_range(start: date, end: date):
    d = start
    while d <= end:
        yield d
        d += timedelta(days=1)
