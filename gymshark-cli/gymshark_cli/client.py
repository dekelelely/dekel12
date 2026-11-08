"""Read-only client for Gymshark's public storefront.

Gymshark runs a headless Shopify store on Next.js. Every collection and
product page embeds its data as JSON in a ``<script id="__NEXT_DATA__">``
tag, so no API key is needed: we fetch the page, pull out that JSON, and
normalise it into plain dicts.

Ground rules this client follows:
  * Only paths robots.txt allows (/collections, /products). No /search,
    /cart, /account or /checkout.
  * An honest User-Agent, gzip, an on-disk cache and a delay between
    uncached requests, so repeated runs don't hammer the site.
  * If Gymshark sends us to its Queue-it waiting room (product drops), we
    stop and say so instead of trying to get around it.
"""

from __future__ import annotations

import gzip
import hashlib
import json
import os
import re
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path

DEFAULT_BASE_URL = "https://www.gymshark.com"
USER_AGENT = "gymshark-cli/0.1 (personal read-only tool)"
NEXT_DATA_RE = re.compile(r'<script id="__NEXT_DATA__"[^>]*>(.*?)</script>', re.S)
GENDERS = ("mens", "womens")


class GymsharkError(Exception):
    """Something went wrong talking to Gymshark."""


class NotFound(GymsharkError):
    pass


class WaitingRoom(GymsharkError):
    """Gymshark redirected us to its virtual queue (usually a product drop)."""


class _NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None  # surface 3xx to the caller so we can inspect Location


@dataclass
class Page:
    """One page of a collection listing."""

    slug: str
    gender: str | None
    page: int  # 0-based, as Gymshark's search backend reports it
    pages: int
    total: int
    products: list[dict]
    categories: list[dict]
    title: str


def product_handle(value: str) -> str:
    """Accept a bare handle or any gymshark.com product URL."""
    value = value.strip()
    if "/" not in value:
        return value
    path = urllib.parse.urlparse(value).path if "://" in value else value
    m = re.search(r"/products/([^/?#]+)", path)
    if not m:
        raise GymsharkError(f"not a product URL or handle: {value}")
    return m.group(1)


