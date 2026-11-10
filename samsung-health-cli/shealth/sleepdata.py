"""Everything recorded about a night's sleep.

Samsung doesn't document its export columns and they change between app
versions and devices, so this module *discovers* data instead of assuming it:

  * session fields  — every non-empty column of the sleep session row
                      (score, efficiency, latency, mental/physical recovery,
                      cycles, ... whatever your app version records);
  * stages          — the sleep_stage table, linked by sleep_id;
  * related tables  — any other table whose name contains "sleep", "snor",
                      "vascular", "apnea", ... (snoring, vascular load, sleep
                      apnea checks, sleep_combined, ...), linked by a sleep_id-like
                      column or by falling inside the session's time window;
  * overlays        — time-series measured during the night (heart rate,
                      SpO2, respiratory rate, skin temperature, HRV, stress),
                      matched by table name and clipped to the session window.
"""

from __future__ import annotations

from datetime import date, datetime, timedelta
from statistics import mean

from .db import NoDataError, columns, find_table, pick
from .timeutil import parse_ts, to_local

STAGES = {40001: "awake", 40002: "light", 40003: "deep", 40004: "rem"}
STAGE_ORDER = ("awake", "rem", "light", "deep")

SESSION_SOURCES = ("com.samsung.shealth.sleep", "com.samsung.health.sleep")
STAGE_SOURCES = ("com.samsung.shealth.sleep_stage", "com.samsung.health.sleep_stage")

# key, label, unit, substrings of the Samsung source name, value-column candidates
OVERLAYS = (
    ("heart_rate", "Heart rate", "bpm", ("tracker.heart_rate", "health.heart_rate"), ("heart_rate",)),
    ("spo2", "Blood oxygen", "%", ("oxygen_saturation",), ("spo2", "oxygen_saturation")),
    ("respiratory_rate", "Respiratory rate", "br/min", ("respiratory_rate",),
     ("respiratory_rate", "average", "rate", "value")),
    ("skin_temperature", "Skin temperature", "°C", ("skin_temperature",),
     ("temperature", "skin_temperature", "mean", "average")),
    ("hrv", "Heart rate variability", "ms", (".hrv",), ("rmssd", "hrv", "sdnn", "value")),
    ("stress", "Stress", "", ("shealth.stress",), ("score",)),
)

# Tables matching any of these are treated as sleep-related. "vascular" = Vascular
# Load and "apnea"/"breathing" = sleep-apnea checks (Galaxy Watch Ultra / Watch7+);
# all are measured while you sleep but don't have "sleep" in their name.
RELATED_KEYWORDS = ("sleep", "snor", "vascular", "apnea", "breathing_disturb", "bedtime")

LINK_COLS = ("sleep_id", "sleep_uuid", "sleep_datauuid", "parent_id")

# Bookkeeping columns that say nothing about your sleep.
HIDE = {
    "datauuid", "deviceuuid", "pkg_name", "source_package_name", "create_time", "update_time",
    "time_offset", "start_time", "end_time", "comment", "custom", "create_sh_ver",
    "modify_sh_ver", "client_data_id", "client_data_ver", "extra_data", "binning_data",
    "sleep_id", "sleep_uuid", "sleep_datauuid", "parent_id", "combined_id", "integrated_id",
}

LABELS = {
    "sleep_score": "Sleep score", "score": "Score", "efficiency": "Efficiency (%)",
    "sleep_latency": "Time to fall asleep", "mental_recovery": "Mental recovery",
    "physical_recovery": "Physical recovery", "movement_awakening": "Movement / awakenings",
    "sleep_cycle": "Sleep cycles", "total_rem_duration": "Total REM",
    "total_light_duration": "Total light sleep", "sleep_duration": "Sleep duration",
    "total_sleep_time": "Total sleep time", "wake_score": "Wake score",
    "quality": "Quality (your rating)", "has_sleep_data": "Has stage data",
    "sleep_type": "Sleep type", "stage_analyzed_type": "Stage analysis type",
}


def label(col: str) -> str:
    return LABELS.get(col, col.replace("_", " ").capitalize())


def pretty(col: str, v) -> str:
    """Human formatting. Duration-like columns are almost always milliseconds
    in Samsung exports; we only convert when the value is clearly ms-sized."""
    if isinstance(v, (int, float)) and any(k in col for k in ("duration", "latency", "_time")):
        if v >= 60_000:
            return _hm(v / 60_000)
    if isinstance(v, float):
        return f"{v:.1f}"
    return str(v)


