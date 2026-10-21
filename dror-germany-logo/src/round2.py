"""Round 2 prototype: concepts 1 and 2, refined, with simple mockups."""
from html import escape

from build import (CONCEPTS, BLUE, GOLD, GOLD_LT, INK, NAME, ROOT, VARIANTS,
                   layers_for, shapes, svg)
from geo import P, to_d, union
from textpath import line_path

PICKS = [c for c in CONCEPTS if c["slug"][0] in "12"]


def placed(shape_list, scale, dx, dy):
    return [(r, P(to_d(s)).transform(scale, 0, 0, scale, dx, dy))
            for r, s in shape_list]


def horizontal(c):
    """Mark on the right (where Hebrew starts), text right-aligned beside it."""
    mark = c["mark"]()
    x0, y0, x1, y1 = union(*[s for _, s in mark]).bounds
    k = 340 / (y1 - y0)
    out = placed(mark, k, 1540 - x1 * k, 80 - y0 * k)
    edge = 1540 - (x1 - x0) * k - 60
    name, sub = c["lines"][0], c["lines"][1]
    for (text, size, wght, *_), y, role in ((name, 250, "text"),
                                            (sub, 345, "sub")):
        d, _ = line_path(text, c["font"], size * 1.15, edge, y,
                         anchor="end", wght=wght)
        out.append((role, P(d)))
    bx0, by0, bx1, by1 = union(*[s for _, s in out]).bounds
    view = f"{bx0 - 40:.0f} {by0 - 40:.0f} {bx1 - bx0 + 80:.0f} {by1 - by0 + 80:.0f}"
    return out, view


def symbol(sid, view, shape_list):
    by_role = {}
    for role, s in shape_list:
        by_role.setdefault(role, []).append(s)
    paths = "".join(f'<path style="fill:var(--r-{r})" d="{to_d(union(*ss))}"/>'
                    for r, ss in by_role.items())
    return f'<symbol id="{sid}" viewBox="{view}">{paths}</symbol>'


def use(sid, view, variant, cls="art"):
    x, y, w, h = view.split()
    return (f'<svg class="{cls} v-{variant}" viewBox="{view}" aria-hidden="true">'
            f'<use href="#{sid}" x="{x}" y="{y}" width="{w}" height="{h}"/></svg>')


TEE = ("M110,40 L170,18 C190,44 250,44 270,18 L330,40 L392,110 L346,150 "
       "L316,124 L316,380 L124,380 L124,124 L94,150 L48,110 Z")


