"""Serves web/dist for local testing: python3 web/serve.py [port], or PORT.

The standard library server listens with a backlog of 5, which drops some of
the page's parallel module requests; this one allows 64.
"""

import functools
import http.server
import os
import pathlib
import sys


class Server(http.server.ThreadingHTTPServer):
    request_queue_size = 64


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm"}


def main() -> None:
    port = int(sys.argv[1] if len(sys.argv) > 1 else os.environ.get("PORT", "8000"))
    root = pathlib.Path(__file__).resolve().parent / "dist"
    handler = functools.partial(Handler, directory=str(root))
    with Server(("127.0.0.1", port), handler) as server:
        print(f"Serving {root} at http://127.0.0.1:{port}/")
        server.serve_forever()


if __name__ == "__main__":
    main()
