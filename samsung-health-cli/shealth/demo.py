"""Generate a fake Samsung Health export (zip) that mimics the real format,
including its quirks: metadata first line, prefixed column names, trailing
commas, UTC timestamps + separate time_offset. Used for `shealth demo` and tests."""

from __future__ import annotations

import io
import random
import uuid
import zipfile
from datetime import datetime, timedelta, timezone

OFFSET = "UTC+0200"
OFF = timedelta(hours=2)


def _csv(source: str, header: list[str], rows: list[list]) -> str:
    buf = io.StringIO()
    buf.write(f"{source},6315001,{len(header)}\n")
    buf.write(",".join(header) + ",\n")
    for r in rows:
        buf.write(",".join("" if v is None else str(v) for v in r) + ",\n")
    return buf.getvalue()


def _ts(local: datetime) -> str:
    """Local wall-clock -> UTC string as Samsung stores it."""
    return (local - OFF).strftime("%Y-%m-%d %H:%M:%S.") + f"{(local - OFF).microsecond // 1000:03d}"


def _day_ms(d) -> int:
    return int(datetime(d.year, d.month, d.day, tzinfo=timezone.utc).timestamp() * 1000)


def build(path, days: int = 60, end: datetime | None = None, seed: int = 7) -> str:
    rnd = random.Random(seed)
    end = (end or datetime.now()).replace(hour=0, minute=0, second=0, microsecond=0)
    stamp = end.strftime("%Y%m%d") + "120000"
    root = f"samsunghealth_demo_{stamp}/"
    files: dict[str, str] = {}

    def add(source, header, rows):
        files[f"{root}{source}.{stamp}.csv"] = _csv(source, header, rows)

    day_list = [(end - timedelta(days=i)).date() for i in range(days, -1, -1)]

    # --- steps ------------------------------------------------------------ #
    trend, summary = [], []
    for d in day_list:
        weekend = d.weekday() >= 5
        total = int(rnd.gauss(6500 if weekend else 9000, 2500))
        total = max(800, total)
        phone = int(total * rnd.uniform(0.4, 0.7))
        dist, kcal = round(total * 0.75, 1), round(total * 0.04, 2)
        trend.append([_day_ms(d), total, dist, kcal, -2, uuid.uuid4()])
        trend.append([_day_ms(d), phone, round(phone * 0.75, 1), round(phone * 0.04, 2), 0, uuid.uuid4()])
        summary.append([total, dist, kcal, _day_ms(d), "watch-uuid", uuid.uuid4()])
        summary.append([phone, round(phone * 0.75, 1), round(phone * 0.04, 2), _day_ms(d), "phone-uuid", uuid.uuid4()])
    add("com.samsung.shealth.step_daily_trend",
        ["day_time", "count", "distance", "calorie", "source_type", "datauuid"], trend)
    add("com.samsung.shealth.tracker.pedometer_day_summary",
        ["step_count", "distance", "calorie", "day_time", "deviceuuid", "datauuid"], summary)

    # --- sleep + stages --------------------------------------------------- #
    sl, stages = [], []
    P = "com.samsung.health.sleep."
    for d in day_list[1:]:
        bed = datetime(d.year, d.month, d.day) - timedelta(hours=rnd.uniform(0.5, 2.5))
        wake = bed + timedelta(hours=max(4.5, rnd.gauss(7.1, 0.8)))
        sid = str(uuid.uuid4())
        sl.append([_ts(bed), _ts(wake), OFFSET, rnd.randint(60, 92), round(rnd.uniform(82, 96), 1), sid])
        t = bed
        while t < wake:
            seg = min(wake, t + timedelta(minutes=rnd.randint(10, 45)))
            stage = rnd.choices([40001, 40002, 40003, 40004], [0.06, 0.55, 0.17, 0.22])[0]
            stages.append([_ts(t), sid, stage, _ts(seg), OFFSET, uuid.uuid4()])
            t = seg
    add("com.samsung.shealth.sleep",
        [P + "start_time", P + "end_time", P + "time_offset", "sleep_score", "efficiency", P + "datauuid"], sl)
    add("com.samsung.shealth.sleep_stage",
        ["start_time", "sleep_id", "stage", "end_time", "time_offset", "datauuid"], stages)

    # --- heart rate (every 10 min) + stress (hourly) ---------------------- #
    hr, st = [], []
    P = "com.samsung.health.heart_rate."
    for d in day_list:
        base = datetime(d.year, d.month, d.day)
        for m in range(0, 24 * 60, 10):
            t = base + timedelta(minutes=m)
            asleep = t.hour < 7
            v = round(rnd.gauss(55 if asleep else 72, 6))
            hr.append([_ts(t), _ts(t + timedelta(minutes=1)), OFFSET, v, v - 3, v + 4, uuid.uuid4()])
        for h in range(8, 23):
            t = base + timedelta(hours=h)
            st.append([_ts(t), _ts(t), OFFSET, rnd.randint(10, 75), uuid.uuid4()])
    add("com.samsung.shealth.tracker.heart_rate",
        [P + "start_time", P + "end_time", P + "time_offset", P + "heart_rate", P + "min", P + "max",
         P + "datauuid"], hr)
    add("com.samsung.shealth.stress", ["start_time", "end_time", "time_offset", "score", "datauuid"], st)

    # --- weight (every ~3 days) ------------------------------------------- #
    w, kg = [], 78.0
    for d in day_list[::3]:
        kg += rnd.uniform(-0.4, 0.3)
        t = datetime(d.year, d.month, d.day, 7, rnd.randint(0, 40))
        w.append([_ts(t), OFFSET, round(kg, 1), round(rnd.uniform(17, 20), 1), round(kg * 0.43, 1), 175, uuid.uuid4()])
    add("com.samsung.health.weight",
        ["start_time", "time_offset", "weight", "body_fat", "skeletal_muscle_mass", "height", "datauuid"], w)

    # --- workouts --------------------------------------------------------- #
    ex = []
    P = "com.samsung.health.exercise."
    for d in day_list:
        if rnd.random() < 0.45:
            kind = rnd.choice([1001, 1002, 11007, 9002, 15005])
            mins = rnd.randint(20, 70)
            s = datetime(d.year, d.month, d.day, rnd.choice([7, 12, 18]), rnd.randint(0, 59))
            dist = {1001: 85, 1002: 170, 11007: 380, 15005: 140}.get(kind, 0) * mins or None
            ex.append([_ts(s), _ts(s + timedelta(minutes=mins)), OFFSET, kind, mins * 60000, dist,
                       round(mins * rnd.uniform(5, 11), 1), rnd.randint(105, 150), rnd.randint(150, 182),
                       uuid.uuid4()])
    add("com.samsung.shealth.exercise",
        [P + c for c in ("start_time", "end_time", "time_offset", "exercise_type", "duration", "distance",
                         "calorie", "mean_heart_rate", "max_heart_rate", "datauuid")], ex)

    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as zf:
        for name, content in files.items():
            zf.writestr(name, content)
    return str(path)
