import io
import json
import tempfile
import unittest
import zipfile
from contextlib import redirect_stdout
from datetime import date, datetime, timedelta
from pathlib import Path

from shealth import cli, demo, metrics
from shealth.db import connect, import_export, read_table, short_columns, source_name, sql_name
from shealth.timeutil import day_from_daytime, parse_offset, to_local

END = datetime(2026, 3, 15)


class FormatQuirks(unittest.TestCase):
    def test_meta_line_prefixes_and_trailing_comma(self):
        raw = ("com.samsung.shealth.sleep,6302001,4\n"
               "com.samsung.health.sleep.start_time,sleep_score,com.samsung.health.sleep.time_offset,\n"
               "2026-03-01 22:00:00.000,81,UTC+0300,\n")
        cols, rows = read_table(io.StringIO(raw), "com.samsung.shealth.sleep")
        self.assertEqual(cols, ["start_time", "sleep_score", "time_offset"])
        self.assertEqual(list(rows), [["2026-03-01 22:00:00.000", 81, "UTC+0300"]])

    def test_header_without_meta_line(self):
        cols, rows = read_table(io.StringIO("a,b\n1,x\n"), "whatever")
        self.assertEqual(cols, ["a", "b"])
        self.assertEqual(list(rows), [[1, "x"]])

    def test_column_collisions_keep_full_name(self):
        self.assertEqual(short_columns(["com.x.start_time", "start_time"]),
                         ["start_time", "start_time_"])

    def test_names(self):
        self.assertEqual(source_name("a/com.samsung.shealth.sleep.20260301120000.csv"),
                         ("com.samsung.shealth.sleep", "20260301120000"))
        self.assertEqual(sql_name("com.samsung.shealth.tracker.heart_rate"), "tracker_heart_rate")

    def test_time_offset(self):
        self.assertEqual(parse_offset("UTC+0300"), timedelta(hours=3))
        self.assertEqual(parse_offset("UTC-0530"), -timedelta(hours=5, minutes=30))
        # 23:30 UTC at UTC+0200 is 01:30 the *next* local day
        self.assertEqual(to_local("2026-03-01 23:30:00.000", "UTC+0200"), datetime(2026, 3, 2, 1, 30))
        self.assertEqual(day_from_daytime(1772323200000), date(2026, 3, 1))


class EndToEnd(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        d = Path(cls.tmp.name)
        cls.zip = d / "export.zip"
        cls.db = d / "h.db"
        demo.build(cls.zip, days=30, end=END)
        cls.info = import_export(cls.zip, cls.db)
        cls.conn = connect(cls.db, readonly=True)
        cls.start, cls.end = date(2026, 3, 1), date(2026, 3, 14)

    @classmethod
    def tearDownClass(cls):
        cls.conn.close()
        cls.tmp.cleanup()

    def test_export_date_detected(self):
        self.assertEqual(metrics.latest_date(self.conn), date(2026, 3, 15))

    def test_steps_use_merged_total_not_sum(self):
        rows = metrics.steps(self.conn, self.start, self.end)
        self.assertEqual(len(rows), 14)
        # step_daily_trend has a merged row (-2) and a phone row; we must pick the merged one
        merged = {r["day_time"]: r["count"] for r in self.conn.execute(
            "SELECT day_time, count FROM step_daily_trend WHERE source_type = -2")}
        for r in rows:
            self.assertIn(r["steps"], merged.values())

    def test_steps_fallback_to_pedometer_summary(self):
        with tempfile.TemporaryDirectory() as d:
            z = Path(d) / "e.zip"
            demo.build(z, days=30, end=END)  # same args as setUpClass -> same data
            # drop step_daily_trend from the zip
            z2 = Path(d) / "e2.zip"
            with zipfile.ZipFile(z) as src, zipfile.ZipFile(z2, "w") as dst:
                for i in src.infolist():
                    if "step_daily_trend" not in i.filename:
                        dst.writestr(i, src.read(i))
            import_export(z2, Path(d) / "x.db")
            c = connect(Path(d) / "x.db", readonly=True)
            a = metrics.steps(c, date(2026, 3, 10), date(2026, 3, 14))
            b = metrics.steps(self.conn, date(2026, 3, 10), date(2026, 3, 14))
            self.assertEqual([r["steps"] for r in a], [r["steps"] for r in b])
            c.close()

    def test_sleep_stages_add_up(self):
        rows = metrics.sleep(self.conn, self.start, self.end)
        self.assertTrue(rows)
        for r in rows:
            staged = sum(r[k] for k in ("deep_min", "rem_min", "light_min", "awake_min"))
            self.assertAlmostEqual(staged / 60, r["hours"], delta=0.1)

    def test_other_metrics(self):
        hr = metrics.heart_rate(self.conn, self.start, self.end)
        self.assertEqual(len(hr), 14)
        self.assertTrue(all(r["low"] <= r["avg"] <= r["max"] for r in hr))
        self.assertTrue(metrics.stress(self.conn, self.start, self.end))
        self.assertTrue(metrics.weight(self.conn, self.start, self.end))
        w = metrics.workouts(self.conn, self.start, self.end)
        self.assertTrue(all(r["type"] in metrics.EXERCISE_TYPES.values() for r in w))
        self.assertTrue(metrics.spo2(self.conn, self.start, self.end))

    def test_reimport_keeps_newest_copy(self):
        with tempfile.TemporaryDirectory() as d:
            folder = Path(d) / "export"
            folder.mkdir()
            for stamp, steps in (("20260101000000", 111), ("20260301000000", 222)):
                (folder / f"com.samsung.shealth.step_daily_trend.{stamp}.csv").write_text(
                    "com.samsung.shealth.step_daily_trend,1,3\n"
                    f"day_time,count,source_type,\n1772323200000,{steps},-2,\n")
            import_export(folder, Path(d) / "x.db")
            c = connect(Path(d) / "x.db", readonly=True)
            self.assertEqual(metrics.steps(c, date(2026, 3, 1), date(2026, 3, 1))[0]["steps"], 222)
            c.close()

    def _run(self, *argv):
        buf = io.StringIO()
        with redirect_stdout(buf):
            code = cli.main([*argv, "--db", str(self.db)])
        return code, buf.getvalue()

    def test_cli_commands(self):
        for cmd in ("summary", "steps", "sleep", "heart", "stress", "weight", "workouts", "tables"):
            code, out = self._run(cmd)
            self.assertEqual(code, 0, cmd)
            self.assertTrue(out.strip(), cmd)
        code, out = self._run("summary", "--json")
        data = json.loads(out)
        self.assertEqual(len(data["days"]), 7)
        self.assertEqual(data["to"], "2026-03-15")
        code, out = self._run("sql", "SELECT sh_date(start_time, time_offset) d FROM sleep LIMIT 1", "--csv")
        self.assertTrue(out.startswith("d\n"))

    def test_sql_is_read_only(self):
        with self.assertRaises(SystemExit):
            self._run("sql", "DELETE FROM sleep")


if __name__ == "__main__":
    unittest.main()
