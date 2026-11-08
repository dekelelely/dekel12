"""gymshark: browse Gymshark's storefront from the terminal."""

from __future__ import annotations

import argparse
import json
import shutil
import sys
import time

from . import __version__
from .client import GENDERS, Client, GymsharkError

EXIT_OK, EXIT_OUT_OF_STOCK, EXIT_ERROR = 0, 1, 2
MIN_WATCH_SECONDS = 300
SORTS = {
    "price": (lambda p: p["price"] or 0, False),
    "-price": (lambda p: p["price"] or 0, True),
    "discount": (lambda p: p["discount"] or 0, True),
    "rating": (lambda p: weighted_rating(p), True),
    "reviews": (lambda p: p["rating_count"] or 0, True),
}


# ---- filtering / formatting ------------------------------------------------

PRIOR_RATING, PRIOR_WEIGHT = 4.0, 20


def weighted_rating(p: dict) -> float:
    """Average rating pulled toward 4.0 until there are enough reviews.

    A 5.0 from 1 review shouldn't outrank a 4.6 from 900 reviews.
    """
    n = p["rating_count"] or 0
    avg = p["rating"] or 0
    return (avg * n + PRIOR_RATING * PRIOR_WEIGHT) / (n + PRIOR_WEIGHT) if n else 0.0


def norm_size(s: str) -> str:
    return s.strip().lower()


def matches(p: dict, args) -> bool:
    if getattr(args, "in_stock", False) and not p["in_stock"]:
        return False
    if getattr(args, "size", None):
        want = norm_size(args.size)
        if not any(norm_size(s["size"] or "") == want and s["in_stock"] for s in p["sizes"]):
            return False
    if getattr(args, "colour", None) and args.colour.lower() not in (p["colour"] or "").lower():
        return False
    price = p["price"] or 0
    if getattr(args, "max_price", None) is not None and price > args.max_price:
        return False
    if getattr(args, "min_price", None) is not None and price < args.min_price:
        return False
    if getattr(args, "sale", False) and not p["discount"]:
        return False
    if getattr(args, "min_rating", None) is not None and (p["rating"] or 0) < args.min_rating:
        return False
    terms = getattr(args, "terms", None)
    if terms:
        hay = " ".join(
            str(x)
            for x in [p["title"], p["colour"], p["type"], p["fit"], p["range"], *p["features"], *p["activities"]]
            if x
        ).lower()
        if not all(t.lower() in hay for t in terms):
            return False
    return True


def sort_products(products: list[dict], key: str | None) -> list[dict]:
    if not key or key == "relevance":
        return products
    fn, rev = SORTS[key]
    return sorted(products, key=fn, reverse=rev)


def money(v, currency="USD") -> str:
    if v is None:
        return "-"
    sym = {"USD": "$", "GBP": "£", "EUR": "€"}.get(currency, currency + " ")
    return f"{sym}{v:,.2f}".replace(".00", "")


def price_label(p: dict) -> str:
    s = money(p["price"], p["currency"])
    if p["discount"] and p["compare_at_price"]:
        s += f" (was {money(p['compare_at_price'], p['currency'])}, -{p['discount']}%)"
    return s


def sizes_label(p: dict) -> str:
    if not p["sizes"]:
        return "in stock" if p["in_stock"] else "SOLD OUT"
    have = [s["size"].upper() for s in p["sizes"] if s["in_stock"] and s["size"]]
    return " ".join(have) if have else "SOLD OUT"


def rating_label(p: dict) -> str:
    if not p["rating"]:
        return ""
    return f"{p['rating']:.1f}★ ({p['rating_count']})"


def clip(s: str, n: int) -> str:
    s = s or ""
    return s if len(s) <= n else s[: max(n - 1, 0)] + "…"


