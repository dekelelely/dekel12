"""Output: aligned text tables with inline bars, JSON, or CSV."""

from __future__ import annotations

import csv
import json
import sys

BLOCKS = " ▏▎▍▌▋▊▉█"


def bar(value, vmax, width=20) -> str:
    if not isinstance(value, (int, float)) or not vmax or value <= 0:
        return ""
    cells = max(0.0, min(1.0, value / vmax)) * width
    full = int(cells)
    part = BLOCKS[int((cells - full) * 8)] if full < width else ""
    return ("█" * full + part).rstrip()


def _fmt(v) -> str:
    if v is None:
        return "–"
    if isinstance(v, float):
        return f"{v:,.1f}" if abs(v) < 1000 else f"{v:,.0f}"
    if isinstance(v, int) and abs(v) >= 1000:
        return f"{v:,}"
    return str(v)


def table(rows: list[dict], cols: list[str] | None = None, bar_col: str | None = None,
          footer: dict | None = None, out=None):
    out = out or sys.stdout
    if not rows:
        print("(no data in this date range)", file=out)
        return
    cols = list(cols or rows[0].keys())
    body = [[_fmt(r.get(c)) for c in cols] for r in rows]
    numeric = [all(isinstance(r.get(c), (int, float)) or r.get(c) is None for r in rows) for c in cols]
    foot = [_fmt(footer.get(c)) if c in footer else "" for c in cols] if footer else None
    if bar_col in cols:  # inline bar chart right after its column
        i = cols.index(bar_col) + 1
        vmax = max((r.get(bar_col) or 0 for r in rows), default=0)
        cols.insert(i, "")
        numeric.insert(i, False)
        for r, b in zip(rows, body):
            b.insert(i, bar(r.get(bar_col), vmax))
        if foot:
            foot.insert(i, "")
    widths = [max(len(c), *(len(b[i]) for b in body + ([foot] if foot else [])))
              for i, c in enumerate(cols)]
    rule = "  ".join("─" * w for w in widths)

    def line(cells):
        return "  ".join(c.rjust(w) if num else c.ljust(w)
                         for c, w, num in zip(cells, widths, numeric)).rstrip()

    print(line(cols), file=out)
    print(rule, file=out)
    for b in body:
        print(line(b), file=out)
    if foot:
        print(rule, file=out)
        print(line(foot), file=out)


def emit(rows, fmt: str, cols=None, bar_col=None, footer=None, out=None):
    out = out or sys.stdout
    if fmt == "json":
        json.dump(rows, out, indent=2, default=str)
        out.write("\n")
    elif fmt == "csv":
        if not rows:
            return
        w = csv.DictWriter(out, fieldnames=cols or list(rows[0].keys()), extrasaction="ignore", lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    else:
        table(rows, cols, bar_col, footer, out)
