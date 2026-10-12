"""The delegation emblem: a mission-patch seal with engraved detail.

Run: python3 src/emblem.py  ->  emblem/emblem-{color,black,white}.svg
Everything is a filled path; gaps between parts are cut, so the one-color
versions keep every separation.
"""
import math
from pathlib import Path

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
MARKER = False                 # True: the ring markers' swallow silhouette


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


# Left wing outline, seen from below, after a photograph of a barn swallow
# with its wings fully spread: held out flat and broad, the tip just raised.
LEAD = [((478, 298), (440, 286), (390, 280), (350, 280)),
        ((350, 280), (318, 280), (290, 282), (270, 288))]
TRAIL = [((270, 288), (276, 304), (292, 330), (318, 350)),
         ((318, 350), (360, 376), (430, 384), (482, 370))]


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
    # Nine primaries fanning from the hand to the tip.
    for i in range(9):
        t = i / 8
        base = (360 + 36 * t, 288 + 10 * t)
        tip = along(TRAIL, 0.5 * t ** 0.9)
        acc = lay(acc, flight_feather(base, tip, 14 + 5 * t), 1.4)
    # Nine secondaries along the arm, tips on the trailing edge.
    for i in range(9):
        t = i / 8
        base = (400 + 76 * t, 300 + 18 * t)
        tip = along(TRAIL, 0.53 + 0.45 * t)
        acc = lay(acc, flight_feather(base, tip, 19), 1.4)
    acc = inter(acc, dilate(outline, 1))
    # The underwing lining: pale coverts along the leading edge, drawn as
    # outlines and small scallops only.
    depth = lambda f: 2 + 30 * (1 - f) ** 0.7
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
    """A barn swallow seen from below with its face turned to the viewer,
    after a photograph: wings spread wide, blue crown, chestnut forehead and
    throat, pale chest, cinnamon belly and long streamers. Returns
    (blue parts, gold parts, silhouette)."""
    import random
    rnd = random.Random(5)
    wing, wing_lines, wing_sil = left_wing()

    # Tail: thin streamers, almost parallel, and a shallow inner fork whose
    # feathers carry the white spots.
    tail = None
    for base, tip, w, spot in (((493, 420), (465, 522), 8, None),
                               ((494, 423), (482, 468), 11, .72),
                               ((496, 425), (486, 462), 11, .7),
                               ((497, 426), (490, 457), 10, .68),
                               ((499, 427), (495, 452), 9, .66)):
        tail = lay(tail, flight_feather(base, tip, w, spot), 1.4)
    tail = union(tail, mirror(tail))

    # The head is a low dome, widest at the base where it sits on the wings.
    # Its lower rim, under the chestnut throat, is the dark breast band.
    head = P("M500,231 C517,231 528,241 531,255 C533,265 533,278 530,288 "
             "C522,295 511,298.5 500,298.5 C489,298.5 478,295 470,288 "
             "C467,278 467,265 469,255 C472,241 483,231 500,231 Z")
    torso = P("M522,286 C531,300 532,320 528,342 C524,370 514,400 507,428 "
              "Q503,440 500,446 Q497,440 493,428 C486,400 476,370 472,342 "
              "C468,320 469,300 478,286 Z")
    body = union(head, torso)
    sil = union(union(wing_sil, mirror(wing_sil)), tail, body)

    # As in the photograph: a chestnut shield on the forehead narrowing to
    # the bill, a short wide bill with the gape spread to the sides, and a
    # broad chestnut throat below it.
    beak = P("M492,263.6 L496.6,260.8 L503.4,260.8 L508,263.6 "
             "Q503.2,264.6 500,270.2 Q496.8,264.6 492,263.6 Z")
    forehead = diff(P("M487.6,247.8 Q500,245.6 512.4,247.8 C511,252 508.6,256.6 505.6,260.6 "
                      "L494.4,260.6 C491.4,256.6 489,252 487.6,247.8 Z"), dilate(beak, 1.3))
    throat = diff(inter(body, P("M492,263.6 C487,264.6 481.5,265.2 477.5,268.2 "
                                "C474.5,273.5 476.8,280.6 483.5,284.4 C490,287.6 510,287.6 516.5,284.4 "
                                "C523.2,280.6 525.5,273.5 522.5,268.2 C518.5,265.2 513,264.6 508,263.6 "
                                "L500,270.2 Z")), dilate(beak, 1.3))
    notches = []
    for k in range(22):
        a = math.radians(-8 + k * 196 / 21)
        bx, by = 500 + 24.5 * math.cos(a), 276.4 + 11.6 * math.sin(a)
        nx, ny = math.cos(a) / 24.5, math.sin(a) / 11.6
        n = math.hypot(nx, ny); nx, ny = nx / n, ny / n
        d_ = rnd.uniform(2.6, 4.2)
        notches.append(poly([(bx + nx * 2 - ny * 1.3, by + ny * 2 + nx * 1.3),
                             (bx - nx * d_, by - ny * d_),
                             (bx + nx * 2 + ny * 1.3, by + ny * 2 - nx * 1.3)]))
    throat = diff(throat, *notches)
    # The cinnamon lower belly begins in a ragged line of feather tips.
    edge, x = [], 550.0
    while x > 450:
        step = rnd.uniform(3.2, 4.8)
        arc = 6 * (1 - ((x - 500) / 30) ** 2) if abs(x - 500) < 30 else 0
        edge.append((x - step * .5, 382 + rnd.uniform(4, 9) - arc))
        edge.append((x - step, 380 + rnd.uniform(-1.5, 1.5) - arc))
        x -= step
    belly_gold = inter(torso, poly([(550, 470), (550, 380)] + edge + [(450, 380), (450, 470)]))
    if SOLID:
        face = union(throat, forehead)
        return diff(sil, dilate(face, 2)), face, sil

    # The face: dark masks run from the forehead shield out through the eyes;
    # each eye is marked only by a lower lid and a catchlight in the same
    # place in both. The crown above is lighter, cut with fine feather ticks.
    eyes = [(484.2, 254.2), (515.8, 254.2)]
    mask = P("M488.4,248.2 C484.5,247 478.5,247.8 473.6,250.6 C469.8,253.4 469,258 470,262.4 "
             "C475.5,260.4 481.5,260.8 486.5,262.4 C489,263.2 490.8,263.6 492,263.6 "
             "C493,259.5 491,253.5 488.4,248.2 Z")
    mask = union(mask, mirror(mask))
    ticks = []
    for gy in range(0, 34):
        for gx in range(0, 32):
            x = 468 + gx * 2.1 + (gy % 2) * 1.05 + rnd.uniform(-.5, .5)
            y = 232 + gy * 1.9 + rnd.uniform(-.4, .4)
            ang = math.atan2(y - 266, x - 500)
            L = rnd.uniform(1.1, 1.7)
            ticks.append(taper((x, y), (x + math.cos(ang) * L, y + math.sin(ang) * L),
                               0.34, 0.06))
    gloss = diff(inter(union(*ticks), head), dilate(mask, 1.2),
                 dilate(forehead, 1.4), dilate(throat, 1.4), dilate(beak, 1.4),
                 *[circle(x, y, 5.2) for x, y in eyes],
                 P("M466,288 C478,297 490,301 500,301 C510,301 522,297 534,288 V310 H466 Z"))
    crown = diff(head, dilate(throat, 0.6), dilate(forehead, 0.6), dilate(beak, 1.1), gloss,
                 *[circle(x - 0.8, y - 0.7, 1.25) for x, y in eyes],
                 *[circle(x + 1.3, y + 1.0, 0.45) for x, y in eyes])
    # The bill is pale: drawn as an outline with the ridge down its middle.
    beak = union(diff(beak, diff(beak, stroke(to_d(beak), 1.5))),
                 inter(stroke("M500,261.6 L500,266.6", 0.6, cap="butt"), beak))
    head_ink = union(crown, beak)

    belly_lines = [stroke(to_d(body), 1.8)]
    flanks = diff(torso, diff(torso, stroke(to_d(torso), 13)))
    for y in range(318, 380, 6):
        arc = stroke(f"M466,{y} Q500,{y + 9} 534,{y}", 0.85, cap="butt")
        belly_lines.append(inter(arc, flanks))
    for j, y in enumerate(range(322, 360, 7)):         # chest feather tips
        for x in range(484 + (j % 2) * 4, 518, 8):
            belly_lines.append(inter(stroke(f"M{x - 3},{y} Q{x},{y + 3.2} {x + 3},{y}",
                                            0.6, cap="butt"), torso))

    # Chestnut and cinnamon, engraved: fine strokes falling through the gold.
    def feathered(region, n, y0, y1):
        strokes_ = []
        for _ in range(n):
            x, y = rnd.uniform(468, 532), rnd.uniform(y0, y1)
            L = rnd.uniform(2.5, 5)
            ang = math.pi / 2 + (x - 500) * 0.02
            strokes_.append(taper((x, y), (x + math.cos(ang) * L, y + math.sin(ang) * L),
                                  rnd.uniform(0.35, 0.55), 0.1))
        return diff(region, *strokes_)
    throat = feathered(throat, 80, 266, 288)
    forehead = feathered(forehead, 14, 248, 258)
    belly_gold = feathered(belly_gold, 110, 366, 446)
    face_gold = union(throat, forehead, belly_gold)

    bird = union(wing, mirror(wing), wing_lines, mirror(wing_lines))
    bird = lay(bird, tail, 1.6)
    bird = diff(bird, dilate(body, 1.2))
    bird = union(bird, head_ink, *belly_lines)
    bird = diff(bird, dilate(face_gold, 0.6))
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

    # Witten, after the photograph of the 1885 synagogue: a front block
    # framed by one great round arch with a rose window over the door,
    # slender corner turrets with spires, a tall drum with arched windows
    # under a bulbous ribbed dome with a spike, and a long wing to the right
    # with two rows of paired arched windows and its own curved gable.
    def arched(x, y_bot, w, h):
        r = w / 2
        return P(f"M{x - r},{y_bot} V{y_bot - h + r} A{r},{r} 0 0 1 {x + r},{y_bot - h + r} "
                 f"V{y_bot} Z")

    def spire(x, y_top, w=6, h=30, tip=14):
        return union(rect(x - w / 2, y_top, w, h + 200),
                     P(f"M{x - w / 2 - 1},{y_top} L{x},{y_top - tip} L{x + w / 2 + 1},{y_top} Z"))

    fc = 724                                   # front block centre
    front = union(rect(fc - 26, b - 74, 52, 74), arched(fc, b - 60, 52, 40))
    wing = union(rect(fc + 26, b - 54, 826 - fc - 26, 54),
                 poly([(fc + 26, b - 54), (812, b - 54), (806, b - 62), (fc + 30, b - 62)]))
    gable_r = union(rect(806, b - 66, 22, 66), arched(817, b - 64, 22, 18))
    left = union(rect(682, b - 40, fc - 26 - 682, 40),
                 poly([(682, b - 40), (fc - 26, b - 40), (fc - 26, b - 48), (688, b - 46)]))
    turrets = union(*[intersect for intersect in
                      [inter(spire(x, b - 86, 6, 0, 12), rect(0, 0, 1000, b)) for x in (fc - 26, fc + 26)]],
                    *[inter(spire(x, b - 80, 5, 0, 10), rect(0, 0, 1000, b)) for x in (806, 828)])
    roof = poly([(fc - 30, b - 74), (fc, b - 90), (fc + 30, b - 74)])
    drum = rect(fc - 13, b - 116, 26, 28)
    cornice = rect(fc - 15, b - 118, 30, 3)
    dome = P(f"M{fc - 15},{b - 118} C{fc - 19},{b - 132} {fc - 9},{b - 144} {fc},{b - 146} "
             f"C{fc + 9},{b - 144} {fc + 19},{b - 132} {fc + 15},{b - 118} Z")
    spike = union(rect(fc - 0.9, b - 162, 1.8, 18), circle(fc, b - 150, 2.4))
    syn = union(front, wing, gable_r, left, turrets, roof, drum, cornice, dome, spike)

    cuts = []
    # The great arch of the front, as a recessed line, and its rose window.
    cuts.append(diff(arched(fc, b - 4, 40, 64), arched(fc, b - 4, 37, 62.5)))
    cuts.append(diff(circle(fc, b - 46, 7), circle(fc, b - 46, 5.4)))
    for k in range(8):
        a_ = math.pi * k / 4
        cuts.append(taper((fc + 1.2 * math.cos(a_), b - 46 + 1.2 * math.sin(a_)),
                          (fc + 5.4 * math.cos(a_), b - 46 + 5.4 * math.sin(a_)), 0.7, 0.7))
    cuts.append(arched(fc, b, 12, 26))                                  # the door
    for x in (fc - 15, fc + 15):                                         # windows by it
        cuts.append(arched(x, b - 6, 4, 16))
    # Wing: two rows of paired arched windows.
    for x in range(fc + 34, 803, 12):
        for dx in (-2.2, 2.2):
            cuts.append(arched(x + dx, b - 30, 3, 14))
            cuts.append(arched(x + dx, b - 6, 3, 14))
    cuts.append(diff(circle(817, b - 52, 4.5), circle(817, b - 52, 3.2)))  # gable rose
    for x in (813, 821):
        cuts.append(arched(x, b - 8, 3, 18))
    for x in range(690, fc - 30, 9):                                    # left wing windows
        cuts.append(arched(x, b - 8, 3.4, 20))
    # Drum windows and dome ribs.
    for x in (fc - 8, fc, fc + 8):
        cuts.append(arched(x, b - 92, 4, 16))
    for o in (-10, -5, 0, 5, 10):
        cuts.append(stroke(f"M{fc + o},{b - 118} Q{fc + o * .85},{b - 136} {fc},{b - 145}",
                           0.7, cap="butt"))
    # Two-colour brick: light courses across every wall.
    walls = union(front, wing, gable_r, left)
    courses = union(*[rect(682, y, 148, 0.8) for y in range(int(b - 70), int(b - 1), 5)])
    cuts.append(diff(inter(courses, walls), *[dilate(c, 1.2) for c in cuts[:]]))
    # Gable edges: the decorative arched frieze, as a pale line under the eaves.
    cuts.append(diff(arched(fc, b - 60, 46, 36), arched(fc, b - 60, 44.4, 35.2)))
    cuts.append(diff(arched(817, b - 64, 18, 15), arched(817, b - 64, 16.6, 14.3)))
    syn = diff(syn, *cuts)
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