def print_table(products: list[dict], show_handle: bool = True) -> None:
    width = shutil.get_terminal_size((120, 20)).columns
    rows = [
        (f"{p['title']} — {p['colour']}" if p["colour"] else p["title"], price_label(p), sizes_label(p), rating_label(p), p["handle"] or "")
        for p in products
    ]
    if not rows:
        return
    pw = max(len(r[1]) for r in rows)
    rw = max(len(r[3]) for r in rows)
    sw = min(max(len(r[2]) for r in rows), 28)
    fixed = pw + rw + sw + 6
    tw = max(20, min(60, width - fixed))
    for name, price, sizes, rating, handle in rows:
        line = f"{clip(name, tw):<{tw}}  {price:>{pw}}  {clip(sizes, sw):<{sw}}  {rating:<{rw}}"
        print(line.rstrip())
        if show_handle:
            print(f"    {handle}")


def emit(args, payload, human) -> None:
    if args.json:
        json.dump(payload, sys.stdout, indent=2, ensure_ascii=False)
        print()
    else:
        human()


def progress(args, msg: str) -> None:
    if not args.json and not args.quiet:
        print(msg, file=sys.stderr)


# ---- commands --------------------------------------------------------------


def gather(client: Client, args, slug: str, gender: str | None) -> tuple[list[dict], object, int]:
    """Fetch pages, filter, sort. Returns (shown, first_page, total_matches)."""
    start = max(args.page - 1, 0)
    max_pages = None if args.all else args.pages
    out, first = [], None
    for page in client.iter_collection(slug, gender, start=start, max_pages=max_pages, fresh=args.fresh):
        first = first or page
        if page.pages > 1 and (args.all or args.pages > 1):
            progress(args, f"  page {page.page + 1}/{page.pages}…")
        out.extend(p for p in page.products if matches(p, args))
    out = sort_products(out, args.sort)
    matched = len(out)
    if args.limit:
        out = out[: args.limit]
    return out, first, matched


def cmd_collection(client, args) -> int:
    products, page, matched = gather(client, args, args.slug, args.gender)
    if args.all:
        scanned = "all pages"
    elif args.pages == 1:
        scanned = f"page {args.page}"
    else:
        scanned = f"pages {args.page}-{args.page + args.pages - 1}"

    def human():
        g = f" ({page.gender})" if page.gender else ""
        shown = f"{matched} match on {scanned}" + (f", showing {len(products)}" if len(products) < matched else "")
        print(f"{page.title}{g}: {page.total} products over {page.pages} pages; {shown}")
        print()
        print_table(products)
        if not args.all and page.pages > args.page + args.pages - 1:
            print(f"\nMore: --page {args.page + args.pages} or --all", file=sys.stderr)

    emit(args, {"collection": args.slug, "gender": args.gender, "total": page.total, "pages": page.pages, "matches": matched, "products": products}, human)
    return EXIT_OK


def cmd_deals(client, args) -> int:
    args.slug = "last-chance"
    if args.pages == 1 and args.page == 1:
        args.all = True  # sorting by discount only means something across the whole rack
    if not args.sort:
        args.sort = "discount"
    args.sale = True
    return cmd_collection(client, args)


def cmd_find(client, args) -> int:
    if args.pages == 1 and not args.all:
        args.all = True  # a keyword search is useless over a single page
    products, page, matched = gather(client, args, args.within, args.gender)

    def human():
        more = f", showing {len(products)}" if len(products) < matched else ""
        print(f"{matched} match(es){more} for {' '.join(args.terms)!r} in {args.within}" + (f"/{args.gender}" if args.gender else ""))
        print()
        print_table(products)

    emit(args, {"query": args.terms, "collection": args.within, "matches": matched, "products": products}, human)
    return EXIT_OK


def cmd_categories(client, args) -> int:
    page = client.collection("all-products", args.gender, fresh=args.fresh)

    def human():
        print(f"Categories ({args.gender or 'all'}) — use with: gymshark collection <handle>" + (f" {args.gender}" if args.gender else ""))
        print()
        w = max((len(c["name"]) for c in page.categories), default=0)
        for c in page.categories:
            print(f"  {c['name']:<{w}}  {c['handle']}")

    emit(args, page.categories, human)
    return EXIT_OK


