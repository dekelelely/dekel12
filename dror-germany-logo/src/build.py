"""Build every concept: lockup + mark, in color / reverse / black / white.

Run from anywhere:  python3 src/build.py
All text is shaped with HarfBuzz and written as outlined paths.
"""
from pathlib import Path

from geo import P, _as_path as _P, circle, diff, mirror, rect, stroke, to_d, union
from textpath import line_path

ROOT = Path(__file__).resolve().parent.parent

BLUE = "#0F408E"      # sampled from the school's existing logo
INK = "#0B2D63"       # deeper blue for the name
GOLD = "#C49A3C"      # warm stone/gold accent: light, memory
GOLD_LT = "#E2BE68"   # gold that holds up on dark backgrounds

VARIANTS = {
    "color":   {"primary": BLUE, "accent": GOLD, "text": INK, "sub": BLUE},
    "reverse": {"primary": "#FFFFFF", "accent": GOLD_LT, "text": "#FFFFFF",
                "sub": "#D6E0F0"},
    "black":   dict.fromkeys(("primary", "accent", "text", "sub"), "#000000"),
    "white":   dict.fromkeys(("primary", "accent", "text", "sub"), "#FFFFFF"),
}

NAME = "קריית חינוך דרור"


# --- Concept 1: the swallow bridge -----------------------------------------

def swallow(heavy=False):
    # Wings root at the shoulders, just under the head.
    wing = P("M492,226 C402,158 248,182 150,452 "
             "C255,280 398,252 494,292 Z")
    head = circle(500, 196, 33)
    body = P("M500,190 C522,190 534,206 533,232 C531,268 527,306 522,342 "
             "L566,508 C544,462 522,418 500,392 "
             "C478,418 456,462 434,508 L478,342 "
             "C473,306 469,268 467,232 C466,206 478,190 500,190 Z")
    bird = union(wing, mirror(wing), body, head)
    deck = rect(190, 548, 620, 14, 7) if not heavy else rect(250, 530, 500, 28, 14)
    return [("primary", bird), ("accent", deck)]


# --- Concept 2: flame into sprout ------------------------------------------

def flame(heavy=False):
    outer = P("M526,76 C560,190 648,262 648,414 C648,514 584,584 500,584 "
              "C416,584 352,514 352,414 C352,318 382,252 408,184 "
              "C424,236 446,266 472,280 C466,206 486,146 526,76 Z")
    w = 22 if not heavy else 30
    stem = rect(500 - w / 2, 372, w, 168, w / 2)
    leaf_l = P("M504,420 C462,428 416,400 398,342 C452,334 494,362 504,404 Z")
    leaf_r = P("M496,404 C540,404 586,372 602,310 C548,306 506,336 496,384 Z")
    if heavy:
        leaf_l = union(leaf_l, stroke("M500,414 C462,420 424,394 406,346", 8))
        leaf_r = union(leaf_r, stroke("M500,398 C540,398 580,368 594,316", 8))
    sprout = union(stem, leaf_l, leaf_r)
    base = rect(400, 624, 200, 20 if not heavy else 30, 10 if not heavy else 15)
    return [("primary", diff(outer, sprout)), ("accent", base)]


# --- Concept 3: Hebrew dalet meets Latin D ---------------------------------

def dalet_d(heavy=False):
    t = 70 if not heavy else 84
    top, bot = 160, 560
    gap = 16 if not heavy else 20
    # Latin D on the left, its bowl opening toward the right.
    dx = 230
    r = (bot - top) / 2
    bx = dx + t + 40                       # where the bowl starts to curve
    bowl = P(f"M{dx},{top} H{bx} A{r},{r} 0 0 1 {bx},{bot} H{dx} Z")
    counter = P(f"M{dx + t},{top + t} H{bx} A{r - t},{r - t} 0 0 1 {bx},{bot - t} "
                f"H{dx + t} Z")
    latin_d = diff(bowl, counter)
    # Hebrew dalet on the right, facing the D. The end of its roof is cut
    # along the D's curve, so the two letters meet with one even hairline.
    # The roof runs past the leg on the right: that ear is what makes it a
    # dalet and not a 7.
    lx = 790 - t
    ear = round(t * 0.26)
    dalet = diff(union(rect(bx, top, 790 + ear - bx, t),
                       rect(lx, top, t, bot - top)),
                 circle(bx, (top + bot) / 2, r + gap))
    return [("accent", latin_d), ("primary", dalet)]


# --- Concept 4: two cities, one line ---------------------------------------

def skyline(heavy=False):
    if heavy:
        return skyline_compact()
    sw = 16
    lines = [
        # Jerusalem: crenellated wall, arched gate, citadel tower.
        "M100,520 H330",
        "M175,520 V412 H195 V392 H218 V412 H241 V392 H264 V412 H287 V392 "
        "H310 V412 H330 V520",
        "M228,520 V472 A24,24 0 0 1 276,472 V520",
        "M120,520 V318 H160 V520",
        "M120,318 C120,288 160,288 160,318",
        "M140,296 V270",
        # The bridge between them: one line rising and landing.
        "M330,520 C420,420 590,420 680,520",
        # Berlin: glass dome on a low block, and the TV tower.
        "M680,520 H900",
        "M700,520 V448 H822 V520",
        "M720,448 A41,41 0 0 1 802,448",
        "M761,407 V448",
        "M866,520 V300",
        "M866,248 V172",
    ]
    city = union(*[stroke(d, sw) for d in lines],
                 stroke(circle(866, 274, 26), sw))
    sun = circle(505, 352, 30)
    return [("primary", city), ("accent", sun)]


