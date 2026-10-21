"""Trace the school's own bird (src/school-bird.png) into a vector path.

Writes src/school-bird.path: one SVG path, its bounding box centred on the
origin and 100 units wide. The raster is small, so it is enlarged and
softened before tracing to give smooth curves instead of pixel steps.
"""
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter
from skimage.measure import approximate_polygon, find_contours

HERE = Path(__file__).resolve().parent
UP = 8

im = np.asarray(Image.open(HERE / "school-bird.png").convert("RGB")).astype(float)
blue = np.clip((im[..., 2] - im[..., 0] - 40) / 60, 0, 1)
mask = Image.fromarray((blue * 255).astype(np.uint8))
mask = mask.resize((mask.width * UP, mask.height * UP), Image.BICUBIC)
mask = mask.filter(ImageFilter.GaussianBlur(UP * 0.45))
arr = np.pad(np.asarray(mask).astype(float) / 255, 2)

contours = [approximate_polygon(c, tolerance=0.9) for c in find_contours(arr, 0.5)]
contours = [c for c in contours if len(c) > 4]
pts = np.concatenate(contours)
(y0, x0), (y1, x1) = pts.min(0), pts.max(0)
cx, cy, k = (x0 + x1) / 2, (y0 + y1) / 2, 100 / (x1 - x0)
d = " ".join("M" + " L".join(f"{(x - cx) * k:.2f},{(y - cy) * k:.2f}" for y, x in c) + " Z"
             for c in contours)
# The eye is a dot a few pixels wide, too small to survive the softening;
# it is added back as a circle at its place in the source image.
ex, ey, er = (160 * UP + 2 - cx) * k, (58 * UP + 2 - cy) * k, 1.05
d += (f" M{ex - er:.2f},{ey:.2f} A{er},{er} 0 1 0 {ex + er:.2f},{ey:.2f}"
      f" A{er},{er} 0 1 0 {ex - er:.2f},{ey:.2f} Z")
(HERE / "school-bird.path").write_text(d + "\n")
print(len(contours), "contours,", len(d), "chars; height", round((y1 - y0) * k, 1))