def _hm(minutes: float) -> str:
    h, m = divmod(round(minutes), 60)
    return f"{h}h {m:02d}m" if h else f"{m}m"


def _num(v):
    return v if isinstance(v, (int, float)) and not isinstance(v, bool) else None


def _iso(dt: datetime) -> str:
    return dt.isoformat(timespec="seconds")


class SleepData:
    def __init__(self, conn):
        self.conn = conn
        self.session_table = find_table(conn, *SESSION_SOURCES)
        if not self.session_table:
            raise NoDataError("No sleep data in this export (no com.samsung.shealth.sleep table).")
        self.session_cols = columns(conn, self.session_table)
        self.stage_table = find_table(conn, *STAGE_SOURCES)
        self.stage_cols = columns(conn, self.stage_table) if self.stage_table else []
        self.score_col = pick(self.session_cols, "sleep_score", "score")
        self.eff_col = pick(self.session_cols, "efficiency")

        tables = [dict(r) for r in conn.execute("SELECT name, source FROM _shealth_tables")]
        self.overlays: dict[str, dict] = {}
        for key, lab, unit, subs, cands in OVERLAYS:
            for t in tables:
                if any(s in t["source"] for s in subs):
                    cols = columns(conn, t["name"])
                    v = pick(cols, *cands)
                    if v and "start_time" in cols:
                        self.overlays[key] = {"table": t["name"], "col": v, "cols": cols,
                                              "label": lab, "unit": unit}
                        break
        used = {self.session_table, self.stage_table} | {o["table"] for o in self.overlays.values()}
        self.related: list[dict] = []
        for t in tables:
            src = t["source"].lower()
            if t["name"] in used or not any(k in src for k in RELATED_KEYWORDS):
                continue
            cols = columns(conn, t["name"])
            link = pick(cols, *LINK_COLS)
            if link or "start_time" in cols:
                self.related.append({"table": t["name"], "source": t["source"], "cols": cols,
                                     "link": link, "snoring": "snor" in src})

    # ------------------------------------------------------------------ #
    # sessions
    # ------------------------------------------------------------------ #

    def sessions(self, start: date | None = None, end: date | None = None) -> list[dict]:
        cols = self.session_cols
        out = []
        for r in self.conn.execute(f'SELECT * FROM "{self.session_table}"'):
            off = r["time_offset"] if "time_offset" in cols else None
            s, e = to_local(r["start_time"], off), to_local(r["end_time"], off)
            if not s or not e or e <= s:
                continue
            if start and not (start <= e.date() <= end):
                continue
            row = dict(r)
            out.append({"row": row, "start": s, "end": e, "offset": off,
                        "utc_start": parse_ts(r["start_time"]), "utc_end": parse_ts(r["end_time"]),
                        "uid": row.get("datauuid")})
        out.sort(key=lambda x: x["start"])
        return out

    def _window(self, table: str, cols, sess, select: str = "*"):
        """Rows of `table` whose start_time falls inside the session."""
        if "start_time" not in cols:
            return []
        sample = self.conn.execute(
            f'SELECT start_time FROM "{table}" WHERE start_time IS NOT NULL LIMIT 1').fetchone()
        if not sample:
            return []
        lo, hi = sess["utc_start"], sess["utc_end"]
        if isinstance(sample[0], (int, float)):
            scale = 1000 if sample[0] > 1e11 else 1
            epoch = datetime(1970, 1, 1)
            lo_v = (lo - epoch).total_seconds() * scale
            hi_v = (hi - epoch).total_seconds() * scale
        else:
            lo_v = lo.strftime("%Y-%m-%d %H:%M:%S")
            hi_v = hi.strftime("%Y-%m-%d %H:%M:%S") + ".999"
        return self.conn.execute(
            f'SELECT {select} FROM "{table}" WHERE start_time >= ? AND start_time <= ? '
            f'ORDER BY start_time', (lo_v, hi_v)).fetchall()

    def _local(self, r, cols, sess, col="start_time"):
        off = r["time_offset"] if "time_offset" in cols and r["time_offset"] else sess["offset"]
        return to_local(r[col], off)

    # ------------------------------------------------------------------ #
    # stages
    # ------------------------------------------------------------------ #

    def stage_segments(self, sess) -> list[tuple[datetime, datetime, str]]:
        t, cols = self.stage_table, self.stage_cols
        if not t or "stage" not in cols:
            return []
        if "sleep_id" in cols:
            if not sess["uid"]:
                return []
            rows = self.conn.execute(f'SELECT * FROM "{t}" WHERE sleep_id = ?', (sess["uid"],)).fetchall()
        else:
            rows = self._window(t, cols, sess)
        segs = []
        for r in rows:
            s, e = self._local(r, cols, sess), self._local(r, cols, sess, "end_time")
            name = STAGES.get(r["stage"])
            if s and e and name and e > s:
                segs.append((s, e, name))
        segs.sort()
        return segs

    @staticmethod
    def stage_stats(segs, sess) -> dict:
        mins = {k: 0.0 for k in STAGE_ORDER}
        for s, e, st in segs:
            mins[st] += (e - s).total_seconds() / 60
        total = sum(mins.values())
        asleep = [x for x in segs if x[2] != "awake"]
        stats = {"minutes": {k: round(v) for k, v in mins.items()},
                 "percent": {k: round(100 * v / total) if total else None for k, v in mins.items()}}
        if asleep:
            onset, final = asleep[0][0], asleep[-1][1]
            wake_in_middle = [x for x in segs if x[2] == "awake" and onset <= x[0] < final]
            stats.update({
                "fell_asleep_min": round((onset - sess["start"]).total_seconds() / 60),
                "awakenings": len(wake_in_middle),
                "awake_after_onset_min": round(sum((e - s).total_seconds() for s, e, _ in wake_in_middle) / 60),
                "asleep_min": round(total - mins["awake"]),
                "rem_periods": _count_runs(segs, "rem"),
            })
        return stats

    # ------------------------------------------------------------------ #
    # overlays (heart rate, SpO2, ...) and related tables (snoring, ...)
    # ------------------------------------------------------------------ #

    def series(self, sess, key: str) -> dict | None:
        o = self.overlays.get(key)
        if not o:
            return None
        has_off = "time_offset" in o["cols"]
        sel = f'start_time, "{o["col"]}" AS v' + (", time_offset" if has_off else "")
        pts = []
        for r in self._window(o["table"], o["cols"], sess, sel):
            v = _num(r["v"])
            if v is None or v <= 0:
                continue
            t = to_local(r["start_time"], (r["time_offset"] if has_off else None) or sess["offset"])
            if t:
                pts.append((t, v))
        if not pts:
            return None
        vals = [v for _, v in pts]
        res = {"label": o["label"], "unit": o["unit"], "samples": len(vals),
               "min": round(min(vals), 1), "avg": round(mean(vals), 1), "max": round(max(vals), 1),
               "points": [[_iso(t), v] for t, v in pts]}
        if key == "spo2":
            res["readings_below_90"] = sum(v < 90 for v in vals)
        return res

    def related_for(self, sess) -> tuple[list[dict], dict | None]:
        out, snoring = [], None
        for rel in self.related:
            t, cols = rel["table"], rel["cols"]
            rows = []
            if rel["link"] and sess["uid"]:
                rows = self.conn.execute(f'SELECT * FROM "{t}" WHERE "{rel["link"]}" = ?',
                                         (sess["uid"],)).fetchall()
            if not rows:
                rows = self._window(t, cols, sess)
            if not rows:
                continue
            item = {"table": t, "source": rel["source"], "rows": len(rows)}
            segs = []
            if "start_time" in cols:
                dur_col = pick(cols, "duration")
                for r in rows:
                    s = self._local(r, cols, sess)
                    e = self._local(r, cols, sess, "end_time") if "end_time" in cols else None
                    if s and not e and dur_col and _num(r[dur_col]):
                        d = r[dur_col]
                        e = s + timedelta(milliseconds=d) if d > 1000 else s + timedelta(seconds=d)
                    if s and e and e >= s:
                        segs.append((s, e))
            segs.sort()
            if segs:
                item["total_min"] = round(sum((e - s).total_seconds() for s, e in segs) / 60, 1)
                item["segments"] = [[_iso(s), _iso(e)] for s, e in segs]
            fields = {}
            for c in cols:
                if c in HIDE or c == rel["link"]:
                    continue
                vals = [r[c] for r in rows if r[c] is not None and r[c] != ""]
                nums = [v for v in vals if _num(v) is not None]
                if len(rows) == 1 and vals and not (isinstance(vals[0], str) and len(vals[0]) > 60):
                    fields[c] = vals[0]
                elif nums:
                    fields[c] = round(mean(nums), 2)
            item["fields"] = fields
            out.append(item)
            if rel["snoring"]:
                snoring = {"episodes": len(segs) or len(rows),
                           "minutes": item.get("total_min"),
                           "segments": item.get("segments", []), "table": t}
        if snoring is None:  # snoring is tracked in this export, just none tonight
            snore_tables = [r["table"] for r in self.related if r["snoring"]]
            if snore_tables:
                snoring = {"episodes": 0, "minutes": 0, "segments": [], "table": snore_tables[0]}
        return out, snoring

    def session_fields(self, sess) -> list[dict]:
        out = []
        for k, v in sess["row"].items():
            if k in HIDE or v is None or v == "" or (isinstance(v, str) and len(v) > 60):
                continue
            out.append({"key": k, "label": label(k), "value": v, "pretty": pretty(k, v)})
        return out

    # ------------------------------------------------------------------ #
    # public API
    # ------------------------------------------------------------------ #

    def nights(self, start: date, end: date, all_fields: bool = False) -> list[dict]:
        """One summary row per sleep session (attributed to the wake-up date)."""
        out = []
        for sess in self.sessions(start, end):
            segs = self.stage_segments(sess)
            st = self.stage_stats(segs, sess) if segs else {}
            r = sess["row"]
            rec = {
                "date": sess["end"].date().isoformat(),
                "bedtime": sess["start"].strftime("%H:%M"),
                "wake": sess["end"].strftime("%H:%M"),
                "hours": round((sess["end"] - sess["start"]).total_seconds() / 3600, 2),
                "score": _num(r.get(self.score_col)) if self.score_col else None,
                "efficiency": _num(r.get(self.eff_col)) if self.eff_col else None,
            }
            for k in ("deep", "rem", "light", "awake"):
                rec[f"{k}_min"] = st["minutes"][k] if st else None
            rec["awakenings"] = st.get("awakenings")
            _, snoring = self.related_for(sess) if self.related else ([], None)
            rec["snore_min"] = snoring["minutes"] if snoring else None
            for key, fields in (("heart_rate", ("hr_avg", "hr_min")), ("spo2", ("spo2_avg", "spo2_min")),
                                ("respiratory_rate", ("resp_avg", None))):
                s = self.series(sess, key)
                rec[fields[0]] = s["avg"] if s else None
                if fields[1]:
                    rec[fields[1]] = s["min"] if s else None
            if all_fields:
                for f in self.session_fields(sess):
                    rec.setdefault(f["key"], f["value"])
            rec["id"] = sess["uid"]
            out.append(rec)
        return out

    def latest_night(self) -> date | None:
        s = self.sessions()
        return s[-1]["end"].date() if s else None

    def night(self, day: date | None = None) -> dict:
        day = day or self.latest_night()
        if not day:
            raise NoDataError("No sleep sessions found.")
        sessions = self.sessions(day, day)
        if not sessions:
            raise NoDataError(f"No sleep session ending on {day}. Try `shealth sleep` to list nights.")
        main = max(sessions, key=lambda s: s["end"] - s["start"])
        segs = self.stage_segments(main)
        related, snoring = self.related_for(main)
        series = {k: s for k in self.overlays if (s := self.series(main, k))}
        return {
            "date": day.isoformat(),
            "start": _iso(main["start"]),
            "end": _iso(main["end"]),
            "hours": round((main["end"] - main["start"]).total_seconds() / 3600, 2),
            "id": main["uid"],
            "score": _num(main["row"].get(self.score_col)) if self.score_col else None,
            "efficiency": _num(main["row"].get(self.eff_col)) if self.eff_col else None,
            "measurements": self.session_fields(main),
            "stages": {"segments": [[_iso(s), _iso(e), st] for s, e, st in segs],
                       **(self.stage_stats(segs, main) if segs else {})},
            "snoring": snoring,
            "series": series,
            "related": related,
            "other_sessions": [{"start": _iso(s["start"]), "end": _iso(s["end"]),
                                "hours": round((s["end"] - s["start"]).total_seconds() / 3600, 2)}
                               for s in sessions if s is not main],
            "available": {"stages": bool(self.stage_table), "overlays": sorted(self.overlays),
                          "related_tables": [r["table"] for r in self.related]},
        }


def _count_runs(segs, stage: str, gap_min: float = 15) -> int:
    """Count separate periods of `stage`, merging ones less than `gap_min` apart."""
    runs, last_end = 0, None
    for s, e, st in segs:
        if st != stage:
            continue
        if last_end is None or (s - last_end).total_seconds() / 60 > gap_min:
            runs += 1
        last_end = e
    return runs
