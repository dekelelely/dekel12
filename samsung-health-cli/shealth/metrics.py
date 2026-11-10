"""Turn raw Samsung Health tables into per-day / per-session records.

Every function returns a list of plain dicts so the CLI can render them as a
table, JSON or CSV.
"""

from __future__ import annotations

from collections import defaultdict
from datetime import date, datetime, timedelta
from statistics import mean

from .db import NoDataError, columns, find_table, pick
from .timeutil import day_from_daytime, to_local

# Samsung Health exercise_type codes (most common ones; unknown codes are shown as "type <n>").
EXERCISE_TYPES = {
    0: "Custom", 1001: "Walking", 1002: "Running",
    3001: "Golf", 4003: "Basketball", 4006: "Soccer", 5001: "Volleyball",
    6002: "Tennis", 6003: "Badminton", 6004: "Table tennis",
    7002: "Boxing", 7003: "Martial arts", 8002: "Dancing",
    9001: "Pilates", 9002: "Yoga",
    10001: "Stretching", 10002: "Jump rope", 10007: "Circuit training",
    11007: "Cycling", 12001: "Aerobics",
    13001: "Hiking", 13002: "Rock climbing", 13004: "Mountain biking",
    14001: "Swimming", 14010: "Rowing",
    15002: "Weight machine", 15003: "Exercise bike", 15004: "Rowing machine",
    15005: "Treadmill", 15006: "Elliptical",
    16002: "Skiing", 16007: "Snowboarding",
}


def _need(conn, what: str, *sources: str) -> str:
    t = find_table(conn, *sources)
    if not t:
        raise NoDataError(f"No {what} data in this export (looked for: {', '.join(sources)}).")
    return t


def _in(d: date | None, start: date, end: date) -> bool:
    return d is not None and start <= d <= end


def _num(v):
    return v if isinstance(v, (int, float)) else None


# --------------------------------------------------------------------------- #
# steps
# --------------------------------------------------------------------------- #

def steps(conn, start: date, end: date) -> list[dict]:
    """Daily steps. Prefers step_daily_trend (source_type -2 = Samsung's merged
    phone+watch total); falls back to pedometer_day_summary taking the max
    device per day so phone and watch aren't double-counted."""
    by_day: dict[date, dict] = {}
    t = find_table(conn, "com.samsung.shealth.step_daily_trend")
    if t:
        cols = columns(conn, t)
        cnt = pick(cols, "count", "step_count")
        st = pick(cols, "source_type")
        q = f'SELECT * FROM "{t}"' + (f' WHERE "{st}" = -2' if st else "")
        for r in conn.execute(q):
            _keep_max(by_day, r, cnt, start, end)
    if not by_day:
        t = _need(conn, "step", "com.samsung.shealth.tracker.pedometer_day_summary",
                  "com.samsung.shealth.step_daily_trend")
        cols = columns(conn, t)
        cnt = pick(cols, "step_count", "count")
        for r in conn.execute(f'SELECT * FROM "{t}"'):
            _keep_max(by_day, r, cnt, start, end)
    return [by_day[d] for d in sorted(by_day)]


def _keep_max(by_day, r, cnt_col, start, end):
    keys = r.keys()
    d = day_from_daytime(r["day_time"]) if "day_time" in keys else None
    if not _in(d, start, end):
        return
    n = _num(r[cnt_col]) or 0
    if d in by_day and by_day[d]["steps"] >= n:
        return
    dist = _num(r["distance"]) if "distance" in keys else None
    kcal = _num(r["calorie"]) if "calorie" in keys else None
    by_day[d] = {
        "date": d.isoformat(),
        "steps": int(n),
        "distance_km": round(dist / 1000, 2) if dist is not None else None,
        "active_kcal": round(kcal) if kcal is not None else None,
    }


# --------------------------------------------------------------------------- #
# sleep
# --------------------------------------------------------------------------- #

def sleep(conn, start: date, end: date, all_fields: bool = False) -> list[dict]:
    """One record per sleep session (wake-up date). See sleepdata.py."""
    from .sleepdata import SleepData
    return SleepData(conn).nights(start, end, all_fields)


def sleep_nightly(conn, start, end) -> dict[str, float]:
    """Total hours slept per wake-up date (naps included)."""
    tot: dict[str, float] = defaultdict(float)
    for r in sleep(conn, start, end):
        tot[r["date"]] += r["hours"]
    return tot


# --------------------------------------------------------------------------- #
# heart rate / stress / spo2 — "point sample" tables
# --------------------------------------------------------------------------- #

def _daily_samples(conn, start, end, what, sources, value_cols):
    t = _need(conn, what, *sources)
    cols = columns(conn, t)
    v = pick(cols, *value_cols)
    if not v:
        raise NoDataError(f"{what} table {t} has no value column ({', '.join(value_cols)}).")
    off = "time_offset" if "time_offset" in cols else None
    day: dict[date, list[float]] = defaultdict(list)
    q = f'SELECT start_time, "{v}" AS v' + (f', "{off}" AS off' if off else "") + f' FROM "{t}"'
    for r in conn.execute(q):
        val = _num(r["v"])
        if not val or val <= 0:
            continue
        dt = to_local(r["start_time"], r["off"] if off else None)
        if dt and start <= dt.date() <= end:
            day[dt.date()].append(val)
    return day


def heart_rate(conn, start, end) -> list[dict]:
    day = _daily_samples(conn, start, end, "heart rate",
                         ("com.samsung.shealth.tracker.heart_rate", "com.samsung.health.heart_rate"),
                         ("heart_rate",))
    out = []
    for d in sorted(day):
        vals = sorted(day[d])
        # "low" = 5th percentile: a robust stand-in for resting HR that ignores
        # single bad readings. It is NOT Samsung's own resting-HR figure.
        low = vals[max(0, int(len(vals) * 0.05) - 1)] if len(vals) >= 20 else vals[0]
        out.append({"date": d.isoformat(), "samples": len(vals), "low": round(low),
                    "avg": round(mean(vals)), "max": round(vals[-1])})
    return out