class Client:
    def __init__(
        self,
        base_url: str | None = None,
        cache_dir: str | os.PathLike | None = None,
        cache_ttl: float = 600,
        delay: float = 1.0,
        timeout: float = 30,
        fetcher=None,
    ):
        self.base_url = (base_url or os.environ.get("GYMSHARK_BASE_URL") or DEFAULT_BASE_URL).rstrip("/")
        self.host = urllib.parse.urlparse(self.base_url).netloc
        if cache_dir is None:
            root = os.environ.get("XDG_CACHE_HOME") or os.path.join(Path.home(), ".cache")
            cache_dir = os.path.join(root, "gymshark-cli")
        self.cache_dir = Path(cache_dir)
        self.cache_ttl = cache_ttl
        self.delay = delay
        self.timeout = timeout
        self._fetch = fetcher or self._http_get
        self._opener = urllib.request.build_opener(_NoRedirect)
        self._last_request = 0.0

    # ---- transport -------------------------------------------------------

    def _http_get(self, url: str, _hops: int = 0) -> str:
        wait = self.delay - (time.monotonic() - self._last_request)
        if wait > 0:
            time.sleep(wait)
        self._last_request = time.monotonic()
        req = urllib.request.Request(
            url,
            headers={"User-Agent": USER_AGENT, "Accept": "text/html", "Accept-Encoding": "gzip"},
        )
        try:
            with self._opener.open(req, timeout=self.timeout) as resp:
                body = resp.read()
                if resp.headers.get("Content-Encoding") == "gzip":
                    body = gzip.decompress(body)
                return body.decode("utf-8", "replace")
        except urllib.error.HTTPError as e:
            if e.code in (301, 302, 303, 307, 308):
                location = urllib.parse.urljoin(url, e.headers.get("Location", ""))
                host = urllib.parse.urlparse(location).netloc
                if host.startswith("queue."):
                    raise WaitingRoom(
                        f"{url} is behind Gymshark's waiting room right now (likely a product drop). "
                        "Open it in a browser to join the queue."
                    ) from None
                if host != self.host or _hops >= 5:
                    raise GymsharkError(f"unexpected redirect to {location}") from None
                return self._http_get(location, _hops + 1)
            if e.code == 404:
                raise NotFound(f"not found: {url}") from None
            raise GymsharkError(f"HTTP {e.code} from {url}") from None
        except urllib.error.URLError as e:
            raise GymsharkError(f"could not reach {url}: {e.reason}") from None

    def _cache_path(self, url: str) -> Path:
        return self.cache_dir / (hashlib.sha256(url.encode()).hexdigest()[:32] + ".json")

    def next_data(self, path: str, params: dict | None = None, fresh: bool = False) -> dict:
        """Fetch a page and return its embedded __NEXT_DATA__ JSON."""
        url = self.base_url + path
        if params:
            url += "?" + urllib.parse.urlencode(params)
        cpath = self._cache_path(url)
        if not fresh and self.cache_ttl > 0 and cpath.exists():
            if time.time() - cpath.stat().st_mtime < self.cache_ttl:
                try:
                    return json.loads(cpath.read_text())
                except (OSError, ValueError):
                    pass
        html = self._fetch(url)
        m = NEXT_DATA_RE.search(html)
        if not m:
            raise GymsharkError(f"no page data found at {url} (the site layout may have changed)")
        data = json.loads(m.group(1))
        if data.get("page") == "/404":
            raise NotFound(f"not found: {url}")
        if self.cache_ttl > 0:
            try:
                self.cache_dir.mkdir(parents=True, exist_ok=True)
                cpath.write_text(json.dumps(data))
            except OSError:
                pass
        return data

    # ---- collections -----------------------------------------------------

    def collection(self, slug: str, gender: str | None = None, page: int = 0, fresh: bool = False) -> Page:
        if gender and gender not in GENDERS:
            raise GymsharkError(f"gender must be one of {', '.join(GENDERS)}")
        path = f"/collections/{urllib.parse.quote(slug)}" + (f"/{gender}" if gender else "")
        data = self.next_data(path, {"page": page} if page else None, fresh=fresh)
        props = data.get("props", {}).get("pageProps", {})
        q = props.get("ssrQuery")
        if not q:
            raise GymsharkError(f"collection '{slug}' has no product listing")
        return Page(
            slug=slug,
            gender=gender,
            page=q.get("page", page),
            pages=q.get("nbPages", 1),
            total=q.get("nbHits", 0),
            products=[normalise(h) for h in q.get("hits", [])],
            categories=[
                {"name": c.get("name", "").strip(), "handle": c.get("handle")}
                for c in props.get("productTypeFilters") or []
            ],
            title=props.get("title") or slug,
        )

    def iter_collection(self, slug, gender=None, start=0, max_pages=1, fresh=False):
        """Yield Page objects, following pagination up to max_pages (None = all)."""
        page = start
        fetched = 0
        while True:
            p = self.collection(slug, gender, page, fresh=fresh)
            yield p
            fetched += 1
            page += 1
            if page >= p.pages or (max_pages is not None and fetched >= max_pages):
                return

    # ---- products --------------------------------------------------------

    def product(self, handle_or_url: str, fresh: bool = False) -> dict:
        handle = product_handle(handle_or_url)
        data = self.next_data(f"/products/{urllib.parse.quote(handle)}", fresh=fresh)
        pd = data.get("props", {}).get("pageProps", {}).get("productData")
        if not pd or not pd.get("product"):
            raise NotFound(f"no product data for '{handle}'")
        product = normalise(pd["product"])
        product["description"] = html_to_text(pd["product"].get("description") or "")
        product["season"] = pd["product"].get("season")
        product["will_restock"] = pd["product"].get("willRestock")
        product["colourways"] = [normalise(v) for v in pd.get("variants") or []]
        product["get_the_look"] = [normalise(v) for v in pd.get("getTheLookProducts") or []]
        stats = (pd.get("reviews") or {}).get("statistics") or {}
        product["reviews"] = {
            "average": stats.get("averageRating"),
            "count": stats.get("totalReviews"),
            "recommended": stats.get("recommendedCount"),
            "not_recommended": stats.get("notRecommendedCount"),
            "distribution": {
                d["ratingValue"]: d["count"] for d in stats.get("ratingDistribution") or [] if "ratingValue" in d
            },
            "secondary": {
                s.get("label"): s.get("averageRating")
                for s in stats.get("secondaryRatingsAverages") or []
                if s.get("label") and s.get("averageRating") is not None
            },
        }
        return product


def normalise(h: dict) -> dict:
    """Flatten a Gymshark product/hit record into the fields the CLI uses."""
    sizes = [
        {
            "size": s.get("size"),
            "in_stock": bool(s.get("inStock")),
            "quantity": s.get("inventoryQuantity"),
            "price": s.get("price"),
            "sku": s.get("sku"),
        }
        for s in h.get("availableSizes") or []
    ]
    price = h.get("price")
    compare = h.get("compareAtPrice")
    discount = h.get("discountPercentage") or 0
    if not discount and compare and price and compare > price:
        discount = round(100 * (compare - price) / compare)
    rating = h.get("rating") or {}
    handle = h.get("handle")
    return {
        "title": h.get("title"),
        "colour": h.get("colour"),
        "handle": handle,
        "url": f"{DEFAULT_BASE_URL}/products/{handle}" if handle else None,
        "price": price,
        "compare_at_price": compare,
        "discount": discount,
        "currency": h.get("currencyCode") or "USD",
        "in_stock": bool(h.get("inStock")),
        "sizes": sizes,
        "type": h.get("type"),
        "fit": h.get("fit"),
        "range": h.get("range"),
        "features": h.get("features") or [],
        "activities": h.get("activities") or [],
        "gender": h.get("gender") or [],
        "rating": rating.get("average"),
        "rating_count": rating.get("count"),
        "image": (h.get("featuredMedia") or {}).get("src"),
        "sku": h.get("sku"),
        "colours": [c.get("colourName") for c in h.get("availableColours") or [] if c.get("colourName")],
        "is_new": bool(h.get("isNewRelease")),
    }


def html_to_text(html: str) -> str:
    html = re.sub(r"</(p|li|h\d)>|<br\s*/?>", "\n", html, flags=re.I)
    html = re.sub(r"<li[^>]*>", "- ", html, flags=re.I)
    text = re.sub(r"<[^>]+>", "", html)
    import html as _html

    text = _html.unescape(text).replace("\xa0", " ")
    return "\n".join(line.strip() for line in text.splitlines() if line.strip())
