"""Write preview.html: every concept on light and dark, large and at 64px.

Each lockup and mark is embedded once as an SVG <symbol>; its paths are
filled with CSS variables so one copy renders every color variant.
"""
from html import escape

from build import CONCEPTS, GOLD, GOLD_LT, BLUE, INK, NAME, ROOT, shapes
from geo import to_d, union

ABOUT = {
    "1-swallow-bridge": (
        "Dror means both “swallow” and “freedom”. The swallow’s wings make "
        "an arch, a bridge carried by a bird that crosses borders every "
        "year, and the gold line beneath is the way across. It continues the "
        "swallow in the school’s current logo, lighter and moving forward."),
    "2-flame-sprout": (
        "A memorial flame with a young sprout growing from its heart: "
        "remembrance that makes room for growth. The sprout is cut out of "
        "the flame, so the two are one shape. The quietest of the four, set "
        "in the serif Frank Ruhl Libre, with the line “remembering the past, "
        "building the future” underneath."),
    "3-dalet-d": (
        "The Hebrew ד of Dror and the Latin D face each other under one "
        "roofline: two alphabets, one name. The roof of the ד reaches toward "
        "the D and stops a hairline short, cut to the curve of its "
        "neighbour, so they meet without merging. This is the one concept "
        "that carries the German line."),
    "4-two-cities": (
        "A Jerusalem stone gate and citadel on one side, the Reichstag’s "
        "glass dome and Berlin’s TV tower on the other, joined by a bridge "
        "drawn in the same single line. The gold sun over the bridge is "
        "light over both. The avatar version keeps one landmark per city."),
}

NOTES = {
    "1-swallow-bridge": "Rubik. Hebrew + English.",
    "2-flame-sprout": "Frank Ruhl Libre, tagline in Assistant. Hebrew only.",
    "3-dalet-d": "Heebo. Hebrew + German.",
    "4-two-cities": "Assistant. Hebrew + English.",
}

ROLES = ("primary", "accent", "text", "sub")


def symbol(sid, view, shape_list):
    by_role = {}
    for role, s in shape_list:
        by_role.setdefault(role, []).append(s)
    paths = "".join(
        f'<path style="fill:var(--r-{r})" d="{to_d(union(*ss))}"/>'
        for r, ss in by_role.items())
    return f'<symbol id="{sid}" viewBox="{view}">{paths}</symbol>'


def use(sid, view, variant, label):
    # A symbol fills the <use> box, so the box must cover the full viewBox,
    # including its origin, or a mark whose viewBox is offset lands askew.
    x, y, w, h = view.split()
    return (f'<svg class="art v-{variant}" viewBox="{view}" role="img" '
            f'aria-label="{escape(label)}"><use href="#{sid}" x="{x}" y="{y}" '
            f'width="{w}" height="{h}"/></svg>')


