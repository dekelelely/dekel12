"""`shealth serve` — a local sleep dashboard in your browser.

Standard library only. Binds to 127.0.0.1 by default so your health data is
not exposed to your network. No external scripts/fonts are loaded; the page
works fully offline.
"""

from __future__ import annotations

import json
import webbrowser
from datetime import date, timedelta
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from importlib import resources
from pathlib import Path
from urllib.parse import parse_qs, urlparse

from . import metrics
from .db import NoDataError, connect
from .sleepdata import SleepData


def _page() -> bytes:
    return resources.files("shealth").joinpath("web/index.html").read_bytes()


def make_handler(db_path: Path):
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, fmt, *args):  # keep the terminal quiet
            pass

        def _send(self, code: int, body: bytes, ctype: str):
            self.send_response(code)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(body)

        def _json(self, obj, code=200):
            self._send(code, json.dumps(obj, default=str).encode(), "application/json")

        def do_GET(self):
            url = urlparse(self.path)
            q = {k: v[0] for k, v in parse_qs(url.query).items()}
            try:
                if url.path in ("/", "/index.html"):
                    return self._send(200, _page(), "text/html; charset=utf-8")
                conn = connect(db_path, readonly=True)
                try:
                    sd = SleepData(conn)
                    if url.path == "/api/nights":
                        end = date.fromisoformat(q["to"]) if q.get("to") else (
                            sd.latest_night() or metrics.latest_date(conn) or date.today())
                        days = max(1, min(int(q.get("days", 30)), 3650))
                        start = end - timedelta(days=days - 1)
                        return self._json({"from": start, "to": end, "nights": sd.nights(start, end)})
                    if url.path == "/api/night":
                        d = date.fromisoformat(q["date"]) if q.get("date") else None
                        return self._json(sd.night(d))
                finally:
                    conn.close()
                self._json({"error": "not found"}, 404)
            except NoDataError as e:
                self._json({"error": str(e)}, 404)
            except (ValueError, KeyError) as e:
                self._json({"error": f"bad request: {e}"}, 400)

    return Handler


def serve(db_path: Path, host: str = "127.0.0.1", port: int = 8765, open_browser: bool = True):
    connect(db_path, readonly=True).close()  # fail fast if there's no DB
    httpd = ThreadingHTTPServer((host, port), make_handler(db_path))
    url = f"http://{host}:{httpd.server_address[1]}/"
    print(f"Sleep dashboard running at {url}  (Ctrl+C to stop)")
    if host not in ("127.0.0.1", "localhost"):
        print("WARNING: listening beyond this computer — anyone on your network can see your health data.")
    if open_browser:
        webbrowser.open(url)
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nStopped.")
    finally:
        httpd.server_close()
