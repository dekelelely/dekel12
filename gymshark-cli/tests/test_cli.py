"""Offline tests: the fetcher is stubbed with trimmed copies of real pages."""

import io
import json
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from gymshark_cli import cli
from gymshark_cli.client import Client, GymsharkError, NotFound, product_handle

FIX = Path(__file__).parent / "fixtures"
PRODUCT = "gymshark-element-baselayer-t-shirt-black-ss25"


def fake_fetch(url):
    if "/collections/" in url:
        return (FIX / "collection.html").read_text()
    if f"/products/{PRODUCT}" in url:
        return (FIX / "product.html").read_text()
    if "/products/" in url:
        return '<script id="__NEXT_DATA__">{"page": "/404", "props": {}}</script>'
    return "<html>no data</html>"


def client():
    return Client(cache_ttl=0, delay=0, fetcher=fake_fetch)


def run(argv):
    buf = io.StringIO()
    with mock.patch.object(cli, "Client", lambda **kw: client()), redirect_stdout(buf):
        code = cli.main(argv)
    return code, buf.getvalue()


class ClientTests(unittest.TestCase):
    def test_collection_parses_and_normalises(self):
        page = client().collection("all-products", "mens")
        self.assertEqual(page.total, 1307)
        self.assertEqual(page.pages, 17)
        self.assertEqual(len(page.products), 4)
        p = page.products[0]
        self.assertEqual(p["title"], "Element Baselayer T-Shirt")
        self.assertEqual(p["url"], f"https://row.gymshark.com/products/{PRODUCT}")
        self.assertTrue(p["sizes"] and {"size", "in_stock", "quantity"} <= p["sizes"][0].keys())
        self.assertEqual(page.categories[0]["handle"], "t-shirts-tops")

    def test_product_details(self):
        p = client().product(f"https://www.gymshark.com/products/{PRODUCT}?variant=1")
        self.assertEqual(p["handle"], PRODUCT)
        self.assertEqual(p["reviews"]["count"], 137)
        self.assertIn(5, p["reviews"]["distribution"])
        self.assertTrue(p["colourways"])
        self.assertNotIn("<p>", p["description"])

    def test_missing_product(self):
        with self.assertRaises(NotFound):
            client().product("does-not-exist")

    def test_page_without_data(self):
        c = Client(cache_ttl=0, delay=0, fetcher=lambda u: "<html></html>")
        with self.assertRaises(GymsharkError):
            c.collection("x")

    def test_region_selects_store(self):
        seen = []
        c = Client(region="us", cache_ttl=0, delay=0, fetcher=lambda u: seen.append(u) or fake_fetch(u))
        p = c.product(PRODUCT)
        self.assertTrue(seen[0].startswith("https://www.gymshark.com/"))
        self.assertTrue(p["url"].startswith("https://www.gymshark.com/"))
        self.assertEqual(client().region, "row")
        with self.assertRaises(GymsharkError):
            Client(region="mars")

    def test_bad_gender(self):
        with self.assertRaises(GymsharkError):
            client().collection("shorts", "kids")

    def test_handle_parsing(self):
        self.assertEqual(product_handle("abc"), "abc")
        self.assertEqual(product_handle("https://www.gymshark.com/products/abc-def?x=1"), "abc-def")
        self.assertEqual(product_handle("/products/abc#reviews"), "abc")
        with self.assertRaises(GymsharkError):
            product_handle("https://www.gymshark.com/collections/shorts")

    def test_discount_derived_from_compare_price(self):
        from gymshark_cli.client import normalise

        self.assertEqual(normalise({"price": 30, "compareAtPrice": 40})["discount"], 25)


class FilterSortTests(unittest.TestCase):
    def prod(self, **kw):
        base = dict(title="T", colour="Black", type="", fit="", range="", features=[], activities=[],
                    price=20, discount=0, in_stock=True, rating=4.0, rating_count=10,
                    sizes=[{"size": "m", "in_stock": True}, {"size": "l", "in_stock": False}])
        base.update(kw)
        return base

    def args(self, **kw):
        return SimpleNamespace(**kw)

    def test_size_filter_requires_stock(self):
        self.assertTrue(cli.matches(self.prod(), self.args(size="M")))
        self.assertFalse(cli.matches(self.prod(), self.args(size="l")))

    def test_terms_must_all_match(self):
        p = self.prod(title="Vital Seamless Leggings")
        self.assertTrue(cli.matches(p, self.args(terms=["seamless", "LEGGINGS"])))
        self.assertFalse(cli.matches(p, self.args(terms=["seamless", "shorts"])))

    def test_terms_tolerate_typos(self):
        p = self.prod(title="Vengeance T-Shirt")
        self.assertTrue(cli.matches(p, self.args(terms=["vengence"])))
        self.assertFalse(cli.matches(p, self.args(terms=["vintage"])))
        self.assertFalse(cli.matches(self.prod(title="Crest Tee"), self.args(terms=["vest"])))

    def test_price_and_sale(self):
        p = self.prod(price=50, discount=20)
        self.assertFalse(cli.matches(p, self.args(max_price=40)))
        self.assertTrue(cli.matches(p, self.args(sale=True, min_price=10)))

    def test_rating_sort_discounts_tiny_sample(self):
        few = self.prod(title="few", rating=5.0, rating_count=1)
        many = self.prod(title="many", rating=4.6, rating_count=900)
        self.assertEqual([p["title"] for p in cli.sort_products([few, many], "rating")], ["many", "few"])


class CommandTests(unittest.TestCase):
    def test_collection_json(self):
        code, out = run(["ls", "all-products", "mens", "--json", "--limit", "2"])
        self.assertEqual(code, 0)
        data = json.loads(out)
        self.assertEqual(len(data["products"]), 2)
        self.assertEqual(data["matches"], 4)

    def test_stock_exit_codes(self):
        code, out = run(["stock", PRODUCT, "--size", "m"])
        self.assertEqual(code, cli.EXIT_OK)
        self.assertIn("IN STOCK", out)
        code, _ = run(["stock", PRODUCT, "--size", "9xl"])
        self.assertEqual(code, cli.EXIT_ERROR)

    def test_stock_out_of_stock(self):
        p = client().product(PRODUCT)
        for s in p["sizes"]:
            s["in_stock"] = False
        ok, label = cli.stock_status(p, "m")
        self.assertFalse(ok)
        self.assertIn("sold out", label)

    def test_watch_minimum(self):
        code, _ = run(["stock", PRODUCT, "--watch", "5"])
        self.assertEqual(code, cli.EXIT_ERROR)

    def test_product_human_output(self):
        code, out = run(["product", PRODUCT])
        self.assertEqual(code, 0)
        self.assertIn("Element Baselayer T-Shirt", out)
        self.assertIn("Other colours:", out)

    def test_find_trailing_gender(self):
        code, out = run(["find", "baselayer", "mens", "--json"])
        self.assertEqual(code, 0)
        self.assertGreaterEqual(json.loads(out)["matches"], 1)


if __name__ == "__main__":
    unittest.main()
