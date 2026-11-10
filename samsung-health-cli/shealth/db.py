"""SQLite storage + importer for Samsung Health "Download personal data" exports.

Every CSV in the export becomes one SQLite table, so *nothing* is lost even for
data types this tool doesn't have a dedicated command for (use ``shealth sql``).

Export quirks handled here:
  * line 1 is often a metadata line (``com.samsung.shealth.sleep,6302001,4``),
    the real header is line 2;
  * column names are prefixed (``com.samsung.health.sleep.start_time``) — we
    store the short form (``start_time``);
  * rows end with a trailing comma (an extra empty column);
  * re-exporting into the same folder leaves several copies of each table with
    different timestamp suffixes — we keep the newest.
"""

from __future__ import annotations

import csv
import io
import os
import re
import sqlite3
import sys
import zipfile
from datetime import datetime
from pathlib import Path

from .timeutil import to_local

DEFAULT_DB = Path(os.environ.get("SHEALTH_DB") or Path.home() / ".shealth" / "health.db")

_TS_SUFFIX = re.compile(r"\.(\d{12,14})$")
_INT_RE = re.compile(r"-?\d{1,18}")
_FLOAT_RE = re.compile(r"-?\d+\.\d*(?:[eE][-+]?\d+)?|-?\d+[eE][-+]?\d+")
_PREFIXES = ("com.samsung.shealth.", "com.samsung.health.")

csv.field_size_limit(min(sys.maxsize, 2**31 - 1))


class NoDataError(Exception):
    """Raised when the DB is missing or doesn't contain what a command needs."""


# --------------------------------------------------------------------------- #
# connection
# --------------------------------------------------------------------------- #

def connect(path: Path | str = DEFAULT_DB, readonly: bool = False, must_exist: bool = True):
    path = Path(path)
    if must_exist and not path.exists():
        raise NoDataError(
            f"No database at {path}. Run `shealth import <export.zip|folder>` first "
            f"(or `shealth demo` to try it with fake data)."
        )
    if readonly:
        conn = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        conn = sqlite3.connect(path)
    conn.row_factory = sqlite3.Row
    # Usable from `shealth sql`: SELECT sh_local(start_time, time_offset) ...
    conn.create_function("sh_local", 2, _sql_local, deterministic=True)
    conn.create_function("sh_date", 2, _sql_date, deterministic=True)
    return conn


def _sql_local(ts, off):
    dt = to_local(ts, off)
    return dt.isoformat(sep=" ", timespec="seconds") if dt else None


def _sql_date(ts, off):
    dt = to_local(ts, off)
    return dt.date().isoformat() if dt else None


# --------------------------------------------------------------------------- #
# naming helpers
# --------------------------------------------------------------------------- #

def source_name(filename: str) -> tuple[str, str]:
    """``com.samsung.shealth.sleep.20240301120000.csv`` -> (source, timestamp)."""
    stem = Path(filename).name
    if stem.lower().endswith(".csv"):
        stem = stem[:-4]
    m = _TS_SUFFIX.search(stem)
    if m:
        return stem[: m.start()], m.group(1)
    return stem, ""


def sql_name(source: str) -> str:
    n = source
    for p in _PREFIXES:
        if n.startswith(p):
            n = n[len(p):]
            break
    n = re.sub(r"\W+", "_", n).strip("_").lower()
    return n or "unnamed"


def short_columns(header: list[str]) -> list[str]:
    out, seen = [], set()
    for i, raw in enumerate(header):
        raw = raw.strip()
        short = raw.rsplit(".", 1)[-1] if raw else f"col{i}"
        short = re.sub(r"\W+", "_", short).strip("_").lower() or f"col{i}"
        if short in seen:
            short = re.sub(r"\W+", "_", raw).strip("_").lower() or f"col{i}"
            while short in seen:
                short += "_"
        seen.add(short)
        out.append(short)
    return out


def coerce(v: str):
    if v is None:
        return None
    v = v.strip()
    if v == "":
        return None
    if _INT_RE.fullmatch(v):
        return int(v)
    if _FLOAT_RE.fullmatch(v):
        return float(v)
    return v


# --------------------------------------------------------------------------- #
# reading the export
# --------------------------------------------------------------------------- #

def _iter_csv_sources(path: Path):
    """Yield (filename, opener) for every CSV in a folder or zip."""
    if path.is_file() and zipfile.is_zipfile(path):
        zf = zipfile.ZipFile(path)
        for info in zf.infolist():
            if info.filename.lower().endswith(".csv") and not info.is_dir():
                yield info.filename, (lambda i=info: io.TextIOWrapper(
                    zf.open(i), encoding="utf-8-sig", errors="replace", newline=""))
    elif path.is_dir():
        for p in sorted(path.rglob("*.csv")):
            yield str(p), (lambda p=p: open(p, encoding="utf-8-sig", errors="replace", newline=""))
    else:
        raise NoDataError(f"{path} is not a folder or a .zip file")


