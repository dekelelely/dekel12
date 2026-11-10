import io
import json
import tempfile
import threading
import unittest
import urllib.request
from datetime import date, datetime
from http.server import ThreadingHTTPServer
from pathlib import Path

from shealth import demo, nightview
from shealth.db import connect, import_export
from shealth.server import make_handler
from shealth.sleepdata import SleepData

END = datetime(2026, 3, 15)


class SleepDetail(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        d = Path(cls.tmp.name)
        demo.build(d / "e.zip", days=20, end=END)
        cls.db = d / "h.db"
        import_export(d / "e.zip", cls.db)
        cls.conn = connect(cls.db, readonly=True)
        cls.sd = SleepData(cls.conn)

    @classmethod
    def tearDownClass(cls):
        cls.conn.close()
        cls.tmp.cleanup()

    def test_discovery(self):
        self.assertEqual(set(self.sd.overlays) >= {"heart_rate", "spo2", "respiratory_rate", "skin_temperature"}, True)
        self.assertIn("sleep_snoring", [r["table"] for r in self.sd.related])

    def test_night_has_everything(self):
        n = self.sd.night()
        self.assertEqual(n["date"], "2026-03-15")
        st = n["stages"]
        self.assertAlmostEqual(sum(st["minutes"].values()) / 60, n["hours"], delta=0.05)
        self.assertEqual(sum(st["percent"].values()) in (99, 100, 101), True)
        self.assertGreaterEqual(st["fell_asleep_min"], 4)
        keys = {m["key"] for m in n["measurements"]}
        self.assertTrue({"sleep_score", "sleep_latency", "mental_recovery", "sleep_cycle"} <= keys)
        self.assertFalse(keys & {"datauuid", "start_time", "time_offset"})
        latency = next(m for m in n["measurements"] if m["key"] == "sleep_latency")
        self.assertEqual(latency["pretty"], f'{st["fell_asleep_min"]}m')  # demo: latency == initial awake
        for k in ("heart_rate", "spo2", "respiratory_rate", "skin_temperature"):
            s = n["series"][k]
            self.assertTrue(s["min"] <= s["avg"] <= s["max"])
            for t, _ in s["points"]:  # every point inside the night
                self.assertTrue(n["start"] <= t <= n["end"])

    def test_snoring_matches_raw_rows(self):
        for night in self.sd.nights(date(2026, 3, 1), date(2026, 3, 15)):
            raw = self.conn.execute("SELECT COUNT(*) FROM sleep_snoring WHERE sleep_id = ?",
                                    (night["id"],)).fetchone()[0]
            n = self.sd.night(date.fromisoformat(night["date"]))
            self.assertEqual(n["snoring"]["episodes"] if n["snoring"] else 0, raw)

    def test_unknown_sleep_table_is_picked_up(self):
        with tempfile.TemporaryDirectory() as d:
            folder = Path(d) / "x"
            folder.mkdir()
            P = "com.samsung.health.sleep."
            (folder / "com.samsung.shealth.sleep.20260301000000.csv").write_text(
                "com.samsung.shealth.sleep,1,4\n"
                f"{P}start_time,{P}end_time,{P}time_offset,{P}datauuid,\n"
                "2026-02-28 21:00:00.000,2026-03-01 05:00:00.000,UTC+0100,abc,\n")
            (folder / "com.samsung.shealth.sleep_mystery.20260301000000.csv").write_text(
                "sleep_id,new_metric,\nabc,42,\n")
            import_export(folder, Path(d) / "x.db")
            c = connect(Path(d) / "x.db", readonly=True)
            n = SleepData(c).night()
            self.assertEqual(n["related"][0]["fields"], {"new_metric": 42})
            self.assertEqual(n["stages"]["segments"], [])  # no stage table: no crash
            self.assertEqual(n["series"], {})
            out = io.StringIO()
            nightview.render(n, out)  # renders the "not in your export" hints
            self.assertIn("Not in your export", out.getvalue())
            self.assertIn("new_metric: 42", out.getvalue())
            c.close()

    def test_terminal_render(self):
        nights = self.sd.nights(date(2026, 3, 1), date(2026, 3, 15))
        snored = next(n["date"] for n in nights if n["snore_min"])
        quiet = next(n["date"] for n in nights if n["snore_min"] == 0)
        out = io.StringIO()
        nightview.render(self.sd.night(date.fromisoformat(snored)), out)
        for s in ("Hypnogram", "Deep", "Snore │", "episodes", "Blood oxygen", "Mental recovery"):
            self.assertIn(s, out.getvalue())
        out = io.StringIO()
        nightview.render(self.sd.night(date.fromisoformat(quiet)), out)
        self.assertIn("No snoring detected", out.getvalue())
        self.assertNotIn("Not in your export: snoring", out.getvalue())

    def test_web_api(self):
        httpd = ThreadingHTTPServer(("127.0.0.1", 0), make_handler(self.db))
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        base = f"http://127.0.0.1:{httpd.server_address[1]}"
        try:
            html = urllib.request.urlopen(base + "/").read().decode()
            self.assertIn("<title>Sleep", html)
            nights = json.load(urllib.request.urlopen(base + "/api/nights?days=7"))["nights"]
            self.assertEqual(len(nights), 7)
            n = json.load(urllib.request.urlopen(base + "/api/night?date=2026-03-14"))
            self.assertEqual(n["date"], "2026-03-14")
            with self.assertRaises(urllib.error.HTTPError) as e:
                urllib.request.urlopen(base + "/api/night?date=1999-01-01")
            self.assertEqual(e.exception.code, 404)
        finally:
            httpd.shutdown()
            httpd.server_close()


if __name__ == "__main__":
    unittest.main()