def cmd_product(client, args) -> int:
    p = client.product(args.product, fresh=args.fresh)

    def human():
        print(f"{p['title']} — {p['colour']}")
        print(p["url"])
        print()
        print(f"Price:    {price_label(p)}")
        bits = [b for b in [p["fit"], p["range"] and f"{p['range']} range", p["season"]] if b]
        if bits:
            print(f"Details:  {', '.join(bits)}")
        if p["features"]:
            print(f"Features: {', '.join(p['features'])}")
        r = p["reviews"]
        if r.get("count"):
            rec = ""
            if r.get("recommended") is not None and r.get("not_recommended") is not None:
                tot = r["recommended"] + r["not_recommended"]
                if tot:
                    rec = f", {round(100 * r['recommended'] / tot)}% would recommend"
            print(f"Rating:   {r['average']:.2f}/5 from {r['count']} reviews{rec}")
            dist = r.get("distribution") or {}
            top = max(dist.values(), default=0) or 1
            for star in (5, 4, 3, 2, 1):
                n = dist.get(star, 0)
                print(f"          {star}★ {'█' * round(20 * n / top):<20} {n}")
            for label, avg in (r.get("secondary") or {}).items():
                print(f"          {label}: {avg:.1f}/5")
        print()
        print("Stock (this colour):")
        print_sizes(p)
        others = [c for c in p["colourways"] if c["handle"] != p["handle"]]
        if others:
            print()
            print("Other colours:")
            print_table(others)
        if args.description and p["description"]:
            print()
            print(p["description"])
        if p["get_the_look"]:
            print()
            print("Get the look:")
            print_table(p["get_the_look"])

    emit(args, p, human)
    return EXIT_OK


def print_sizes(p: dict) -> None:
    for s in p["sizes"]:
        if s["in_stock"]:
            q = s["quantity"]
            note = "low stock" if q is not None and q < 20 else "in stock"
        else:
            note = "sold out" + (" (restocking)" if p.get("will_restock") else "")
        print(f"  {(s['size'] or '?').upper():<6} {note}")


def stock_status(p: dict, size: str | None) -> tuple[bool, str]:
    if size:
        want = norm_size(size)
        for s in p["sizes"]:
            if norm_size(s["size"] or "") == want:
                return s["in_stock"], f"size {want.upper()} {'IN STOCK' if s['in_stock'] else 'sold out'}"
        avail = ", ".join((s["size"] or "?").upper() for s in p["sizes"])
        raise GymsharkError(f"size '{size}' doesn't exist for this product (sizes: {avail})")
    return p["in_stock"], "IN STOCK" if p["in_stock"] else "sold out"


def cmd_stock(client, args) -> int:
    if args.watch is not None and args.watch < MIN_WATCH_SECONDS:
        raise GymsharkError(f"--watch must be at least {MIN_WATCH_SECONDS} seconds, to be polite to the site")
    fresh = args.fresh or args.watch is not None
    while True:
        p = client.product(args.product, fresh=fresh)
        ok, label = stock_status(p, args.size)
        stamp = time.strftime("%H:%M:%S")
        if args.json:
            emit(args, {"handle": p["handle"], "size": args.size, "in_stock": ok, "price": p["price"], "sizes": p["sizes"]}, None)
        else:
            print(f"[{stamp}] {p['title']} — {p['colour']}: {label} at {price_label(p)}")
            if not args.size and not args.quiet:
                print_sizes(p)
        if ok or args.watch is None:
            if ok and args.watch is not None and not args.json:
                print(f"\a{p['url']}")
            return EXIT_OK if ok else EXIT_OUT_OF_STOCK
        time.sleep(args.watch)


# ---- argument parsing ------------------------------------------------------


def add_listing_opts(sp, default_sort=None):
    g = sp.add_argument_group("filters")
    g.add_argument("--size", help="only items with this size in stock (xs, s, m, l, xl, xxl, 3xl, 28, 30…)")
    g.add_argument("--colour", "--color", dest="colour", help="colour contains this text, e.g. black")
    g.add_argument("--min-price", type=float)
    g.add_argument("--max-price", type=float)
    g.add_argument("--min-rating", type=float, help="e.g. 4.5")
    g.add_argument("--sale", action="store_true", help="only discounted items")
    g.add_argument("--in-stock", action="store_true", help="hide sold-out items")
    o = sp.add_argument_group("paging & output")
    o.add_argument("--sort", choices=["relevance", *SORTS], default=default_sort)
    o.add_argument("--page", type=int, default=1, help="start page (1-based)")
    o.add_argument("--pages", type=int, default=1, help="how many pages to scan (60 items each)")
    o.add_argument("--all", action="store_true", help="scan every page")
    o.add_argument("--limit", type=int, help="show at most N results")


