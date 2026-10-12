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
    shape = pathops.simplify(_as_path(shape), clockwise=True)
    # Skia's boolean engine can trip on a near-degenerate offset; a hair
    # wider or narrower always goes through.
    for k in (1, 1.013, 0.987, 1.031):
        try:
            return union(shape, stroke(to_d(shape), 2 * g * k))
        except pathops.PathOpsError:
            continue
    raise


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


def bezier(p0, p1, p2, p3, t):
    u = 1 - t
    return (u**3 * p0[0] + 3 * u * u * t * p1[0] + 3 * u * t * t * p2[0] + t**3 * p3[0],
            u**3 * p0[1] + 3 * u * u * t * p1[1] + 3 * u * t * t * p2[1] + t**3 * p3[1])


def poly(points):
    return P("M" + " L".join(f"{x:.2f},{y:.2f}" for x, y in points) + " Z")


def taper(p0, p1, w0, w1):
    """A straight line that thins from w0 to w1: one engraver's stroke."""
    dx, dy = p1[0] - p0[0], p1[1] - p0[1]
    L = math.hypot(dx, dy) or 1
    nx, ny = -dy / L, dx / L
    return poly([(p0[0] + nx * w0 / 2, p0[1] + ny * w0 / 2),
                 (p1[0] + nx * w1 / 2, p1[1] + ny * w1 / 2),
                 (p1[0] - nx * w1 / 2, p1[1] - ny * w1 / 2),
                 (p0[0] - nx * w0 / 2, p0[1] - ny * w0 / 2)])


def flight_feather(base, tip, w, spot=None):
    """A dark flight feather engraved in line: pale tapering strokes along
    the vane, lighter at the base, closing up toward the tip, and a pale
    shaft, as the underside of a swallow's wing looks."""
    f = feather(base, tip, w, shaft=False)
    if SOLID:
        return f
    (bx, by), (tx, ty) = base, tip
    L = math.hypot(tx - bx, ty - by)
    ux, uy = (tx - bx) / L, (ty - by) / L
    nx, ny = -uy, ux
    cuts = []
    k = -w / 2 + 2.2
    while k < w / 2 - 1.2:
        p0 = (bx + ux * L * .06 + nx * k * .7, by + uy * L * .06 + ny * k * .7)
        p1 = (bx + ux * L * .9 + nx * k * .25, by + uy * L * .9 + ny * k * .25)
        cuts.append(taper(p0, p1, 1.5, 0.15))
        k += 2.7
    cuts.append(taper((bx + ux * L * .04, by + uy * L * .04),
                      (bx + ux * L * .8, by + uy * L * .8), 1.6, 0.4))
    if spot:
        sx, sy = bx + ux * L * spot, by + uy * L * spot
        cuts.append(P(f"M{sx - 3.2},{sy} A3.2,5 0 1 0 {sx + 3.2},{sy} "
                      f"A3.2,5 0 1 0 {sx - 3.2},{sy} Z"))
    return diff(f, *cuts)


# Left wing outline, seen from below: leading edge shoulder -> wrist -> tip,
# then the trailing edge back to the body. Long, narrow, swept at the wrist.
LEAD = [((478, 300), (442, 282), (406, 272), (372, 276)),
        ((372, 276), (330, 282), (270, 308), (214, 350))]
TRAIL = [((214, 350), (256, 347), (296, 354), (328, 366)),
         ((328, 366), (372, 382), (432, 372), (482, 356))]


def along(segs, t):
    seg = segs[0] if t < 0.5 else segs[1]
    return bezier(*seg, t * 2 if t < 0.5 else t * 2 - 1)


def wing_outline():
    pts = [along(LEAD, i / 40) for i in range(41)]
    pts += [along(TRAIL, i / 40) for i in range(1, 41)]
    return poly(pts)


