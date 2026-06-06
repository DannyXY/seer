from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


ROOT = Path(__file__).resolve().parent
INDEX = ROOT / "index.html"


class SeerSpaHandler(SimpleHTTPRequestHandler):
    def translate_path(self, path):
        translated = Path(super().translate_path(path))
        if translated.exists():
            return str(translated)
        if "." not in Path(path).name:
            return str(INDEX)
        return str(translated)


if __name__ == "__main__":
    server = ThreadingHTTPServer(("", 8088), SeerSpaHandler)
    print("Serving Seer at http://localhost:8088/")
    server.serve_forever()
