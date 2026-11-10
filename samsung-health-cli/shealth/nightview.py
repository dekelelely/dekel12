"""Terminal rendering of one night: hypnogram, snoring, vitals, all fields."""

from __future__ import annotations

from datetime import datetime, timedelta

from .sleepdata import STAGE_ORDER

SPARK = "▁▂▃▄▅▆▇█"
STAGE_NAMES = {"awake": "Awake", "rem": "REM", "light": "Light", "deep": "Deep"}


def _dt(s: str) -> datetime:
    return datetime.fromisoformat(s)


def _hm(m) -> str:
    if m is None:
        return "–"
    h, mm = divmod(round(m), 60)
    return f"{h}h {mm:02d}m" if h else f"{mm}m"


def _buckets(start: datetime, end: datetime, width: int):
    step = (end - start) / width
    return [(start + step * i, start + step * (i + 1)) for i in range(width)], step


def _overlap(a0, a1, b0, b1) -> float:
    return max(0.0, (min(a1, b1) - max(a0, b0)).total_seconds())


def hypnogram(night: dict, width: int = 64) -> list[str]:
    start, end = _dt(night["start"]), _dt(night["end"])
    buckets, step = _buckets(start, end, width)
    segs = [(_dt(s), _dt(e), st) for s, e, st in night["stages"]["segments"]]
    dominant = []
    for b0, b1 in buckets:
        best, best_ov = None, 0.0
        for s, e, st in segs:
            ov = _overlap(b0, b1, s, e)
            if ov > best_ov:
                best, best_ov = st, ov
        dominant.append(best)
    lines = []
    for st in STAGE_ORDER:
        lines.append(f"{STAGE_NAMES[st]:>6} │" + "".join("█" if d == st else " " for d in dominant))
    if night.get("snoring") and night["snoring"].get("segments"):
        sn = [(_dt(s), _dt(e)) for s, e in night["snoring"]["segments"]]
        lines.append(f"{'Snore':>6} │" + "".join(
            "▲" if any(_overlap(b0, b1, s, e) > 0 or b0 <= s < b1 for s, e in sn) else " "
            for b0, b1 in buckets))
    lines.append(" " * 7 + "└" + "─" * width)
    lines.append(" " * 8 + _axis(start, end, width))
    lines.append(f"{'':8}(each column ≈ {step.total_seconds() / 60:.0f} min)")
    return lines


def _axis(start, end, width) -> str:
    axis = [" "] * width
    t = start.replace(minute=0, second=0) + timedelta(hours=1)
    total = (end - start).total_seconds()
    while t < end:
        pos = int((t - start).total_seconds() / total * width)
        lab = t.strftime("%H")
        if pos + len(lab) <= width and all(c == " " for c in axis[max(0, pos - 1):pos + len(lab)]):
            axis[pos:pos + len(lab)] = lab
        t += timedelta(hours=1)
    return "".join(axis)


def sparkline(points, start: datetime, end: datetime, width: int = 64) -> str:
    buckets, _ = _buckets(start, end, width)
    pts = [(_dt(t), v) for t, v in points]
    vals = []
    for b0, b1 in buckets:
        inb = [v for t, v in pts if b0 <= t < b1]
        vals.append(sum(inb) / len(inb) if inb else None)
    real = [v for v in vals if v is not None]
    if not real:
        return ""
    lo, hi = min(real), max(real)
    rng = (hi - lo) or 1
    return "".join(" " if v is None else SPARK[min(7, int((v - lo) / rng * 8))] for v in vals)


def render(night: dict, out) -> None:
    p = lambda *a: print(*a, file=out)  # noqa: E731
    start, end = _dt(night["start"]), _dt(night["end"])
    st = night["stages"]
    p(f"Night ending {night['date']}   {start:%a %H:%M} → {end:%a %H:%M}   ({_hm(night['hours'] * 60)} in bed)")
    head = []
    if night.get("score") is not None:
        head.append(f"Score {night['score']}")
    if st.get("asleep_min") is not None:
        head.append(f"Asleep {_hm(st['asleep_min'])}")
    if night.get("efficiency") is not None:
        head.append(f"Efficiency {night['efficiency']}%")
    if st.get("fell_asleep_min") is not None:
        head.append(f"Fell asleep in {st['fell_asleep_min']} min")
    if st.get("awakenings") is not None:
        head.append(f"{st['awakenings']} awakenings")
    sn = night.get("snoring")
    if sn:
        head.append(f"Snoring {sn['minutes']} min ({sn['episodes']} episodes)" if sn["episodes"]
                    else "No snoring detected")
    p("  ·  ".join(head))
    p()

    if st.get("segments"):
        p("Hypnogram")
        for line in hypnogram(night):
            p(line)
        p()
        p(f"{'Stage':<7}{'time':>8}{'%':>5}")
        for k in STAGE_ORDER:
            p(f"{STAGE_NAMES[k]:<7}{_hm(st['minutes'][k]):>8}{st['percent'][k] or 0:>4}%")
        p(f"REM periods: {st.get('rem_periods', '–')}   "
          f"Awake after first falling asleep: {_hm(st.get('awake_after_onset_min'))}")
        p()
    else:
        p("(no sleep-stage data for this night — usually means it was tracked by the phone only)\n")

    if night["series"]:
        p("During sleep")
        p(f"  {'':<26}{'min':>7}{'avg':>7}{'max':>7}")
        for key, s in night["series"].items():
            name = f"{s['label']} ({s['unit']})" if s["unit"] else s["label"]
            p(f"  {name:<26}{s['min']:>7}{s['avg']:>7}{s['max']:>7}")
            p(f"  {'':<26}{sparkline(s['points'], start, end, 40)}")
            if key == "spo2" and s.get("readings_below_90"):
                p(f"  {'':<26}{s['readings_below_90']} of {s['samples']} readings below 90%")
        p()

    if night["measurements"]:
        p("Every field Samsung recorded for this session")
        w = max(len(m["label"]) for m in night["measurements"])
        for m in night["measurements"]:
            extra = f"   (raw {m['value']})" if m["pretty"] != str(m["value"]) and not isinstance(m["value"], float) else ""
            p(f"  {m['label']:<{w}}  {m['pretty']}{extra}")
        p()

    others = [r for r in night["related"] if not (sn and r["table"] == sn.get("table") and not r["fields"])]
    if others:
        p("Other sleep tables with data for this night")
        for r in others:
            bits = [f"{r['rows']} rows"]
            if r.get("total_min") is not None:
                bits.append(f"{r['total_min']} min total")
            p(f"  {r['table']}: " + ", ".join(bits))
            for k, v in r["fields"].items():
                p(f"      {k}: {v}")
        p()

    if night["other_sessions"]:
        p("Other sleep that day: " + ", ".join(
            f"{_dt(o['start']):%H:%M}–{_dt(o['end']):%H:%M} ({o['hours']} h)" for o in night["other_sessions"]))

    missing = [k for k in ("heart_rate", "spo2", "respiratory_rate", "skin_temperature")
               if k not in night["available"]["overlays"]]
    if missing:
        p(f"Not in your export: {', '.join(missing)} "
          f"(needs a watch that measures it, with the setting turned on)")
    if not sn and not any("snor" in t for t in night["available"]["related_tables"]):
        p("Not in your export: snoring (enable Snore detection in Samsung Health → Sleep)")