def left_wing():
    """Returns (dark parts, pale-lining lines, silhouette)."""
    outline = wing_outline()
    acc = None
    # Nine primaries fanning from the hand to the pointed tip.
    for i in range(9):
        t = i / 8
        base = (334 + 38 * t, 290 + 2 * t)
        tip = along(TRAIL, 0.5 * t ** 0.9)
        acc = lay(acc, flight_feather(base, tip, 14 + 6 * t), 1.4)
    # Eight secondaries along the arm, tips on the trailing edge.
    for i in range(8):
        t = i / 7
        base = (380 + 96 * t, 294 + 22 * t)
        tip = along(TRAIL, 0.53 + 0.45 * t)
        acc = lay(acc, flight_feather(base, tip, 20), 1.4)
    acc = inter(acc, dilate(outline, 1))
    # The underwing lining: pale coverts between the leading edge and the
    # flight feathers, drawn as outlines and small scallops only.
    depth = lambda f: 2 + 32 * (1 - f) ** 0.7
    top = [along(LEAD, t / 30 * .86) for t in range(31)]
    bottom = [(x, y + depth(t / 30)) for t, (x, y) in zip(range(31), top)]
    lining = poly(top + list(reversed(bottom)))
    if SOLID:
        return union(acc, lining), None, union(outline, lining)
    acc = diff(acc, dilate(lining, 1.6))
    lines = [stroke(to_d(lining), 1.5)]
    for row, frac in enumerate((.42, .78)):
        for j in range(3 + row, 30, 2):
            (x0, y0), (x1, y1) = top[j], bottom[j]
            cx, cy = x0 + (x1 - x0) * frac, y0 + (y1 - y0) * frac
            r = 4.5 + 2 * (1 - j / 30)
            lines.append(stroke(f"M{cx - r:.1f},{cy - 2:.1f} Q{cx:.1f},{cy + r:.1f} "
                                f"{cx + r:.1f},{cy - 2:.1f}", 1.0, cap="butt"))
    lead_line = stroke("M" + " L".join(f"{x:.1f},{y:.1f}" for x, y in top), 2.6)
    return acc, union(*lines, lead_line), union(outline, lining)


