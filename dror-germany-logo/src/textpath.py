"""Shape text with HarfBuzz and emit it as outlined SVG paths.

HarfBuzz handles RTL ordering for Hebrew, so the returned path is already in
visual order; nothing in the output depends on an installed font.
"""
from pathlib import Path

import re

import uharfbuzz as hb
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen

FONTS = Path(__file__).resolve().parent.parent / "fonts"

_cache = {}


def _font(name, wght):
    key = (name, wght)
    if key not in _cache:
        blob = hb.Blob.from_file_path(str(FONTS / name))
        face = hb.Face(blob)
        font = hb.Font(face)
        if wght is not None:
            font.set_variations({"wght": wght})
        _cache[key] = (font, face.upem)
    return _cache[key]


def text_path(text, font, size, x, y, anchor="middle", wght=None,
              tracking=0.0, direction=None):
    """Return (svg_path_d, width) for `text` with its baseline at y.

    anchor: 'start' | 'middle' | 'end' of the visual run.
    tracking: extra space between glyphs, in em units (e.g. 0.05).
    """
    hbfont, upem = _font(font, wght)
    buf = hb.Buffer()
    buf.add_str(text)
    buf.guess_segment_properties()
    if direction:
        buf.direction = direction
    hb.shape(hbfont, buf, {"kern": True, "liga": True})

    scale = size / upem
    track = tracking * upem
    n = len(buf.glyph_positions)
    advance = sum(p.x_advance for p in buf.glyph_positions) + track * (n - 1)
    width = advance * scale
    x0 = {"start": x, "middle": x - width / 2, "end": x - width}[anchor]

    pen = SVGPathPen(None, ntos=lambda v: f"{v:.2f}".rstrip("0").rstrip("."))
    cx = 0
    for info, pos in zip(buf.glyph_infos, buf.glyph_positions):
        tx = x0 + (cx + pos.x_offset) * scale
        ty = y - pos.y_offset * scale
        tpen = TransformPen(pen, (scale, 0, 0, -scale, tx, ty))
        hbfont.draw_glyph_with_pen(info.codepoint, tpen)
        cx += pos.x_advance + track
    return pen.getCommands(), width


_LTR = re.compile(r"([0-9A-Za-z\u00C0-\u024F][0-9A-Za-z\u00C0-\u024F.]*)")


def line_path(text, font, size, x, y, anchor="middle", wght=None,
              tracking=0.0):
    """Like text_path, but handles LTR runs (digits, Latin) inside Hebrew.

    HarfBuzz shapes a single direction only, so an RTL line is split into
    runs: Hebrew runs shaped RTL, digit/Latin runs shaped LTR, and the runs
    laid out right to left.
    """
    if not re.search("[\u0590-\u05FF]", text):
        return text_path(text, font, size, x, y, anchor, wght, tracking, "ltr")
    runs = [r for r in _LTR.split(text) if r]
    shaped = [(r, "ltr" if _LTR.fullmatch(r) else "rtl") for r in runs]
    widths = [text_path(r, font, size, 0, 0, "start", wght, tracking, d)[1]
              for r, d in shaped]
    gap = tracking * size
    total = sum(widths) + gap * (len(widths) - 1)
    x0 = {"start": x, "middle": x - total / 2, "end": x - total}[anchor]
    # Logical order runs right to left: the first run sits at the right edge.
    parts, cursor = [], x0 + total
    for (r, d), w in zip(shaped, widths):
        cursor -= w
        parts.append(text_path(r, font, size, cursor, y, "start", wght,
                               tracking, d)[0])
        cursor -= gap
    return " ".join(parts), total


def glyph_run(text, font, size, wght=None, tracking=0.0):
    """Glyphs of one line in visual order, for setting text along a curve.

    Returns ([(path, x, advance)], total_width): each glyph's outline at its
    place on a baseline at y=0, runs ordered as line_path orders them.
    """
    import pathops

    hbfont, upem = _font(font, wght)
    scale = size / upem
    track = tracking * upem
    if re.search("[֐-׿]", text):
        runs = [r for r in _LTR.split(text) if r]
        runs = [(r, "ltr" if _LTR.fullmatch(r) else "rtl") for r in runs]
    else:
        runs = [(text, "ltr")]
    shaped = []
    for r, d in runs:
        buf = hb.Buffer()
        buf.add_str(r)
        buf.guess_segment_properties()
        buf.direction = d
        hb.shape(hbfont, buf, {"kern": True, "liga": True})
        w = (sum(p.x_advance for p in buf.glyph_positions) + track * len(buf.glyph_positions)) * scale
        shaped.append((buf, w))
    total = sum(w for _, w in shaped)
    # An RTL line puts its first run at the right edge; an LTR line at the left.
    rtl = runs[0][1] == "rtl" or len(runs) > 1
    out, cursor = [], total
    order = shaped if rtl else list(reversed(shaped))
    for buf, w in order:
        cursor -= w
        cx = 0
        for info, pos in zip(buf.glyph_infos, buf.glyph_positions):
            p = pathops.Path()
            x = cursor + (cx + pos.x_offset) * scale
            hbfont.draw_glyph_with_pen(
                info.codepoint,
                TransformPen(p.getPen(), (scale, 0, 0, -scale, x, -pos.y_offset * scale)))
            adv = (pos.x_advance + track) * scale
            out.append((p, x, adv))
            cx += pos.x_advance + track
    return out, total