def build_parser() -> argparse.ArgumentParser:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--json", action="store_true", help="machine-readable output")
    common.add_argument("--fresh", action="store_true", help="ignore the local cache")
    common.add_argument("-q", "--quiet", action="store_true")

    ap = argparse.ArgumentParser(
        prog="gymshark",
        description="Browse Gymshark's public storefront from the terminal. Read-only; no account or API key.",
    )
    ap.add_argument("--version", action="version", version=f"%(prog)s {__version__}")
    ap.add_argument("--no-cache", action="store_true", help="don't read or write the on-disk cache")
    sub = ap.add_subparsers(dest="cmd", required=True, metavar="COMMAND")

    sp = sub.add_parser("collection", aliases=["ls"], parents=[common], help="list a collection (e.g. shorts mens)")
    sp.add_argument("slug", help="collection handle, e.g. all-products, shorts, leggings")
    sp.add_argument("gender", nargs="?", choices=GENDERS)
    add_listing_opts(sp)
    sp.set_defaults(fn=cmd_collection)

    sp = sub.add_parser("deals", parents=[common], help="discounted items (Last Chance), biggest discount first")
    sp.add_argument("gender", nargs="?", choices=GENDERS)
    add_listing_opts(sp)
    sp.set_defaults(fn=cmd_deals)

    sp = sub.add_parser("find", parents=[common], help="keyword search, e.g. find seamless leggings womens")
    sp.add_argument("terms", nargs="+", help="words that must all appear (title, colour, type, fit, range, features)")
    sp.add_argument("--gender", choices=GENDERS)
    sp.add_argument("--in", dest="within", default="all-products", metavar="COLLECTION", help="collection to search (default all-products)")
    add_listing_opts(sp)
    sp.set_defaults(fn=cmd_find)

    sp = sub.add_parser("categories", parents=[common], help="list collection handles to browse")
    sp.add_argument("gender", nargs="?", choices=GENDERS)
    sp.set_defaults(fn=cmd_categories)

    sp = sub.add_parser("product", aliases=["show"], parents=[common], help="full details for one product")
    sp.add_argument("product", help="product handle or gymshark.com URL")
    sp.add_argument("-d", "--description", action="store_true", help="include the product description")
    sp.set_defaults(fn=cmd_product)

    sp = sub.add_parser(
        "stock",
        parents=[common],
        help="check stock; exit 0 if in stock, 1 if not (good for scripts)",
    )
    sp.add_argument("product", help="product handle or gymshark.com URL")
    sp.add_argument("--size", help="the size you want")
    sp.add_argument("--watch", type=int, metavar="SECONDS", help=f"re-check every N seconds (min {MIN_WATCH_SECONDS}) until in stock")
    sp.set_defaults(fn=cmd_stock)
    return ap


def main(argv=None) -> int:
    ap = build_parser()
    args = ap.parse_args(argv)
    if args.cmd == "find":
        # "find leggings womens" reads naturally; treat a trailing gender word as --gender
        if not args.gender and args.terms and args.terms[-1].lower() in GENDERS and len(args.terms) > 1:
            args.gender = args.terms.pop().lower()
    client = Client(cache_ttl=0 if args.no_cache else 600)
    try:
        return args.fn(client, args)
    except GymsharkError as e:
        print(f"gymshark: {e}", file=sys.stderr)
        return EXIT_ERROR
    except KeyboardInterrupt:
        return 130
    except BrokenPipeError:
        # output piped into `head` etc.; stop quietly
        import os

        os.dup2(os.open(os.devnull, os.O_WRONLY), sys.stdout.fileno())
        return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