def swallow():
    """A barn swallow seen from below, engraved. Returns
    (blue parts, gold parts, silhouette)."""
    wing, wing_lines, wing_sil = left_wing()
    # Tail: the long outer streamer and four inner feathers, each with the
    # white spot of a barn swallow's tail.
    tail = None
    for base, tip, w, spot in (((490, 390), (444, 526), 12, None),
                               ((492, 394), (462, 460), 15, .74),
                               ((494, 396), (471, 448), 15, .72),
                               ((496, 397), (481, 438), 14, .7),
                               ((498, 398), (491, 432), 13, .68)):
        tail = lay(tail, flight_feather(base, tip, w, spot), 1.4)
    tail = union(tail, mirror(tail))

    # An eagle's head on a swallow's body. The head and neck are white,
    # engraved in fine feather strokes, and end in a ragged fringe over the
    # dark breast band, as on a bald eagle; below that the body is the
    # swallow's.
    import random
    rnd = random.Random(11)
    # Ragged lower edge: feather tips of uneven length, notches between.
    fringe, x = [], 538.0
    while x > 462:
        step = rnd.uniform(2.6, 4.2)
        fringe.append((x - step * .5, 300 + rnd.uniform(5, 15)))      # tip
        fringe.append((x - step, 298 + rnd.uniform(-2, 2.5)))           # notch
        x -= step
    R_ = lambda pts: [(1000 - px, py) for px, py in reversed(pts)]
    right = ([bezier((500, 203), (521, 203), (533, 213), (535, 229), k / 10) for k in range(1, 11)]
             + [bezier((535, 229), (538, 243), (541, 256), (538, 266), k / 8) for k in range(1, 9)]
             + [bezier((538, 266), (535, 275), (533, 283), (538, 298), k / 8) for k in range(1, 9)])
    hood_pts = [(500, 203)] + right + fringe + R_(right)[1:]
    hood0 = poly(hood_pts)
    # The head is drawn at full size, then scaled down about the neck so
    # it sits in proportion on the swallow's body.
    K = 0.76
    DY = 12      # sit the head low, so the wings meet the neck
    T = lambda p: _as_path(p).transform(K, 0, 0, K, 500 - 500 * K, 300 - 300 * K + DY)
    hood = T(hood0)
    torso = P("M526.5,290 C531,306 530,322 526,342 C522,362 514,380 507,400 L493,400 "
              "C486,380 478,362 474,342 C470,322 469,306 473.5,290 Z")
    body = union(hood, torso)
    sil = union(union(wing_sil, mirror(wing_sil)), tail, body)

    # The hooked beak, seen head-on, with the cere at its base and the
    # fleshy gape running back on each side; gold eyes under a heavy brow.
    beak_g = P("M490.5,238.5 C490.5,232.5 509.5,232.5 509.5,238.5 "
               "C510.5,249 506.5,260 500,271 C493.5,260 489.5,249 490.5,238.5 Z")
    gapes = [P(f"M{500 + s_ * 9.6:.1f},246 Q{500 + s_ * 15:.1f},246.6 {500 + s_ * 19:.1f},250.2 "
               f"Q{500 + s_ * 14.6:.1f},249.6 {500 + s_ * 9.2:.1f},250.4 Z") for s_ in (-1, 1)]
    eyes = [(483.4, 236.4), (516.6, 236.4)]
    R = 4.8
    irises, brows, lids = [], [], []
    for x, y in eyes:
        side = -1 if x < 500 else 1
        bx0, by0 = x + side * 8.5, y - 5.6
        bx1, by1 = x - side * 7.5, y - 1.0
        lid = poly([(bx0, by0), (bx1, by1), (bx1, y - 14), (bx0, y - 14)])
        lids.append(lid)
        irises.append(diff(circle(x, y, R), lid, circle(x, y + 0.4, 2.8)))
        brows.append(diff(poly([(bx0 + side * 1.5, by0 + 0.4), (bx1 - side * 0.5, by1 + 0.3),
                                (bx1 - side * 1.5, by1 - 4.2), (bx0 + side * 2, by0 - 3.6)]),
                          *[stroke(f"M{bx0 + side * k:.1f},{by0 - 3:.1f} L{bx0 + side * (k - 3):.1f},"
                                   f"{by0 + 0.2:.1f}", 0.45, cap="butt")
                            for k in range(-2, 16, 3)]))
    face_gold = union(beak_g, *gapes, *irises)
    if SOLID:
        return diff(sil, dilate(T(face_gold), 2)), T(face_gold), sil

    def outline(shape, w0=0.7, w1=1.9):
        return diff(dilate(shape, w1), dilate(shape, w0))

    ink = [outline(beak_g)]
    ink += [outline(g, 0.6, 1.5) for g in gapes]
    for (x, y), iris in zip(eyes, irises):
        ink.append(outline(diff(circle(x, y, R), lids[eyes.index((x, y))]), 0.5, 1.4))
        ink.append(diff(circle(x, y + 0.4, 2.2), circle(x - 1.1, y - 0.6, 0.85)))
    ink += brows
    # Cere line, nostrils and the ridge of the bill, cut into the gold.
    face_gold = diff(face_gold,
                     inter(stroke("M490.6,242.5 Q500,239.5 509.4,242.5", 0.9, cap="butt"),
                           beak_g),
                     *[P(f"M{x - 1.5},244.6 Q{x},242.8 {x + 1.5},244.6 Q{x},246.2 {x - 1.5},244.6 Z")
                       for x in (495.4, 504.6)],
                     taper((500, 246), (500, 266), 1.0, 0.2))
    # Feathers: fine strokes falling from the crown and down the neck, set
    # closer toward the sides, where the head turns away.
    keep_out = union(dilate(beak_g, 3), *[dilate(g, 2.5) for g in gapes],
                     *[circle(x, y, R + 3.2) for x, y in eyes], *[dilate(b_, 1) for b_ in brows])
    strokes_ = []
    for _ in range(520):
        x, y = rnd.uniform(462, 538), rnd.uniform(203, 306)
        edge = abs(x - 500) / 38
        if rnd.random() > 0.18 + 0.82 * edge ** 1.4:
            continue
        ang = math.pi / 2 + (x - 500) * 0.022 + rnd.uniform(-0.18, 0.18)
        if y < 232:
            ang = math.atan2(y - 236, x - 500) * 0.35 + math.pi / 2 * 0.65
        L = rnd.uniform(2.4, 5.2)
        strokes_.append(taper((x, y), (x + math.cos(ang) * L, y + math.sin(ang) * L),
                              rnd.uniform(0.35, 0.6), 0.08))
    hood_in = diff(hood0, stroke(to_d(hood0), 3))
    crown = T(union(*ink, diff(inter(union(*strokes_), hood_in), keep_out)))
    face_gold = T(face_gold)

    # The breast band under the fringe, and the pale belly below it.
    # The dark breast feathers end in their own ragged edge, so the white
    # hood, the dark band and the pale belly read as one plumage.
    low, x = [], 536.0
    while x > 464:
        step = rnd.uniform(2.8, 4.4)
        low.append((x - step * .5, 338 + rnd.uniform(3, 10)))
        low.append((x - step, 336 + rnd.uniform(-2, 2)))
        x -= step
    band_shape = poly([(464, 286), (536, 286), (538, 336)] + low + [(462, 336)])
    band = diff(inter(torso, band_shape), dilate(hood, 1.0),
                *[taper((x_, 318 + rnd.uniform(-3, 3)), (x_ + (x_ - 500) * .04, 344), 0.7, 0.15)
                  for x_ in [rnd.uniform(468, 532) for _ in range(26)]])
    belly_lines = [stroke(to_d(body), 1.8)]          # one contour, head to tail
    flanks = diff(torso, diff(torso, stroke(to_d(torso), 26)))
    for y in range(344, 398, 5):
        arc = stroke(f"M466,{y} Q500,{y + 9} 534,{y}", 0.85, cap="butt")
        belly_lines.append(inter(arc, flanks))
    for j, y in enumerate(range(350, 372, 7)):        # chest feather tips
        for x in range(486 + (j % 2) * 4, 516, 8):
            belly_lines.append(inter(stroke(f"M{x - 3},{y} Q{x},{y + 3.2} {x + 3},{y}",
                                            0.6, cap="butt"), torso))
    for y in range(384, 400, 5):                     # undertail coverts
        belly_lines.append(inter(stroke(f"M488,{y} Q500,{y + 6} 512,{y}", 0.9,
                                        cap="butt"), torso))
    bird = union(wing, mirror(wing), wing_lines, mirror(wing_lines))
    bird = lay(bird, tail, 1.6)
    bird = diff(bird, dilate(body, 2.4))
    bird = union(bird, crown, band, *belly_lines)
    bird = diff(bird, dilate(face_gold, 0.4))
    return bird, face_gold, sil


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
    """Left bank: the Old City wall with the Dome of the Rock behind it.
    Right bank: the Witten synagogue (1885, destroyed 1938): a two-colour
    brick hall in round-arch style with a central dome on a drum.
    Returns (blue silhouettes, gold dome)."""
    b = DECK
    # Jerusalem ----------------------------------------------------------
    wall = union(rect(170, b - 16, 148, 16),
                 *[rect(x, b - 23, 7, 8) for x in range(172, 314, 12)])
    wall = diff(wall, *[rect(x, b - 11, 3, 6, 1.5) for x in range(180, 314, 12)])
    cx = 236
    octa = poly([(cx - 46, b - 16), (cx - 46, b - 44), (cx - 30, b - 50),
                 (cx + 30, b - 50), (cx + 46, b - 44), (cx + 46, b - 16)])
    octa_cuts = [rect(cx - 47, b - 47, 94, 1.2)]            # parapet line
    for x in range(cx - 26, cx + 27, 9):                     # arched windows
        octa_cuts.append(P(f"M{x - 2.4},{b - 22} V{b - 32} A2.4,2.4 0 0 1 {x + 2.4},{b - 32} "
                           f"V{b - 22} Z"))
    for x in (cx - 39, cx + 39):                             # side faces
        octa_cuts.append(P(f"M{x - 1.8},{b - 22} V{b - 30} A1.8,1.8 0 0 1 {x + 1.8},{b - 30} "
                           f"V{b - 22} Z"))
    for x in (cx - 30, cx + 30):                             # corner edges
        octa_cuts.append(rect(x - 0.5, b - 46, 1, 30))
    octa_cuts += [rect(cx - 46, y, 92, 0.8) for y in (b - 38, b - 36)]  # tile band
    octa = diff(octa, *octa_cuts)
    drum = diff(rect(cx - 22, b - 64, 44, 14),
                *[rect(x, b - 61, 2.4, 7, 1.2) for x in range(cx - 18, cx + 18, 6)])
    gold = P(f"M{cx - 25},{b - 64} C{cx - 27},{b - 84} {cx - 14},{b - 100} {cx},{b - 104} "
             f"C{cx + 14},{b - 100} {cx + 27},{b - 84} {cx + 25},{b - 64} Z")
    # Engraved light on the gold: meridians that wrap the curve of the dome.
    gold = diff(gold, *[stroke(f"M{cx + o},{b - 64} Q{cx + o * .8},{b - 92} {cx},{b - 103}",
                               0.8, cap="butt") for o in (-16, -8, 8, 16)],
                stroke(f"M{cx - 25},{b - 70} Q{cx},{b - 66} {cx + 25},{b - 70}", 0.8,
                       cap="butt"))
    spire = union(rect(cx - 0.9, b - 118, 1.8, 15), circle(cx, b - 108, 2.2))
    trees = [P(f"M{x - 4},{b - 16} C{x - 9},{b - 34} {x - 5},{b - 58} {x},{b - 66} "
               f"C{x + 5},{b - 58} {x + 9},{b - 34} {x + 4},{b - 16} Z")
             for x in (182, 296)]
    jer = union(wall, octa, drum, spire, *trees)

    # Witten -------------------------------------------------------------
    w0, w1, wc = 682, 826, 754
    hall = rect(w0, b - 50, w1 - w0, 50)
    gable = poly([(wc - 30, b - 50), (wc - 30, b - 66), (wc, b - 80),
                  (wc + 30, b - 66), (wc + 30, b - 50)])
    corners = [rect(x, b - 62, 10, 62) for x in (w0, w1 - 10)]
    caps = [P(f"M{x},{b - 62} L{x + 5},{b - 70} L{x + 10},{b - 62} Z") for x in (w0, w1 - 10)]
    drum = rect(wc - 17, b - 96, 34, 16)
    dome = P(f"M{wc - 20},{b - 96} C{wc - 22},{b - 112} {wc - 10},{b - 124} {wc},{b - 126} "
             f"C{wc + 10},{b - 124} {wc + 22},{b - 112} {wc + 20},{b - 96} Z")
    lantern = union(rect(wc - 3.5, b - 136, 7, 11), P(f"M{wc - 4.5},{b - 136} "
                                                   f"Q{wc},{b - 144} {wc + 4.5},{b - 136} Z"),
                    rect(wc - 0.7, b - 152, 1.4, 10))
    tablets = union(*[P(f"M{x},{b - 80} V{b - 87} A3,3 0 0 1 {x + 6},{b - 87} V{b - 80} Z")
                      for x in (wc - 6.6, wc + 0.6)])
    syn = union(hall, gable, *corners, *caps, drum, dome, lantern)
    cuts = []
    for y in range(int(b - 46), int(b - 4), 6):              # two-colour brick bands
        cuts.append(rect(w0 + 11, y, w1 - w0 - 22, 1.1))
    for x in list(range(w0 + 18, wc - 34, 13)) + list(range(wc + 38, w1 - 14, 13)):
        cuts.append(P(f"M{x - 3},{b - 12} V{b - 34} A3,3 0 0 1 {x + 3},{b - 34} V{b - 12} Z"))
    cuts.append(P(f"M{wc - 8},{b} V{b - 18} A9,9 0 1 1 {wc + 8},{b - 18} V{b} Z"))  # horseshoe door
    cuts.append(circle(wc, b - 62, 7.5))                     # rose window
    for x in range(wc - 12, wc + 13, 6):                     # drum windows
        cuts.append(P(f"M{x - 1.6},{b - 83} V{b - 91} A1.6,1.6 0 0 1 {x + 1.6},{b - 91} "
                      f"V{b - 83} Z"))
    for o in (-11, 0, 11):                                   # dome ribs
        cuts.append(stroke(f"M{wc + o},{b - 96} Q{wc + o * .7},{b - 116} {wc},{b - 125}",
                           0.9, cap="butt"))
    syn = diff(syn, *cuts)
    # Rose window with a Star of David in its tracery.
    r = 5.6
    tri = lambda a0: [(wc + r * math.sin(math.radians(a0 + k * 120)),
                       b - 62 - r * math.cos(math.radians(a0 + k * 120))) for k in range(3)]
    star = union(stroke("M" + " L".join(f"{x:.2f},{y:.2f}" for x, y in tri(0)) + " Z", 1.1),
                 stroke("M" + " L".join(f"{x:.2f},{y:.2f}" for x, y in tri(60)) + " Z", 1.1))
    syn = union(syn, inter(star, circle(wc, b - 62, 7)))
    return union(jer, syn), gold


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
    bird, throat, _ = swallow()
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
    bird, throat, _ = swallow()
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
    bird, throat, sil = swallow()
    deck, openings = bridge()
    fl = memorial_flame()
    bird_halo = dilate(sil, 9)
    city, gold_dome = skylines()
    blue = union(
        inter(sky([bird_halo, inter(circle(C, C, 186), rect(0, 0, 1000, DECK)),
                   dilate(deck, 3), dilate(union(city, gold_dome), 4),
                   dilate(fl, 6)]), field),
        inter(light([bird_halo, dilate(fl, 7)]), field),
        inter(deck, field),
        inter(diff(soffits(ARCHES), dilate(deck, 2)), field),
        inter(city, field),
        inter(water(), field),
        bird,
        ring(FIELD, FIELD + 4),
    )
    gold = union(diff(fl, dilate(sil, 4)), throat, inter(gold_dome, field))

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
    return [("paper", P(circle(C, C, 492))), ("ink", band),
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
