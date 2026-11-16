"""Small boolean-geometry helpers so every shape ships as a plain filled path."""
import pathops
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.svgLib.path import parse_path


def P(d):
    p = pathops.Path()
    parse_path(d, p.getPen())
    return p


def stroke(d, width, cap="round", join="round"):
    p = P(d)
    p.stroke(width, getattr(pathops.LineCap, f"{cap.upper()}_CAP"),
             getattr(pathops.LineJoin, f"{join.upper()}_JOIN"), 4)
    p.convertConicsToQuads()
    return p


def _as_path(x):
    return P(x) if isinstance(x, str) else x


def _clean(x):
    # Each operand is simplified on its own first; otherwise contours drawn
    # in opposite directions cancel out and a union turns into an XOR.
    return pathops.simplify(_as_path(x), clockwise=True)


def union(*xs):
    out = _clean(xs[0])
    for x in xs[1:]:
        out = pathops.op(out, _clean(x), pathops.PathOp.UNION)
    return out


def diff(a, *bs):
    out = _clean(a)
    for b in bs:
        out = pathops.op(out, _clean(b), pathops.PathOp.DIFFERENCE)
    return out


def to_d(x):
    pen = SVGPathPen(None, ntos=lambda v: f"{v:.2f}".rstrip("0").rstrip("."))
    _as_path(x).draw(pen)
    return pen.getCommands()


def circle(cx, cy, r):
    return (f"M{cx - r},{cy} A{r},{r} 0 1 0 {cx + r},{cy} "
            f"A{r},{r} 0 1 0 {cx - r},{cy} Z")


def rect(x, y, w, h, r=0):
    if not r:
        return f"M{x},{y} H{x + w} V{y + h} H{x} Z"
    return (f"M{x + r},{y} H{x + w - r} A{r},{r} 0 0 1 {x + w},{y + r} "
            f"V{y + h - r} A{r},{r} 0 0 1 {x + w - r},{y + h} H{x + r} "
            f"A{r},{r} 0 0 1 {x},{y + h - r} V{y + r} A{r},{r} 0 0 1 {x + r},{y} Z")


def mirror(d, axis=500):
    return _as_path(d).transform(-1, 0, 0, 1, 2 * axis, 0)
