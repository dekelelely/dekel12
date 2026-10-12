"""The delegation emblem: a mission-patch seal with engraved detail.

Run: python3 src/emblem.py  ->  emblem/emblem-{color,black,white}.svg
Everything is a filled path; gaps between parts are cut, so the one-color
versions keep every separation.
"""
import math

import pathops

from build import BLUE, GOLD, INK, NAME, ROOT, flame
from geo import P, _as_path, circle, diff, mirror, rect, stroke, to_d, union
from textpath import glyph_run

C = 500                        # centre of the seal
FIELD = 342                    # radius of the picture inside the rings
DECK = 548                     # top of the bridge parapet
WATER = 736                    # waterline / springing of the arches


def inter(a, b):
    return pathops.op(pathops.simplify(_as_path(a), clockwise=True),
                      pathops.simplify(_as_path(b), clockwise=True),
                      pathops.PathOp.INTERSECTION)


def ring(r0, r1):
    return diff(circle(C, C, r1), circle(C, C, r0))


def dilate(shape, g):
    return union(shape, stroke(to_d(shape), 2 * g))


SOLID = False                  # True: draw the bird as one silhouette


def lay(acc, new, g=3.0):
    """Put `new` on top of `acc`, cutting a hairline gap where they meet."""
    if acc is None:
        return _as_path(new)
    if SOLID:
        return union(acc, new)
    return union(diff(acc, dilate(new, g)), new)


def polar(r, deg):
    a = math.radians(deg)
    return C + r * math.sin(a), C - r * math.cos(a)


# --- text on the rings ------------------------------------------------------

def arc_text(text, font, size, radius, side, wght=None, tracking=0.0,
             spread=None):
    """Set a line along a circle. side='top' reads over the top, glyphs
    pointing out; 'bottom' reads along the bottom, glyphs pointing in;
    'around' wraps the whole circle (spread = total width to fill 360°)."""
    glyphs, total = glyph_run(text, font, size, wght, tracking)
    out = []
    for path, x, adv in glyphs:
        xc = x + adv / 2
        if side == "around":
            phi = 360 * xc / (spread or total)
            rot = phi
        elif side == "top":
            phi = math.degrees((xc - total / 2) / radius)
            rot = phi
        else:
            phi = 180 - math.degrees((xc - total / 2) / radius)
            rot = phi - 180
        px, py = polar(radius, phi)
        a = math.radians(rot)
        c, s = math.cos(a), math.sin(a)
        out.append(path.transform(c, s, -s, c, px - c * xc, py - s * xc))
    return union(*out)


# --- the swallow -------------------------------------------------------------

def feather(base, tip, w, shaft=True):
    """A pointed feather from base to tip, with its shaft cut in."""
    (bx, by), (tx, ty) = base, tip
    dx, dy = tx - bx, ty - by
    L = math.hypot(dx, dy)
    ux, uy = dx / L, dy / L
    nx, ny = -uy, ux
    h = w / 2

    def pt(x, y):
        return f"{x:.2f},{y:.2f}"
    d = (f"M{pt(bx + nx * h * .55, by + ny * h * .55)} "
         f"C{pt(bx + dx * .35 + nx * h, by + dy * .35 + ny * h)} "
         f"{pt(tx - ux * L * .2 + nx * h * .85, ty - uy * L * .2 + ny * h * .85)} "
         f"{pt(tx, ty)} "
         f"C{pt(tx - ux * L * .2 - nx * h * .85, ty - uy * L * .2 - ny * h * .85)} "
         f"{pt(bx + dx * .35 - nx * h, by + dy * .35 - ny * h)} "
         f"{pt(bx - nx * h * .55, by - ny * h * .55)} Z")
    f = P(d)
    if shaft and not SOLID:
        cuts = [stroke(f"M{pt(bx + ux * L * .12, by + uy * L * .12)} "
                       f"L{pt(bx + ux * L * .78, by + uy * L * .78)}", 2.0)]
        t = 0.2
        while t < 0.8:                       # barbs, swept back from the shaft
            sx, sy = bx + ux * L * t, by + uy * L * t
            for side in (1, -1):
                ex = sx + nx * side * h * 1.3 + ux * h * 0.9
                ey = sy + ny * side * h * 1.3 + uy * h * 0.9
                cuts.append(stroke(f"M{pt(sx + nx * side * 2.5, sy + ny * side * 2.5)} "
                                   f"L{pt(ex, ey)}", 0.9, cap="butt"))
            t += 6.5 / L
        f = diff(f, *cuts)
    return f