def read_table(fh, source: str):
    """Return (columns, rows_iterator) for one export CSV."""
    reader = csv.reader(fh)
    first = next(reader, None)
    if first is None:
        return [], iter(())
    is_meta = bool(first) and (
        first[0].strip() == source
        or (len(first) <= 4 and any(f.strip().isdigit() for f in first[1:]))
    )
    header = next(reader, None) if is_meta else first
    if not header:
        return [], iter(())
    while header and header[-1].strip() == "":  # trailing comma
        header = header[:-1]
    cols = short_columns(header)
    n = len(cols)

    def rows():
        for r in reader:
            if not r or all(c.strip() == "" for c in r):
                continue
            r = (r + [""] * n)[:n]
            yield [coerce(c) for c in r]

    return cols, rows()


def import_export(path: Path | str, db_path: Path | str = DEFAULT_DB, log=print) -> dict:
    path = Path(path).expanduser()
    if not path.exists():
        raise NoDataError(f"{path} does not exist")

    # Pick newest copy of each table (re-exports leave duplicates behind).
    chosen: dict[str, tuple[str, str, object]] = {}
    for fname, opener in _iter_csv_sources(path):
        src, ts = source_name(fname)
        if src not in chosen or ts > chosen[src][1]:
            chosen[src] = (fname, ts, opener)
    if not chosen:
        raise NoDataError(f"No CSV files found in {path}. Is this a Samsung Health export?")

    conn = connect(db_path, must_exist=False)
    cur = conn.cursor()
    for (t,) in cur.execute(
        "SELECT name FROM sqlite_master WHERE type='table'").fetchall():
        cur.execute(f'DROP TABLE "{t}"')
    cur.execute("CREATE TABLE _shealth_tables (name TEXT PRIMARY KEY, source TEXT, file TEXT, rows INTEGER)")
    cur.execute("CREATE TABLE _shealth_meta (key TEXT PRIMARY KEY, value TEXT)")

    used, total_rows, newest_ts = set(), 0, ""
    for src in sorted(chosen):
        fname, ts, opener = chosen[src]
        newest_ts = max(newest_ts, ts)
        name = sql_name(src)
        while name in used or name.startswith("_shealth"):
            name += "_"
        used.add(name)
        with opener() as fh:
            cols, rows = read_table(fh, src)
            if not cols:
                continue
            col_sql = ", ".join(f'"{c}"' for c in cols)
            cur.execute(f'CREATE TABLE "{name}" ({col_sql})')
            ph = ", ".join("?" * len(cols))
            cur.executemany(f'INSERT INTO "{name}" VALUES ({ph})', rows)
            n = cur.execute(f'SELECT COUNT(*) FROM "{name}"').fetchone()[0]
        cur.execute("INSERT INTO _shealth_tables VALUES (?,?,?,?)", (name, src, fname, n))
        total_rows += n

    exported_at = ""
    if newest_ts:
        try:
            exported_at = datetime.strptime(newest_ts[:14].ljust(14, "0"), "%Y%m%d%H%M%S").isoformat(sep=" ")
        except ValueError:
            pass
    meta = {
        "source_path": str(path.resolve()),
        "imported_at": datetime.now().isoformat(sep=" ", timespec="seconds"),
        "exported_at": exported_at,
    }
    cur.executemany("INSERT INTO _shealth_meta VALUES (?,?)", meta.items())
    conn.commit()
    conn.close()
    return {"tables": len(used), "rows": total_rows, **meta}


# --------------------------------------------------------------------------- #
# lookup helpers used by metrics
# --------------------------------------------------------------------------- #

def meta(conn) -> dict:
    try:
        return {r["key"]: r["value"] for r in conn.execute("SELECT key, value FROM _shealth_meta")}
    except sqlite3.OperationalError:
        raise NoDataError("Database is empty. Run `shealth import` first.")


def find_table(conn, *sources: str) -> str | None:
    """Return the SQLite name of the first table that exists for the given
    Samsung source names (e.g. ``com.samsung.shealth.sleep``)."""
    for s in sources:
        r = conn.execute("SELECT name FROM _shealth_tables WHERE source = ?", (s,)).fetchone()
        if r:
            return r["name"]
    return None


def columns(conn, table: str) -> list[str]:
    return [r[1] for r in conn.execute(f'PRAGMA table_info("{table}")')]


def pick(cols, *candidates: str) -> str | None:
    s = set(cols)
    for c in candidates:
        if c in s:
            return c
    return None
