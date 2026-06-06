from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import os


ROOT = Path(__file__).resolve().parent
PUBLIC_DIR = ROOT / "public"
INDEX = PUBLIC_DIR / "index.html"


class SeerSpaHandler(SimpleHTTPRequestHandler):
    def translate_path(self, path):
        # Serve from public directory
        path = path.lstrip("/")
        translated = PUBLIC_DIR / path

        if translated.exists() and translated.is_file():
            return str(translated)

        # SPA routing: if no file extension and doesn't exist, serve index.html
        if "." not in Path(path).name:
            return str(INDEX)

        return str(translated)

    def log_message(self, format, *args):
        # Cleaner logging
        print(format % args)


if __name__ == "__main__":
    os.chdir(PUBLIC_DIR)
    server = ThreadingHTTPServer(("", 8088), SeerSpaHandler)
    print("🚀 Serving Seer at http://localhost:8088/")
    print("📁 Serving from:", PUBLIC_DIR)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n✋ Server stopped")
        server.shutdown()