def stress(conn, start, end) -> list[dict]:
    day = _daily_samples(conn, start, end, "stress", ("com.samsung.shealth.stress",), ("score",))
    return [{"date": d.isoformat(), "samples": len(v), "avg": round(mean(v)), "max": round(max(v))}
            for d, v in sorted(day.items())]


def spo2(conn, start, end) -> list[dict]:
    day = _daily_samples(conn, start, end, "blood oxygen",
                         ("com.samsung.shealth.tracker.oxygen_saturation",
                          "com.samsung.health.oxygen_saturation"), ("spo2",))
    return [{"date": d.isoformat(), "samples": len(v), "avg": round(mean(v), 1), "min": round(min(v))}
            for d, v in sorted(day.items())]


# --------------------------------------------------------------------------- #
# weight / workouts
# --------------------------------------------------------------------------- #

def weight(conn, start, end) -> list[dict]:
    t = _need(conn, "weight", "com.samsung.health.weight", "com.samsung.shealth.weight")
    cols = columns(conn, t)
    extra = [c for c in ("body_fat", "skeletal_muscle_mass", "basal_metabolic_rate") if c in cols]
    out = []
    for r in conn.execute(f'SELECT * FROM "{t}"'):
        dt = to_local(r["start_time"], r["time_offset"] if "time_offset" in cols else None)
        w = _num(r["weight"])
        if not dt or not w or not _in(dt.date(), start, end):
            continue
        rec = {"date": dt.date().isoformat(), "time": dt.strftime("%H:%M"), "weight_kg": round(w, 1)}
        for c in extra:
            val = _num(r[c])
            rec[c] = round(val, 1) if val is not None else None
        out.append(rec)
    out.sort(key=lambda x: (x["date"], x["time"]))
    return out


def latest_weight(conn) -> dict | None:
    try:
        rows = weight(conn, date(1970, 1, 1), date(2999, 1, 1))
    except NoDataError:
        return None
    return rows[-1] if rows else None


def workouts(conn, start, end) -> list[dict]:
    t = _need(conn, "workout", "com.samsung.shealth.exercise", "com.samsung.health.exercise")
    cols = columns(conn, t)
    g = lambda r, *c: _num(r[pick(cols, *c)]) if pick(cols, *c) else None  # noqa: E731
    out = []
    for r in conn.execute(f'SELECT * FROM "{t}"'):
        off = r["time_offset"] if "time_offset" in cols else None
        s = to_local(r["start_time"], off)
        if not s or not _in(s.date(), start, end):
            continue
        e = to_local(r["end_time"], off) if "end_time" in cols else None
        dur_ms = g(r, "duration")
        mins = dur_ms / 60000 if dur_ms else ((e - s).total_seconds() / 60 if e else None)
        code = g(r, "exercise_type")
        dist = g(r, "distance")
        kcal = g(r, "calorie")
        hr = g(r, "mean_heart_rate")
        out.append({
            "date": s.date().isoformat(),
            "start": s.strftime("%H:%M"),
            "type": EXERCISE_TYPES.get(int(code), f"type {int(code)}") if code is not None else "?",
            "minutes": round(mins) if mins else None,
            "distance_km": round(dist / 1000, 2) if dist else None,
            "kcal": round(kcal) if kcal else None,
            "avg_hr": round(hr) if hr else None,
            "max_hr": round(g(r, "max_heart_rate")) if g(r, "max_heart_rate") else None,
        })
    out.sort(key=lambda x: (x["date"], x["start"]))
    return out


# --------------------------------------------------------------------------- #
# summary
# --------------------------------------------------------------------------- #

def _safe(fn, *a):
    try:
        return fn(*a)
    except NoDataError:
        return []


def summary(conn, start, end) -> list[dict]:
    st = {r["date"]: r["steps"] for r in _safe(steps, conn, start, end)}
    sl = _safe(sleep_nightly, conn, start, end) or {}
    hr_rows = _safe(heart_rate, conn, start, end)
    hr = {r["date"]: r["avg"] for r in hr_rows}
    hr_low = {r["date"]: r["low"] for r in hr_rows}
    sx = {r["date"]: r["avg"] for r in _safe(stress, conn, start, end)}
    wo: dict[str, list] = defaultdict(list)
    for w in _safe(workouts, conn, start, end):
        wo[w["date"]].append(w)
    rows = []
    d = start
    while d <= end:
        k = d.isoformat()
        rows.append({
            "date": k,
            "steps": st.get(k),
            "sleep_h": round(sl[k], 1) if k in sl else None,
            "hr_avg": hr.get(k),
            "hr_low": hr_low.get(k),
            "stress": sx.get(k),
            "workouts": ", ".join(f'{w["type"]} {w["minutes"] or "?"}m' for w in wo[k]) or None,
        })
        d += timedelta(days=1)
    return rows


def averages(rows: list[dict], keys) -> dict:
    out = {}
    for k in keys:
        vals = [r[k] for r in rows if isinstance(r.get(k), (int, float))]
        out[k] = round(mean(vals), 1) if vals else None
    return out


def latest_date(conn) -> date | None:
    """The day the export was made — the default end of the date window, so
    `--days 14` means "the 14 days before your export", not before today."""
    m = conn.execute("SELECT value FROM _shealth_meta WHERE key='exported_at'").fetchone()
    if m and m["value"]:
        try:
            return datetime.fromisoformat(m["value"]).date()
        except ValueError:
            pass
    return None