def blades(base_a, base_b, tip_a, tip_b, w_a, w_b, n, shaft=True):
    """n feathers: bases spaced from base_a to base_b, tips from tip_a to
    tip_b, so they lie side by side like the real thing."""
    out = []
    for i in range(n):
        t = i / (n - 1)
        b = (base_a[0] + (base_b[0] - base_a[0]) * t,
             base_a[1] + (base_b[1] - base_a[1]) * t)
        p = (tip_a[0] + (tip_b[0] - tip_a[0]) * t,
             tip_a[1] + (tip_b[1] - tip_a[1]) * t)
        out.append(feather(b, p, w_a + (w_b - w_a) * t, shaft))
    return out


def left_wing():
    acc = None
    # Primaries: long blades side by side, the outermost forming the tip.
    for f in blades((318, 286), (378, 298), (206, 300), (326, 352), 17, 21, 7):
        acc = lay(acc, f, 2.4)
    # Secondaries along the trailing edge of the arm.
    for f in blades((388, 300), (462, 330), (344, 356), (448, 380), 24, 24, 6):
        acc = lay(acc, f, 2.4)
    # One row of coverts over their bases, no shafts.
    for f in blades((336, 284), (470, 322), (300, 312), (446, 352), 22, 22, 8,
                    shaft=False):
        acc = lay(acc, f, 2.2)
    # Leading edge: marginal coverts as one band from shoulder to hand.
    band = P("M486,316 C446,286 406,262 364,260 C330,259 296,272 258,296 "
             "C296,290 330,282 364,286 C404,292 444,314 480,342 Z")
    return lay(acc, band, 2.8)


def swallow():
    wing = left_wing()
    tail = None
    for f in (feather((490, 400), (436, 524), 15, False),
              feather((494, 404), (472, 466), 18),
              feather((498, 404), (488, 450), 18)):
        tail = lay(tail, f, 2.4)
    tail = union(tail, mirror(tail))
    body = union(P("M500,272 C517,272 526,290 526,312 C526,348 517,382 509,406 "
                   "C505,416 495,416 491,406 C483,382 474,348 474,312 "
                   "C474,290 483,272 500,272 Z"), circle(500, 258, 21),
                 circle(500, 284, 22))
    if not SOLID:                                 # engraved contour lines
        cuts = [stroke(f"M{482 + (y - 330) * .18:.1f},{y} Q500,{y + 7} "
                       f"{518 - (y - 330) * .18:.1f},{y}", 1.3, cap="butt")
                for y in range(326, 404, 8)]
        cuts.append(stroke("M488,250 Q494,242 504,242", 1.6))       # head light
        body = diff(body, *cuts)
    bird = lay(union(wing, mirror(wing)), tail, 3)
    bird = lay(bird, body, 3)
    throat = P("M486,286 Q500,300 514,286 Q515,304 500,312 Q485,304 486,286 Z")
    return diff(bird, dilate(throat, 2.6)), throat


# --- bridge, flame, light, sky, water ----------------------------------------

