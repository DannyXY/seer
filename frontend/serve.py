from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import os


ROOT = Path(__file__).resolve().parent
PUBLIC_DIR = ROOT / "public"
INDEX = PUBLIC_DIR / "index.html"


class SeerSpaHandler(SimpleHTTPRequestHandler):
    def translate_path(self, path):
        # Remove leading slash
        path = path.lstrip("/")

        # First check if file exists in the frontend root (where public/ and src/ are)
        full_path = ROOT / path
        if full_path.exists() and full_path.is_file():
            return str(full_path)

        # Check in public directory for special files
        public_path = PUBLIC_DIR / path
        if public_path.exists() and public_path.is_file():
            return str(public_path)

        # SPA routing: if no file extension and doesn't exist, serve index.html
        if "." not in Path(path).name:
            return str(INDEX)

        return str(full_path)

    def log_message(self, format, *args):
        # Cleaner logging
        print(format % args)


if __name__ == "__main__":
    os.chdir(ROOT)
    server = ThreadingHTTPServer(("", 8088), SeerSpaHandler)
    print("🚀 Serving Seer at http://localhost:8088/")
    print("📁 Serving from:", ROOT)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n✋ Server stopped")
        server.shutdown()
