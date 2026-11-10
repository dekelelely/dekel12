"""shealth — query your Samsung Health data from the terminal.

Workflow:
  1. Phone: Samsung Health → ☰/Settings → "Download personal data" → Download.
     The export lands in  Internal storage/Download/Samsung Health/<folder>
  2. Copy that folder (or a zip of it) to your computer.
  3. shealth import <folder-or-zip>
  4. shealth summary | steps | sleep | heart | workouts | weight | stress | spo2 | sql
"""

from __future__ import annotations

import argparse
import json
import sqlite3
import sys
from datetime import date, timedelta
from pathlib import Path

from . import __version__, demo, metrics, render
from .db import DEFAULT_DB, NoDataError, columns, connect, import_export, meta


# --------------------------------------------------------------------------- #
# helpers
# --------------------------------------------------------------------------- #

def _window(args, conn) -> tuple[date, date]:
    end = args.to or metrics.latest_date(conn) or date.today()
    start = args.from_ or end - timedelta(days=args.days - 1)
    if start > end:
        raise SystemExit("error: --from is after --to")
    return start, end


def _header(args, start, end, title):
    if args.format == "table":
        print(f"{title}  ·  {start} → {end}\n")


def _metric_cmd(fn, title, bar_col=None, avg_keys=(), cols=None, total_key=None):
    def run(args):
        conn = connect(args.db, readonly=True)
        start, end = _window(args, conn)
        rows = fn(conn, start, end)
        footer = {"date": "average", **metrics.averages(rows, avg_keys)} if rows and avg_keys else None
        _header(args, start, end, title)
        render.emit(rows, args.format, cols, bar_col, footer)
        if total_key and rows and args.format == "table":
            print(f"\nTotal {total_key}: {sum(r[total_key] or 0 for r in rows):,} over {len(rows)} days")
    return run


# --------------------------------------------------------------------------- #
# commands
# --------------------------------------------------------------------------- #

def cmd_import(args):
    info = import_export(args.path, args.db)
    if args.format == "json":
        print(json.dumps(info, indent=2))
        return
    print(f"Imported {info['rows']:,} rows from {info['tables']} tables into {args.db}")
    if info["exported_at"]:
        print(f"Export date: {info['exported_at']}  (date windows end here by default)")
    print("Next: `shealth summary`, or `shealth tables` to see everything that was imported.")


def cmd_demo(args):
    out = Path(args.out)
    demo.build(out, days=args.demo_days)
    print(f"Wrote fake export to {out}")
    if not args.no_import:
        info = import_export(out, args.db)
        print(f"Imported {info['rows']:,} rows into {args.db}. Try `shealth summary`.")


def cmd_tables(args):
    conn = connect(args.db, readonly=True)
    if args.name:
        name = args.name
        r = conn.execute("SELECT name FROM _shealth_tables WHERE name = ? OR source = ?",
                         (name, name)).fetchone()
        if not r:
            raise NoDataError(f"No table named {name!r}. Run `shealth tables` to list them.")
        name = r["name"]
        rows = [dict(x) for x in conn.execute(f'SELECT * FROM "{name}" LIMIT ?', (args.limit,))]
        if args.format != "table":
            render.emit(rows, args.format)
            return
        print(f"{name}: {', '.join(columns(conn, name))}\n")
        for row in rows:
            print(json.dumps({k: v for k, v in row.items() if v is not None}, default=str))
        return
    rows = [dict(x) for x in conn.execute(
        "SELECT name, rows, source FROM _shealth_tables ORDER BY rows DESC")]
    if args.format == "table":
        m = meta(conn)
        print(f"Export: {m.get('source_path')}\nExported: {m.get('exported_at') or '?'}"
              f"   Imported: {m.get('imported_at')}\n")
    render.emit(rows, args.format)


def cmd_summary(args):
    conn = connect(args.db, readonly=True)
    start, end = _window(args, conn)
    rows = metrics.summary(conn, start, end)
    avg = metrics.averages(rows, ["steps", "sleep_h", "hr_avg", "hr_low", "stress"])
    lw = metrics.latest_weight(conn)
    if args.format == "json":
        print(json.dumps({"from": str(start), "to": str(end), "days": rows,
                          "averages": avg, "latest_weight": lw}, indent=2))
        return
    if args.format == "csv":
        render.emit(rows, "csv")
        return
    _header(args, start, end, "Daily summary")
    render.table(rows, bar_col="steps", footer={"date": "average", **avg})
    if lw:
        print(f"\nLatest weight: {lw['weight_kg']} kg on {lw['date']}")