def arch_ring(cx, r, t, blocks, key_up=6):
    """Voussoirs: the ring of wedge stones over one arch, joints cut radially."""
    ring_ = diff(circle(cx, WATER, r + t), circle(cx, WATER, r))
    ring_ = inter(ring_, rect(cx - r - t - 2, WATER - r - t - key_up - 2,
                              2 * (r + t) + 4, r + t + key_up + 2))
    key = P(f"M{cx - 13},{WATER - r - t - key_up} H{cx + 13} "
            f"L{cx + 10},{WATER - r + 2} H{cx - 10} Z")
    ring_ = union(ring_, key)
    cuts = []
    for i in range(1, blocks):
        a = math.radians(180 * i / blocks)
        x0, y0 = cx - math.cos(a) * (r - 2), WATER - math.sin(a) * (r - 2)
        x1, y1 = cx - math.cos(a) * (r + t + 8), WATER - math.sin(a) * (r + t + 8)
        if abs(i - blocks / 2) < 0.6:
            continue
        cuts.append(stroke(f"M{x0:.2f},{y0:.2f} L{x1:.2f},{y1:.2f}", 2.4))
    for dx in (-13, 13):                         # keystone joints
        cuts.append(stroke(f"M{cx + dx * .85},{WATER - r + 2} "
                           f"L{cx + dx},{WATER - r - t - key_up}", 2.4))
    return diff(ring_, *cuts)


def bridge():
    arches = ARCHES
    T = 24
    coping = rect(120, DECK, 760, 14)
    rail = rect(120, DECK + 14, 760, 20)
    holes = [rect(x, DECK + 16, 7, 17, 3.5) for x in range(128, 872, 16)]
    rail = diff(rail, *holes)
    cornice = rect(120, DECK + 36, 760, 12)
    top = DECK + 48
    wall = rect(120, top, 760, WATER - top)
    openings = [circle(cx, WATER, r) for cx, r, _ in arches]
    rings = [arch_ring(cx, r, T, n) for cx, r, n in arches]
    gaps = [circle(cx, WATER, r + T + 3) for cx, r, _ in arches]
    wall = diff(wall, *gaps)
    joints = []
    course = 34
    for j, y in enumerate(range(top + course, WATER + course, course)):
        if y < WATER:
            joints.append(rect(120, y, 760, 2.4))
        for x in range(120 + (j % 2) * 48, 880, 96):
            joints.append(rect(x, y - course, 2.4, course))
    wall = diff(wall, *joints)
    return union(coping, rail, cornice, wall, *rings), union(*openings)


def soffits(arches):
    """Radial hatching just inside each arch: the vault's shadow."""
    out = []
    for cx, r, _ in arches:
        lines = []
        for k in range(1, 60):
            a = math.radians(180 * k / 60)
            x0, y0 = cx - math.cos(a) * r, WATER - math.sin(a) * r
            x1, y1 = cx - math.cos(a) * (r - 16), WATER - math.sin(a) * (r - 16)
            lines.append(stroke(f"M{x0:.2f},{y0:.2f} L{x1:.2f},{y1:.2f}", 1.1, cap="butt"))
        out.append(diff(union(*lines), circle(cx, WATER, r - 17)))
    return union(*out)


def skylines():
    """Jerusalem on the left bank, Berlin on the right, as silhouettes."""
    b = DECK
    jer = [
        rect(176, b - 22, 136, 22),                       # city wall
        *[rect(x, b - 30, 7, 9) for x in range(178, 306, 12)],  # crenels
        rect(206, b - 74, 18, 52), rect(203, b - 82, 24, 6),     # Tower of David
        circle(215, b - 86, 9), rect(214, b - 104, 2.4, 12),
        rect(256, b - 44, 38, 22),                        # the Hurva dome
        P(f"M256,{b - 44} A19,19 0 0 1 294,{b - 44} Z"),
        rect(270, b - 70, 10, 9), rect(274.2, b - 80, 1.6, 10),
        P(f"M242,{b - 22} C236,{b - 40} 240,{b - 60} 244,{b - 66} "
          f"C248,{b - 60} 252,{b - 40} 246,{b - 22} Z"),  # cypresses
        P(f"M300,{b - 22} C295,{b - 36} 298,{b - 50} 302,{b - 54} "
          f"C306,{b - 50} 309,{b - 36} 304,{b - 22} Z"),
    ]
    jer_cuts = [rect(x, b - 15, 4, 8, 2) for x in range(186, 306, 14)] + \
               [rect(212, b - 64, 6, 10, 3)]
    ber = [
        rect(690, b - 30, 76, 30),                        # Reichstag
        rect(686, b - 36, 84, 6),
        P(f"M708,{b - 36} A20,20 0 0 1 748,{b - 36} Z"),  # glass dome
        rect(772, b - 40, 22, 40), rect(796, b - 26, 18, 26),  # new blocks
        rect(803, b - 120, 5, 120),                       # TV tower
        circle(805.5, b - 128, 11), rect(804.6, b - 176, 1.8, 40),
    ]
    ber_cuts = [stroke(f"M{x},{b - 36} L728,{b - 55}", 1, cap="butt")
                for x in (712, 720, 736, 744)] + \
               [rect(x, b - 24, 4, 16) for x in range(696, 764, 9)] + \
               [rect(776, y, 14, 2.2) for y in range(b - 34, b - 4, 6)]
    return union(diff(union(*jer), *jer_cuts), diff(union(*ber), *ber_cuts))