def school_bird(cx, cy, width=80):
    """The school's own bird, traced from its logo (src/school-bird.path)."""
    p = P(open(Path(__file__).with_name("school-bird.path")).read())
    p.fillType = pathops.FillType.EVEN_ODD
    p = pathops.simplify(p, clockwise=True)
    k = width / 100
    return p.transform(k, 0, 0, k, cx, cy)


def tiny_swallow(cx, cy, k=0.16):
    global SOLID, MARKER
    SOLID = MARKER = True
    bird, throat, _ = swallow()
    SOLID = MARKER = False
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
    lift = lambda p: _as_path(p).transform(1, 0, 0, 1, 0, -16)
    bird, throat, sil = lift(bird), lift(throat), lift(sil)
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
    # The school's own bird, traced from its logo, on both sides.
    diamonds = [school_bird(x, C, 88) for x in (C - 432, C + 432)]
    text_halo = dilate(union(top, bottom), 5)
    marks = [dilate(d, 2.8) for d in diamonds]
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
        def merged(shapes):
            # One path per colour when Skia can merge them; otherwise the
            # shapes side by side, which paints the same.
            try:
                return [to_d(union(*shapes))]
            except pathops.PathOpsError:
                return [to_d(x) for x in shapes]
        paths = "\n".join(f'  <path fill="{c}" d="{d}"/>'
                          for c, s in by.items() for d in merged(s))
        (out / f"{prefix}-{name}.svg").write_text(
            '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 1000" '
            f'role="img" aria-label="{NAME} · משלחת לגרמניה 2026">\n'
            f"  <title>{NAME} · משלחת לגרמניה 2026</title>\n{paths}\n</svg>\n",
            encoding="utf-8")
        print("wrote", prefix, name)


if __name__ == "__main__":
    main()
