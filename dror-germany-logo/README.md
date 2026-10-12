# קריית חינוך דרור · משלחת לגרמניה 2026: logo concepts, round 1

Brand guide: https://claude.ai/artifact/1fteou2y4ZPJA12cun1MaQ · Mockup board: https://claude.ai/artifact/1nXB2ToUP7QG5mFv7zPHLp (private until shared).

Round 2 (concepts 1 and 2, refined, with T-shirt, badge and WhatsApp mockups): open `round2.html`.

Open `preview.html` in a browser to see all four concepts side by side on light and dark backgrounds, and at 64px avatar size.

## Concepts

**1. Swallow Bridge** (`1-swallow-bridge/`)
Dror means both "swallow" and "freedom". The swallow's wings make an arch, a bridge carried by a bird that crosses borders every year, and the gold line beneath is the way across. It continues the swallow in the school's current logo, lighter and moving forward. *Rubik; Hebrew + English.*

**2. Flame & Sprout** (`2-flame-sprout/`)
A memorial flame with a young sprout growing from its heart: remembrance that makes room for growth. The sprout is cut out of the flame, so the two are one shape. The quietest of the four, with the line "זוכרים את העבר · בונים את העתיד" underneath. *Frank Ruhl Libre, tagline in Assistant; Hebrew only.*

**3. ד meets D** (`3-dalet-d/`)
The Hebrew ד of Dror and the Latin D face each other under one roofline: two alphabets, one name. The roof of the ד reaches toward the D and stops a hairline short, cut to the curve of its neighbour, so they meet without merging. The one concept with the German line. *Heebo; Hebrew + German.*

**4. Two Cities, One Line** (`4-two-cities/`)
A Jerusalem stone gate and citadel on one side, the Reichstag's glass dome and Berlin's TV tower on the other, joined by a bridge drawn in the same single line. The gold sun over the bridge is light over both. The avatar version keeps one landmark per city. *Assistant; Hebrew + English.*

## Files in each concept folder

| File | Use |
|---|---|
| `logo-color.svg` | Full lockup, blue and gold, for light backgrounds |
| `logo-reverse.svg` | White and gold, for dark backgrounds |
| `logo-black.svg` / `logo-white.svg` | One-color versions |
| `logo-horizontal-*.svg` | Concepts 1 and 2 only: symbol on the right, text beside it, for banners and letterheads |
| `mark-*.svg` | Symbol only, heavier, padded for round crops (avatars, badges) |
| `png/*.png` | Every SVG above rendered at 1024px, transparent background |

All text is converted to outlines and every shape is a plain filled path (no fonts, strokes or masks).

## Palette

| | |
|---|---|
| School blue | `#0F408E` (sampled from the current school logo) |
| Deep blue (name) | `#0B2D63` |
| Stone gold | `#C49A3C` |
| Gold on dark | `#E2BE68` |

## Rebuilding

Requires Python 3 with `fonttools`, `uharfbuzz` and `skia-pathops`, plus Node with Playwright and Chromium.

```sh
python3 src/build.py          # SVGs for every concept and variant
python3 src/preview.py        # preview.html
python3 src/round2.py         # round2.html
node src/render.mjs 1024 1-swallow-bridge/png/logo-color.png 1-swallow-bridge/logo-color.svg   # PNGs, in pairs
```

Fonts in `fonts/` are from Google Fonts under the SIL Open Font License (`fonts/OFL-*.txt`).