def guilloche(keep_out):
    """Banknote-style interlaced waves for the name band, each line clipped
    on its own (the four together trip the boolean engine)."""
    lines = []
    for k in range(4):
        ph = k * math.pi / 2
        pts = []
        for i in range(0, 2160):
            th = 2 * math.pi * i / 2160
            r = 432 + 38 * math.sin(36 * th + ph)
            pts.append(f"{C + r * math.sin(th):.2f},{C - r * math.cos(th):.2f}")
        line = stroke("M" + " L".join(pts) + " Z", 1.3, cap="butt")
        lines.append(diff(inter(line, ring(388, 477)), keep_out))
    return union(*lines)


def tiny_swallow(cx, cy, k=0.16):
    global SOLID
    SOLID = True
    bird, throat = swallow()
    SOLID = False
    bird = union(bird, throat)
    return bird.transform(k, 0, 0, k, cx - 500 * k, cy - 360 * k)


def memorial_flame():
    """Concept 2's flame and sprout, small, standing on the parapet."""
    shapes = flame()
    f = shapes[0][1]
    s = 0.17
    return _as_path(f).transform(s, 0, 0, s, C - 500 * s, DECK - 4 - 584 * s)


def sky(exclude):
    lines = []
    y = 160.0
    while y < WATER:
        k = max(0.0, (DECK - y) / (DECK - 160))
        w = 0.7 + 2.1 * k ** 1.3 if y < DECK else 0.8
        lines.append(rect(120, y - w / 2, 760, w))
        y += 7.5
    return diff(union(*lines), *exclude)


def light(exclude):
    rings = [ring(r - 0.8, r + 0.8) for r in range(58, 184, 11)]
    halo = inter(union(*rings), rect(0, 0, 1000, DECK - 6))
    return diff(halo, *exclude)


ARCHES = [(500, 104, 13), (278, 50, 9), (722, 50, 9)]


def water():
    """Wave lines, heavier under the piers where the bridge is reflected."""
    def under_arch(x):
        return any(abs(x - cx) < r - 6 for cx, r, _ in ARCHES)
    lines = []
    for j, y in enumerate(range(WATER + 8, C + FIELD, 8)):
        fade = (y - WATER) / (C + FIELD - WATER)
        x = 120
        while x < 880:
            open_ = under_arch(x)
            x1 = x
            while x1 < 880 and under_arch(x1) == open_:
                x1 += 4
            w = (1.9 - 0.9 * fade) * (1 if open_ else 2.1 - 1.1 * fade)
            pts = " L".join(f"{xx},{y + 1.6 * math.sin((xx + j * 37) / 23):.2f}"
                            for xx in range(x, x1 + 1, 4))
            lines.append(stroke("M" + pts, w, cap="butt"))
            x = x1
    return union(*lines)


# --- the seal ---------------------------------------------------------------