def main():
    symbols, compare, details = [], [], []
    for i, c in enumerate(CONCEPTS, 1):
        lockup, mark, (x, y, s) = shapes(c)
        lv, mv = "0 0 1000 1000", f"{x:.1f} {y:.1f} {s:.1f} {s:.1f}"
        lid, mid = f"c{i}-logo", f"c{i}-mark"
        symbols += [symbol(lid, lv, lockup), symbol(mid, mv, mark)]
        t = c["title"]

        def L(v, lbl=""):
            return use(lid, lv, v, f"{t} {lbl}".strip())

        def M(v):
            return use(mid, mv, v, f"{t} mark")

        compare.append(f"""
      <div class="col">
        <h3 class="col-name"><span class="num">{i}</span> {escape(t)}</h3>
        <div class="tile light big">{L("color")}</div>
        <div class="tile dark big">{L("reverse")}</div>
        <div class="avatars">
          <div class="av light" title="64px on light">{M("color")}</div>
          <div class="av dark" title="64px on dark">{M("reverse")}</div>
          <div class="av light small" title="32px">{M("color")}</div>
          <div class="av light mono" title="64px, black only">{M("black")}</div>
        </div>
      </div>""")

        lines = "".join(
            f'<li dir="auto">{escape(spec[0])}</li>' for spec in c["lines"])
        details.append(f"""
    <article class="concept" id="concept-{i}">
      <div class="concept-text">
        <p class="eyebrow">Concept {i}</p>
        <h2>{escape(t)}</h2>
        <p>{escape(ABOUT[c['slug']])}</p>
        <h4>Wording</h4>
        <ul class="wording">{lines}</ul>
        <p class="meta">{escape(NOTES[c['slug']])} Files in
          <code>{c['slug']}/</code></p>
      </div>
      <div class="variants">
        <figure><div class="tile light">{L("color")}</div>
          <figcaption>Color</figcaption></figure>
        <figure><div class="tile dark">{L("reverse")}</div>
          <figcaption>Reverse, on dark</figcaption></figure>
        <figure><div class="tile light">{L("black")}</div>
          <figcaption>One color, black</figcaption></figure>
        <figure><div class="tile dark">{L("white")}</div>
          <figcaption>One color, white</figcaption></figure>
      </div>
    </article>""")

    html = TEMPLATE.format(
        symbols="".join(symbols), compare="".join(compare),
        details="".join(details), BLUE=BLUE, INK=INK, GOLD=GOLD,
        GOLD_LT=GOLD_LT, NAME=NAME)
    (ROOT / "preview.html").write_text(html, encoding="utf-8")
    print("wrote preview.html", len(html) // 1024, "KB")


TEMPLATE = """<title>Dror Germany Logo Concepts</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Assistant:wght@400;600;700&family=Frank+Ruhl+Libre:wght@500;700&display=swap">
<style>
/* Layout: a review sheet. A four-up comparison first, then one row per concept. */
:root {{
  --bg: #F3F5F8;
  --fg: #12203B;
  --muted: #56627A;
  --rule: #D9DFE8;
  --panel: #FFFFFF;
  --link: {BLUE};
  /* The test grounds stay the same in both page themes: they are the point. */
  --tile-light: #FFFFFF;
  --tile-dark: #141A24;
  --tile-edge: #D9DFE8;
  --font-display: "Frank Ruhl Libre", "Times New Roman", serif;
  --font-body: "Assistant", "Segoe UI", system-ui, sans-serif;
  --font-mono: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
}}
@media (prefers-color-scheme: dark) {{
  :root:not([data-theme="light"]) {{
    --bg: #0B1018; --fg: #E6EBF3; --muted: #98A3B8; --rule: #253043;
    --panel: #111826; --link: #8FB0EA; --tile-edge: #2A3548;
    color-scheme: dark;
  }}
}}
:root[data-theme="dark"] {{
  --bg: #0B1018; --fg: #E6EBF3; --muted: #98A3B8; --rule: #253043;
  --panel: #111826; --link: #8FB0EA; --tile-edge: #2A3548;
  color-scheme: dark;
}}
.v-color   {{ --r-primary: {BLUE}; --r-accent: {GOLD}; --r-text: {INK}; --r-sub: {BLUE}; }}
.v-reverse {{ --r-primary: #FFFFFF; --r-accent: {GOLD_LT}; --r-text: #FFFFFF; --r-sub: #D6E0F0; }}
.v-black   {{ --r-primary: #000000; --r-accent: #000000; --r-text: #000000; --r-sub: #000000; }}
.v-white   {{ --r-primary: #FFFFFF; --r-accent: #FFFFFF; --r-text: #FFFFFF; --r-sub: #FFFFFF; }}

body {{
  background: var(--bg); color: var(--fg);
  font-family: var(--font-body); font-size: 1.0625rem; line-height: 1.55;
}}
.wrap {{ max-width: 1240px; margin: 0 auto; padding-inline: 20px; padding-block: 40px 72px; }}
header {{ display: grid; gap: 10px; max-width: 760px; margin-bottom: 36px; }}
.eyebrow {{ margin: 0; font-size: .78rem; letter-spacing: .12em; text-transform: uppercase; color: var(--muted); font-weight: 600; }}
h1 {{ margin: 0; font-family: var(--font-display); font-weight: 700; font-size: clamp(1.9rem, 4vw, 2.7rem); line-height: 1.15; text-wrap: balance; }}
h1 .he {{ display: block; font-size: .62em; font-weight: 500; color: var(--muted); }}
header p {{ margin: 0; color: var(--muted); max-width: 64ch; }}
h2 {{ font-family: var(--font-display); font-weight: 700; font-size: 1.6rem; margin: 0; text-wrap: balance; }}
.section-head {{ display: flex; justify-content: space-between; align-items: baseline; gap: 12px; flex-wrap: wrap; border-top: 1px solid var(--rule); padding-top: 18px; margin: 0 0 18px; }}
.section-head p {{ margin: 0; color: var(--muted); font-size: .95rem; }}

.compare {{ display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 16px; margin-bottom: 56px; }}
.col {{ display: grid; gap: 10px; align-content: start; min-width: 0; }}
.col-name {{ margin: 0; font-size: 1rem; font-weight: 700; display: flex; gap: 8px; align-items: baseline; }}
.num {{ font-family: var(--font-display); color: var(--muted); font-variant-numeric: tabular-nums; }}

.tile {{ border-radius: 6px; border: 1px solid var(--tile-edge); overflow: hidden; display: grid; place-items: center; }}
.tile.light {{ background: var(--tile-light); }}
.tile.dark {{ background: var(--tile-dark); }}
.tile .art {{ display: block; width: 100%; height: auto; aspect-ratio: 1; max-width: 100%; }}

.avatars {{ display: flex; gap: 12px; align-items: center; flex-wrap: wrap; padding-top: 4px; }}
.av {{ width: 64px; height: 64px; border-radius: 50%; overflow: hidden; flex: none; border: 1px solid var(--tile-edge); }}
.av.light {{ background: var(--tile-light); }}
.av.dark {{ background: var(--tile-dark); border-color: var(--tile-dark); }}
.av.small {{ width: 32px; height: 32px; }}
.av .art {{ display: block; width: 100%; height: 100%; }}
.legend {{ font-size: .85rem; color: var(--muted); margin: -40px 0 56px; }}

.concept {{ display: grid; grid-template-columns: minmax(0, 5fr) minmax(0, 7fr); gap: 32px; padding-block: 28px; border-top: 1px solid var(--rule); }}
.concept-text {{ display: grid; gap: 10px; align-content: start; min-width: 0; }}
.concept-text p {{ margin: 0; max-width: 60ch; }}
.concept-text h4 {{ margin: 8px 0 0; font-size: .78rem; letter-spacing: .12em; text-transform: uppercase; color: var(--muted); }}
.wording {{ margin: 0; padding: 0; list-style: none; display: grid; gap: 2px; }}
.wording li {{ font-size: 1.05rem; }}
.wording li[dir="rtl"], .wording li:dir(rtl) {{ text-align: right; }}
.meta {{ color: var(--muted); font-size: .92rem; }}
code {{ font-family: var(--font-mono); font-size: .85em; }}
.variants {{ display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; min-width: 0; }}
figure {{ margin: 0; display: grid; gap: 6px; min-width: 0; }}
figcaption {{ font-size: .82rem; color: var(--muted); }}

.palette {{ display: flex; gap: 20px; flex-wrap: wrap; margin: 0 0 24px; padding: 0; list-style: none; }}
.palette li {{ display: flex; gap: 10px; align-items: center; font-size: .92rem; }}
.sw {{ width: 36px; height: 36px; border-radius: 50%; border: 1px solid var(--tile-edge); flex: none; }}
.checks {{ margin: 0; padding-left: 1.2em; display: grid; gap: 4px; max-width: 72ch; }}
.notes {{ border-top: 1px solid var(--rule); padding-top: 18px; display: grid; gap: 14px; }}

@media (max-width: 980px) {{
  .compare {{ grid-template-columns: repeat(2, minmax(0, 1fr)); }}
  .concept {{ grid-template-columns: minmax(0, 1fr); }}
}}
@media (max-width: 560px) {{
  .wrap {{ padding-inline: 16px; }}
  .compare {{ grid-template-columns: minmax(0, 1fr); }}
  .variants {{ grid-template-columns: repeat(2, minmax(0, 1fr)); }}
}}
</style>

<svg width="0" height="0" style="position:absolute" aria-hidden="true">{symbols}</svg>

<div class="wrap">
  <header>
    <p class="eyebrow">Logo concepts · round 1</p>
    <h1><span class="he" lang="he" dir="rtl">{NAME} · משלחת לגרמניה 2026</span>
      Dror School, delegation to Germany</h1>
    <p>Four directions, each holding friendship and remembrance together. No
      flags, eagles, blackletter or wartime imagery. The blue is sampled from
      the school’s current logo. All text is converted to outlines.</p>
  </header>

  <div class="section-head">
    <h2>Side by side</h2>
    <p>Large on light and on dark, then the avatar mark.</p>
  </div>
  <div class="compare">{compare}
  </div>
  <p class="legend">Avatar row, left to right: 64px on light, 64px on dark,
    32px, and 64px in black only. Circles show a WhatsApp-style round crop.</p>

  <div class="section-head">
    <h2>The concepts</h2>
    <p>Each in color, reverse, and one color.</p>
  </div>
  {details}

  <section class="notes">
    <h2>Palette and checks</h2>
    <ul class="palette">
      <li><span class="sw" style="background:{BLUE}"></span>School blue {BLUE}</li>
      <li><span class="sw" style="background:{INK}"></span>Deep blue {INK}</li>
      <li><span class="sw" style="background:{GOLD}"></span>Stone gold {GOLD}</li>
      <li><span class="sw" style="background:{GOLD_LT}"></span>Gold on dark {GOLD_LT}</li>
    </ul>
    <ul class="checks">
      <li>Hebrew shaped with HarfBuzz and checked at full size: right-to-left order, and 2026 placed correctly inside the Hebrew line.</li>
      <li>Every SVG is plain filled paths: no fonts, strokes or masks, so it prints, cuts and embroiders the same everywhere.</li>
      <li>Each concept has a separate, heavier mark for small uses, padded to survive a round crop.</li>
      <li>Concept 3 at very small sizes: readers who don’t know Hebrew may see the ד as a T. Hebrew readers see ד.</li>
    </ul>
  </section>
</div>
"""

if __name__ == "__main__":
    main()