def cmd_sql(args):
    conn = connect(args.db, readonly=True)
    q = args.query if args.query != "-" else sys.stdin.read()
    try:
        rows = [dict(r) for r in conn.execute(q)]
    except sqlite3.Error as e:
        raise SystemExit(f"SQL error: {e}")
    render.emit(rows, args.format)


# --------------------------------------------------------------------------- #
# parser
# --------------------------------------------------------------------------- #

def build_parser() -> argparse.ArgumentParser:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--db", type=Path, default=DEFAULT_DB,
                        help=f"database path (default: $SHEALTH_DB or {DEFAULT_DB})")
    fmt = common.add_mutually_exclusive_group()
    fmt.add_argument("-f", "--format", choices=["table", "json", "csv"], default="table")
    fmt.add_argument("--json", dest="format", action="store_const", const="json", help="same as -f json")
    fmt.add_argument("--csv", dest="format", action="store_const", const="csv", help="same as -f csv")

    def window(default_days):
        w = argparse.ArgumentParser(add_help=False)
        w.add_argument("-d", "--days", type=int, default=default_days,
                       help=f"number of days to show (default {default_days})")
        w.add_argument("--from", dest="from_", type=date.fromisoformat, metavar="YYYY-MM-DD")
        w.add_argument("--to", type=date.fromisoformat, metavar="YYYY-MM-DD",
                       help="end date (default: the day of the export)")
        return w

    p = argparse.ArgumentParser(prog="shealth", description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--version", action="version", version=f"shealth {__version__}")
    sub = p.add_subparsers(dest="cmd", required=True, metavar="COMMAND")

    s = sub.add_parser("import", parents=[common], help="import a Samsung Health export (folder or .zip)")
    s.add_argument("path")
    s.set_defaults(func=cmd_import)

    s = sub.add_parser("demo", parents=[common], help="generate + import a fake export to try the tool")
    s.add_argument("--out", default="shealth-demo-export.zip")
    s.add_argument("--demo-days", type=int, default=60)
    s.add_argument("--no-import", action="store_true")
    s.set_defaults(func=cmd_demo)

    s = sub.add_parser("tables", parents=[common], help="list imported tables, or inspect one")
    s.add_argument("name", nargs="?")
    s.add_argument("-n", "--limit", type=int, default=5)
    s.set_defaults(func=cmd_tables)

    s = sub.add_parser("summary", parents=[common, window(7)], help="day-by-day overview")
    s.set_defaults(func=cmd_summary)

    specs = [
        ("steps", metrics.steps, "Steps", "steps", ["steps", "distance_km", "active_kcal"], 14, "steps"),
        ("sleep", metrics.sleep, "Sleep sessions", "hours",
         ["hours", "score", "efficiency", "deep_min", "rem_min", "light_min", "awake_min"], 14, None),
        ("heart", metrics.heart_rate, "Heart rate (bpm; low = 5th percentile)", None,
         ["low", "avg", "max"], 14, None),
        ("stress", metrics.stress, "Stress score", "avg", ["avg", "max"], 14, None),
        ("spo2", metrics.spo2, "Blood oxygen (%)", None, ["avg", "min"], 14, None),
        ("weight", metrics.weight, "Weight", None, ["weight_kg", "body_fat", "skeletal_muscle_mass"], 90, None),
        ("workouts", metrics.workouts, "Workouts", "minutes", [], 30, None),
    ]
    for name, fn, title, bar_col, avg_keys, days, total in specs:
        s = sub.add_parser(name, parents=[common, window(days)], help=title.split(" (")[0].lower())
        s.set_defaults(func=_metric_cmd(fn, title, bar_col, avg_keys, total_key=total))

    s = sub.add_parser("sql", parents=[common], help="run a read-only SQL query ('-' reads stdin)",
                       description="Tables are listed by `shealth tables`. Helper functions: "
                                   "sh_local(start_time, time_offset) → local datetime, "
                                   "sh_date(start_time, time_offset) → local date.")
    s.add_argument("query")
    s.set_defaults(func=cmd_sql)
    return p


def main(argv=None) -> int:
    args = build_parser().parse_args(argv)
    try:
        args.func(args)
    except NoDataError as e:
        print(f"shealth: {e}", file=sys.stderr)
        return 1
    except BrokenPipeError:  # e.g. `shealth heart | head`
        return 0
    return 0