def emblem_small():
    """For 24–96px uses: the same seal without hairline detail."""
    global SOLID
    field = circle(C, C, FIELD + 40)
    SOLID = True
    bird, throat = swallow()
    SOLID = False
    k = 1.12
    bird = bird.transform(k, 0, 0, k, 500 - 500 * k, 300 - 330 * k)
    throat = throat.transform(k, 0, 0, k, 500 - 500 * k, 300 - 330 * k)
    deck = union(rect(120, DECK, 760, 48),
                 diff(rect(120, DECK + 48, 760, WATER - DECK - 48),
                      circle(500, WATER, 112), circle(278, WATER, 58),
                      circle(722, WATER, 58)))
    fl = memorial_flame()
    wave = union(*[rect(120, y, 760, 9) for y in (WATER + 22, WATER + 50)])
    blue = union(inter(union(deck, wave), field), bird)
    gold = union(diff(fl, dilate(bird, 6)), throat, ring(452, 492))
    return [("paper", P(circle(C, C, 492))), ("ink", ring(FIELD + 40, 446)),
            ("blue", blue), ("gold", gold)]


def emblem():
    field = circle(C, C, FIELD)
    bird, throat = swallow()
    deck, openings = bridge()
    fl = memorial_flame()
    bird_halo = dilate(bird, 9)
    city = skylines()
    blue = union(
        inter(sky([bird_halo, inter(circle(C, C, 186), rect(0, 0, 1000, DECK)),
                   dilate(deck, 3), dilate(city, 4),
                   dilate(fl, 6)]), field),
        inter(light([bird_halo, dilate(fl, 7)]), field),
        inter(deck, field),
        inter(diff(soffits(ARCHES), dilate(deck, 2)), field),
        inter(city, field),
        inter(water(), field),
        bird,
        ring(FIELD, FIELD + 4),
    )
    gold = union(diff(fl, dilate(bird, 4)), throat)

    top = arc_text(NAME, "Rubik-VF.ttf", 66, 412, "top", wght=700,
                   tracking=0.03)
    bottom = arc_text("משלחת לגרמניה 2026", "Rubik-VF.ttf", 56, 458,
                      "bottom", wght=500, tracking=0.02)
    diamonds = [tiny_swallow(x, C) for x in (C - 432, C + 432)]
    text_halo = dilate(union(top, bottom), 5)
    marks = [dilate(d, 5) for d in diamonds]
    band = diff(ring(384, 481), top, bottom, *marks)
    lace = guilloche(union(text_halo, *marks))

    micro_text = "DROR SCHOOL · LEV HASHARON · ISRAEL · DELEGATION TO GERMANY 2026 · "
    _, one = glyph_run(micro_text, "Assistant-VF.ttf", 15, 700, 0.14)
    reps = max(1, int(2 * math.pi * 356 / one))
    micro = arc_text(micro_text * reps, "Assistant-VF.ttf", 15, 356, "around",
                     wght=700, tracking=0.14, spread=one * reps)
    blue = union(blue, micro)

    gold = union(gold, ring(485, 492), ring(377, 381), *diamonds)
    return [("paper", P(circle(C, C, 492))), ("ink", diff(band, lace)),
            ("lace", lace), ("blue", blue), ("gold", gold)]


VARIANTS = {
    "color": {"paper": "#FFFFFF", "ink": INK, "lace": "#24508F", "blue": BLUE,
              "gold": GOLD},
    "black": {"ink": "#000000", "lace": "#000000", "blue": "#000000", "gold": "#000000"},
    "white": {"ink": "#FFFFFF", "lace": "#FFFFFF", "blue": "#FFFFFF", "gold": "#FFFFFF"},
}


def main():
    out = ROOT / "emblem"
    out.mkdir(exist_ok=True)
    for prefix, layers in (("emblem", emblem()), ("emblem-small", emblem_small())):
        write(out, prefix, layers)


def write(out, prefix, layers):
    for name, colors in VARIANTS.items():
        by = {}
        for role, shape in layers:
            if role in colors:
                by.setdefault(colors[role], []).append(shape)
        paths = "\n".join(f'  <path fill="{c}" d="{to_d(union(*s))}"/>'
                          for c, s in by.items())
        (out / f"{prefix}-{name}.svg").write_text(
            '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 1000" '
            f'role="img" aria-label="{NAME} · משלחת לגרמניה 2026">\n'
            f"  <title>{NAME} · משלחת לגרמניה 2026</title>\n{paths}\n</svg>\n",
            encoding="utf-8")
        print("wrote", prefix, name)


if __name__ == "__main__":
    main()
