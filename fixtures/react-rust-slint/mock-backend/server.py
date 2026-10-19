#!/usr/bin/env python3
"""
Shared Mock Backend Server (`127.0.0.1:4010`) for MAE v0.1 React <-> Rust + Slint Parity Evaluation.

Both the React web application (`http://127.0.0.1:4173`) and the migrated native Ubuntu Rust + Slint
application communicate with this single shared mock backend instance over HTTP JSON.
"""

from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import sys
import threading

LOCK = threading.Lock()

BACKEND_STATE = {
    "sync_revision": 1,
    "active_sprint": "Sprint 14",
    "completion_percent": 78,
    "velocity_score": 92,
    "tasks": [
        {"id": "task-1", "title": "Verify native Slint UI roundness (8px) & scale (14px)", "priority": "high", "done": False},
        {"id": "task-2", "title": "Preserve migration contracts & fresh evidence", "priority": "critical", "done": False},
        {"id": "task-3", "title": "Sync React & Rust with shared Mock Backend", "priority": "high", "done": True},
    ],
    "kanban": [
        {"id": "card-101", "title": "Stage 1: Token & Timing Eval Metrics", "column": "done"},
        {"id": "card-102", "title": "Stage 2: Shared Mock Backend HTTP Sync", "column": "in_progress"},
        {"id": "card-103", "title": "Stage 3: Ubuntu .deb Package & Live Comparison", "column": "backlog"},
    ],
    "notifications": [
        {"id": "notif-1", "text": "Contract contract.native_compliance passed on r4", "read": False},
        {"id": "notif-2", "text": "VisualParityVerifier verified border-radius: 8px", "read": False},
        {"id": "notif-3", "text": "Shared Mock Backend online at 127.0.0.1:4010", "read": False},
    ],
    "settings": {
        "wip_limit": 5,
        "compact_mode": False,
        "notifications_muted": False,
        "theme_mode": "dark-slate",
        "time_range": "30d",
    },
}


class MockBackendHandler(BaseHTTPRequestHandler):
    def _send_cors_headers(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type, Accept")

    def _send_json(self, status_code: int, payload: dict):
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status_code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self._send_cors_headers()
        self.end_headers()
        self.wfile.write(body)

    def do_OPTIONS(self):
        self.send_response(204)
        self._send_cors_headers()
        self.end_headers()

    def do_GET(self):
        with LOCK:
            if self.path.startswith("/api/state") or self.path.startswith("/api/tasks"):
                self._send_json(200, BACKEND_STATE)
                return
            if self.path.startswith("/api/health"):
                self._send_json(200, {"status": "ok", "sync_revision": BACKEND_STATE["sync_revision"]})
                return
        self._send_json(404, {"error": f"Unknown endpoint {self.path}"})

    def do_POST(self):
        content_len = int(self.headers.get("Content-Length", "0"))
        raw_body = self.rfile.read(content_len).decode("utf-8") if content_len > 0 else "{}"
        try:
            data = json.loads(raw_body) if raw_body.strip() else {}
        except Exception:
            data = {}

        with LOCK:
            if self.path.startswith("/api/tasks"):
                action = data.get("action", "create")
                if action == "complete_all":
                    for t in BACKEND_STATE["tasks"]:
                        t["done"] = True
                    BACKEND_STATE["completion_percent"] = 100
                elif action == "clear_completed":
                    BACKEND_STATE["tasks"] = [t for t in BACKEND_STATE["tasks"] if not t.get("done")]
                else:
                    title = str(data.get("title", "")).strip() or f"Desktop Task #{len(BACKEND_STATE['tasks']) + 1}"
                    priority = str(data.get("priority", "high"))
                    new_id = f"task-{len(BACKEND_STATE['tasks']) + 1}"
                    BACKEND_STATE["tasks"].append(
                        {"id": new_id, "title": title, "priority": priority, "done": False}
                    )
                BACKEND_STATE["sync_revision"] += 1
                self._send_json(200, BACKEND_STATE)
                return

            if self.path.startswith("/api/kanban/move"):
                card_id = data.get("card_id", "card-103")
                column = data.get("column", "in_progress")
                moved = False
                for card in BACKEND_STATE["kanban"]:
                    if card["id"] == card_id:
                        card["column"] = column
                        moved = True
                        break
                if not moved and BACKEND_STATE["kanban"]:
                    BACKEND_STATE["kanban"][0]["column"] = column
                BACKEND_STATE["sync_revision"] += 1
                self._send_json(200, BACKEND_STATE)
                return

            if self.path.startswith("/api/notifications/read-all"):
                for n in BACKEND_STATE["notifications"]:
                    n["read"] = True
                if "muted" in data:
                    BACKEND_STATE["settings"]["notifications_muted"] = bool(data["muted"])
                BACKEND_STATE["sync_revision"] += 1
                self._send_json(200, BACKEND_STATE)
                return

            if self.path.startswith("/api/settings"):
                for key in ("wip_limit", "compact_mode", "notifications_muted", "theme_mode", "time_range"):
                    if key in data:
                        BACKEND_STATE["settings"][key] = data[key]
                BACKEND_STATE["sync_revision"] += 1
                self._send_json(200, BACKEND_STATE)
                return

        self._send_json(404, {"error": f"Unknown POST endpoint {self.path}"})

    def log_message(self, fmt, *args):
        sys.stderr.write("[mock-backend] " + (fmt % args) + "\n")


def main():
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 4010
    server = HTTPServer(("127.0.0.1", port), MockBackendHandler)
    print(f"MAE Shared Mock Backend listening on http://127.0.0.1:{port}", flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