def main():
    syms, blocks = [], []
    for i, c in enumerate(PICKS, 1):
        lockup, mark, (x, y, s) = shapes(c)
        hz, hview = horizontal(c)
        for vname, colors in VARIANTS.items():
            (ROOT / c["slug"] / f"logo-horizontal-{vname}.svg").write_text(
                svg(hview, layers_for(hz, colors, vname in ("black", "white")),
                    f"{NAME} · {c['title']}"), encoding="utf-8")
        lv, mv = "0 0 1000 1000", f"{x:.1f} {y:.1f} {s:.1f} {s:.1f}"
        n = c["slug"][0]
        syms += [symbol(f"l{n}", lv, lockup), symbol(f"m{n}", mv, mark),
                 symbol(f"h{n}", hview, hz)]
        blocks.append(f"""
  <section class="concept">
    <h2><span class="num">{n}</span> {escape(c['title'])}</h2>
    <div class="row">
      <figure class="tile light sq">{use(f"l{n}", lv, "color")}<figcaption>Stacked</figcaption></figure>
      <figure class="tile dark sq">{use(f"l{n}", lv, "reverse")}<figcaption>On dark</figcaption></figure>
      <figure class="tile light wide">{use(f"h{n}", hview, "color")}<figcaption>Horizontal, for banners and letterheads</figcaption></figure>
    </div>
    <div class="row mock">
      <figure class="tile shirt">
        <svg viewBox="0 0 440 400" class="tee" aria-hidden="true">
          <path d="{TEE}" fill="var(--shirt)"/>
          <svg x="238" y="88" width="56" height="56" viewBox="{mv}" class="v-{'reverse' if n == '1' else 'reverse'}"><use href="#m{n}" x="{x:.1f}" y="{y:.1f}" width="{s:.1f}" height="{s:.1f}"/></svg>
        </svg>
        <figcaption>Navy T-shirt, chest mark</figcaption>
      </figure>
      <figure class="tile badge-wrap">
        <div class="badge">
          <div class="slot"></div>
          {use(f"l{n}", lv, "color", "badge-logo")}
          <p class="who" lang="he" dir="rtl">שם התלמיד/ה</p>
        </div>
        <figcaption>Lanyard badge</figcaption>
      </figure>
      <figure class="tile wa-wrap">
        <div class="wa" lang="he" dir="rtl">
          <div class="av">{use(f"m{n}", mv, "color")}</div>
          <div><b>משלחת לגרמניה 2026</b><span>34 משתתפים</span></div>
        </div>
        <figcaption>WhatsApp group</figcaption>
      </figure>
    </div>
  </section>""")
    html = TEMPLATE.format(syms="".join(syms), blocks="".join(blocks),
                           BLUE=BLUE, GOLD=GOLD, GOLD_LT=GOLD_LT, INK=INK)
    (ROOT / "round2.html").write_text(html, encoding="utf-8")
    print("wrote round2.html", len(html) // 1024, "KB")


TEMPLATE = """<title>Dror Germany Round 2</title>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Assistant:wght@400;600;700&family=Frank+Ruhl+Libre:wght@700&display=swap">
<style>
/* Layout: two concepts, each a row of lockups then a row of mockups. */
:root {{
  --bg: #F3F5F8; --fg: #12203B; --muted: #56627A; --rule: #D9DFE8;
  --tile-light: #FFFFFF; --tile-dark: #141A24; --edge: #D9DFE8;
  --shirt: #1B2740; --wa: #F0F2F5;
  --font-display: "Frank Ruhl Libre", Georgia, serif;
  --font-body: "Assistant", "Segoe UI", system-ui, sans-serif;
}}
@media (prefers-color-scheme: dark) {{ :root:not([data-theme="light"]) {{
  --bg: #0B1018; --fg: #E6EBF3; --muted: #98A3B8; --rule: #253043; --edge: #2A3548; color-scheme: dark; }} }}
:root[data-theme="dark"] {{ --bg: #0B1018; --fg: #E6EBF3; --muted: #98A3B8; --rule: #253043; --edge: #2A3548; color-scheme: dark; }}
.v-color   {{ --r-primary: {BLUE}; --r-accent: {GOLD}; --r-text: {INK}; --r-sub: {BLUE}; }}
.v-reverse {{ --r-primary: #FFFFFF; --r-accent: {GOLD_LT}; --r-text: #FFFFFF; --r-sub: #D6E0F0; }}
body {{ background: var(--bg); color: var(--fg); font-family: var(--font-body); font-size: 1.05rem; line-height: 1.5; }}
.wrap {{ max-width: 1180px; margin: 0 auto; padding-inline: 20px; padding-block: 36px 64px; display: grid; gap: 40px; }}
header {{ display: grid; gap: 8px; max-width: 70ch; }}
h1, h2 {{ font-family: var(--font-display); margin: 0; text-wrap: balance; }}
h1 {{ font-size: clamp(1.8rem, 4vw, 2.4rem); }}
h2 {{ font-size: 1.5rem; display: flex; gap: 10px; align-items: baseline; }}
.num {{ color: var(--muted); }}
header p {{ margin: 0; color: var(--muted); }}
.concept {{ display: grid; gap: 14px; border-top: 1px solid var(--rule); padding-top: 22px; }}
.row {{ display: grid; grid-template-columns: 1fr 1fr 2fr; gap: 14px; }}
.row.mock {{ grid-template-columns: repeat(3, minmax(0, 1fr)); }}
figure {{ margin: 0; min-width: 0; }}
.tile {{ border: 1px solid var(--edge); border-radius: 6px; background: var(--tile-light); display: grid; align-content: space-between; overflow: hidden; }}
.tile.dark {{ background: var(--tile-dark); }}
.tile figcaption {{ font-size: .82rem; color: #56627A; padding: 6px 10px; background: rgba(255,255,255,.9); }}
.tile.dark figcaption {{ color: #AEB8CA; background: rgba(0,0,0,.25); }}
.art {{ display: block; width: 100%; height: auto; }}
.wide {{ align-content: center; }}
.wide .art {{ padding: 6% 4%; box-sizing: border-box; }}
.tee {{ display: block; width: 100%; height: auto; background: #E9ECF1; }}
.badge-wrap, .wa-wrap {{ background: #E9ECF1; }}
.badge {{ width: 58%; margin: 18px auto; background: #FFFFFF; border-radius: 10px; box-shadow: 0 2px 10px rgba(16,32,64,.15); padding: 14px 10px 16px; display: grid; justify-items: center; gap: 4px; }}
.slot {{ width: 26%; height: 7px; border-radius: 4px; background: #D3D9E2; }}
.badge-logo {{ width: 100%; height: auto; }}
.who {{ margin: 0; font-weight: 700; color: #12203B; font-size: 1rem; }}
.wa-wrap {{ grid-template-rows: 1fr auto; }}
.wa {{ align-self: center; margin: 0 16px; background: var(--wa); border-radius: 10px; padding: 12px; display: flex; gap: 12px; align-items: center; color: #111B21; }}
.wa .av {{ width: 64px; height: 64px; border-radius: 50%; background: #FFFFFF; overflow: hidden; flex: none; }}
.wa .av .art {{ width: 64px; height: 64px; }}
.wa b {{ display: block; font-size: 1rem; }}
.wa span {{ font-size: .85rem; color: #667781; }}
@media (max-width: 860px) {{
  .row, .row.mock {{ grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); }}
  .wide {{ grid-column: 1 / -1; }}
}}
@media (max-width: 520px) {{ .wrap {{ padding-inline: 16px; }} .row.mock {{ grid-template-columns: minmax(0, 1fr); }} }}
</style>
<svg width="0" height="0" style="position:absolute" aria-hidden="true">{syms}</svg>
<div class="wrap">
  <header>
    <h1>Round 2 prototype</h1>
    <p>Concepts 1 and 2, refined, with quick mockups. The swallow’s wings are calmer, with longer tail streamers and a shorter horizon line. The flame has a sharper tip and a second tongue that keeps it from reading as a drop. Pick one to develop.</p>
  </header>
  {blocks}
</div>
"""

if __name__ == "__main__":
    main()