def skyline_compact():
    """Avatar-size version: one landmark per city, the bridge and the sun."""
    sw = 34
    lines = [
        "M190,560 H810",
        "M215,560 V452 A50,50 0 0 1 315,452 V560",   # Jerusalem stone arch
        "M315,560 C400,450 600,450 685,560",          # the bridge
        "M760,560 V336",                              # Berlin TV tower
        "M760,268 V206",
    ]
    city = union(*[stroke(d, sw) for d in lines],
                 stroke(circle(760, 302, 34), sw))
    return [("primary", city), ("accent", circle(500, 390, 44))]


CONCEPTS = [
    {
        "slug": "1-swallow-bridge",
        "title": "Swallow Bridge",
        "mark": swallow,
        "font": "Rubik-VF.ttf",
        "lines": [
            (NAME, 86, 700, 740, "text", 0.0),
            ("משלחת לגרמניה 2026", 54, 400, 820, "sub", 0.0),
            ("Dror School · Delegation to Germany 2026",
             24, 400, 880, "sub", 0.06),
        ],
    },
    {
        "slug": "2-flame-sprout",
        "title": "Flame & Sprout",
        "mark": flame,
        "font": "FrankRuhlLibre-VF.ttf",
        "lines": [
            (NAME, 88, 700, 760, "text", 0.0),
            ("משלחת לגרמניה 2026", 54, 500, 838, "sub", 0.0),
            ("זוכרים את העבר · בונים את העתיד", 32, 400, 900, "sub", 0.02,
             "Assistant-VF.ttf"),
        ],
    },
    {
        "slug": "3-dalet-d",
        "title": "ד meets D",
        "mark": dalet_d,
        "font": "Heebo-VF.ttf",
        "lines": [
            (NAME, 86, 800, 740, "text", 0.0),
            ("משלחת לגרמניה 2026", 54, 400, 818, "sub", 0.0),
            ("Delegation nach Deutschland 2026", 28, 400, 880, "sub", 0.08),
        ],
    },
    {
        "slug": "4-two-cities",
        "title": "Two Cities, One Line",
        "mark": skyline,
        "font": "Assistant-VF.ttf",
        "lines": [
            (NAME, 90, 800, 700, "text", 0.0),
            ("מסע לגרמניה 2026", 56, 400, 784, "sub", 0.0),
            ("Dror School · Delegation to Germany 2026",
             24, 600, 846, "sub", 0.06),
        ],
    },
]


def svg(view, layers, title):
    body = "\n".join(f'  <path fill="{c}" d="{d}"/>' for c, d in layers)
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view}" '
            f'role="img" aria-label="{title}">\n'
            f"  <title>{title}</title>\n{body}\n</svg>\n")


def layers_for(shapes, colors, mono):
    """Merge shapes per final color so overlaps never double-paint."""
    by_color = {}
    for role, shape in shapes:
        by_color.setdefault(colors[role], []).append(shape)
    if mono:
        assert len(by_color) == 1
    return [(c, to_d(union(*s))) for c, s in by_color.items()]


def text_shapes(concept, max_w=900):
    out = []
    for spec in concept["lines"]:
        text, size, wght, y, role, track = spec[:6]
        font = spec[6] if len(spec) > 6 else concept["font"]
        d, w = line_path(text, font, size, 500, y, wght=wght, tracking=track)
        if w > max_w:  # shrink long lines to fit the canvas
            size *= max_w / w
            d, w = line_path(text, font, size, 500, y, wght=wght,
                             tracking=track)
        out.append((role, P(d)))
    return out


def center(shapes, size=1000):
    """Translate the whole lockup so its bounding box sits mid-canvas."""
    x0, y0, x1, y1 = union(*[s for _, s in shapes]).bounds
    dx, dy = (size - (x1 - x0)) / 2 - x0, (size - (y1 - y0)) / 2 - y0
    return [(r, _P(s).transform(1, 0, 0, 1, dx, dy)) for r, s in shapes]


def avatar_box(shapes):
    """Square viewBox with room for a round crop (WhatsApp, badges)."""
    x0, y0, x1, y1 = union(*[s for _, s in shapes]).bounds
    w, h = x1 - x0, y1 - y0
    s = max(max(w, h) / 0.80, (w * w + h * h) ** 0.5 / 0.94)
    return (x0 + w / 2 - s / 2, y0 + h / 2 - s / 2, s)


def shapes(c):
    """(lockup shapes, mark shapes, mark viewBox) for one concept."""
    lockup = center(c["mark"]() + text_shapes(c))
    mark = c["mark"](heavy=True)
    return lockup, mark, avatar_box(mark)


def build():
    for c in CONCEPTS:
        outdir = ROOT / c["slug"]
        outdir.mkdir(exist_ok=True)
        lockup, mark, (x, y, s) = shapes(c)
        for vname, colors in VARIANTS.items():
            mono = vname in ("black", "white")
            title = f"{NAME} · {c['title']}"
            (outdir / f"logo-{vname}.svg").write_text(
                svg("0 0 1000 1000", layers_for(lockup, colors, mono), title),
                encoding="utf-8")
            (outdir / f"mark-{vname}.svg").write_text(
                svg(f"{x} {y} {s} {s}", layers_for(mark, colors, mono), title),
                encoding="utf-8")
        print("built", c["slug"])


if __name__ == "__main__":
    build()
