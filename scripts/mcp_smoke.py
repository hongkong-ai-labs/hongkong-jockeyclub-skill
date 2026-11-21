#!/usr/bin/env python3
"""Minimal MCP stdio handshake and tool-call smoke test."""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BIN = os.environ.get("HKJC_MCP_BIN", str(ROOT / "target" / "release" / "hkjc-mcp"))


class Client:
    def __init__(self) -> None:
        self.process = subprocess.Popen(
            [BIN],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            bufsize=1,
        )
        self.next_id = 1

    def send(self, method: str, params: dict | None = None) -> dict:
        request_id = self.next_id
        self.next_id += 1
        payload = {"jsonrpc": "2.0", "id": request_id, "method": method}
        if params is not None:
            payload["params"] = params
        assert self.process.stdin is not None
        self.process.stdin.write(json.dumps(payload) + "\n")
        self.process.stdin.flush()

        assert self.process.stdout is not None
        while True:
            line = self.process.stdout.readline()
            if not line:
                raise RuntimeError("MCP server closed stdout")
            line = line.strip()
            if not line:
                continue
            message = json.loads(line)
            if message.get("id") == request_id:
                if "error" in message:
                    raise RuntimeError(message["error"])
                return message.get("result", {})

    def notify(self, method: str, params: dict | None = None) -> None:
        payload = {"jsonrpc": "2.0", "method": method}
        if params is not None:
            payload["params"] = params
        assert self.process.stdin is not None
        self.process.stdin.write(json.dumps(payload) + "\n")
        self.process.stdin.flush()

    def close(self) -> None:
        if self.process.stdin:
            self.process.stdin.close()
        self.process.terminate()
        self.process.wait(timeout=5)


def tool_text(result: dict) -> dict | list:
    content = result.get("content", [])
    text = next((item.get("text") for item in content if item.get("type") == "text"), None)
    if text is None:
        raise AssertionError("tool returned no text content")
    return json.loads(text)


def main() -> int:
    if not Path(BIN).is_file():
        print(f"Binary not found: {BIN}. Run cargo build --release.", file=sys.stderr)
        return 1

    client = Client()
    try:
        initialized = client.send(
            "initialize",
            {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "hkjc-smoke", "version": "1.0"},
            },
        )
        print(
            "Initialized: "
            f"{initialized['serverInfo']['name']} {initialized['serverInfo']['version']}"
        )
        client.notify("notifications/initialized")

        listed = client.send("tools/list", {})
        names = [tool["name"] for tool in listed["tools"]]
        print(f"Tools exposed ({len(names)}): {', '.join(names)}")
        expected = {
            "hk_race_meetings",
            "hk_race_card",
            "hk_race_odds",
            "hk_race_results",
            "hk_race_fixtures",
            "hk_race_profile",
            "hk_race_lookup",
            "hk_football_matches",
            "hk_football_match",
            "hk_marksix_results",
            "hk_marksix_latest",
            "hk_marksix_stats",
        }
        missing = expected - set(names)
        if missing:
            raise AssertionError(f"missing tools: {sorted(missing)}")

        meetings = client.send("tools/call", {"name": "hk_race_meetings", "arguments": {}})
        data = tool_text(meetings)
        if not isinstance(data, list) or not data:
            raise AssertionError("expected at least one race meeting")
        print(f"hk_race_meetings OK: {len(data)} meeting(s), {len(data[0]['races'])} races")

        fixtures = client.send(
            "tools/call", {"name": "hk_race_fixtures", "arguments": {}}
        )
        fixture_data = tool_text(fixtures)
        if not isinstance(fixture_data, list) or not fixture_data:
            raise AssertionError("expected at least one fixture")
        print(f"hk_race_fixtures OK: {len(fixture_data)} meeting(s)")

        lookup = client.send(
            "tools/call", {"name": "hk_race_lookup", "arguments": {"query": "Purton"}}
        )
        hits = tool_text(lookup)
        if not hits or not hits[0].get("id"):
            raise AssertionError("expected a jockey lookup hit")
        profile = client.send(
            "tools/call",
            {
                "name": "hk_race_profile",
                "arguments": {"kind": hits[0]["kind"], "id": hits[0]["id"]},
            },
        )
        profile_data = tool_text(profile)
        if not profile_data.get("name") or not profile_data.get("stats"):
            raise AssertionError("expected a parsed profile with statistics")
        print(
            f"hk_race_profile OK: {profile_data['name']}, "
            f"{len(profile_data['stats'])} stats"
        )

        football = client.send(
            "tools/call", {"name": "hk_football_matches", "arguments": {"limit": 5}}
        )
        matches = tool_text(football)
        if not isinstance(matches, list) or not matches:
            raise AssertionError("expected at least one football match")
        board = client.send(
            "tools/call",
            {"name": "hk_football_match", "arguments": {"id": matches[0]["id"]}},
        )
        board_data = tool_text(board)
        if not board_data.get("odds"):
            raise AssertionError("expected an odds board for the football match")
        print(
            f"hk_football_matches OK: {len(matches)} matches; "
            f"hk_football_match OK: {len(board_data['odds'])} pools"
        )

        latest = client.send("tools/call", {"name": "hk_marksix_latest", "arguments": {}})
        draw = tool_text(latest)
        if len(draw.get("numbers", [])) != 6:
            raise AssertionError("expected six Mark Six numbers")
        print(f"hk_marksix_latest OK: draw {draw['draw_no']} {draw['numbers']}")

        stats = client.send(
            "tools/call", {"name": "hk_marksix_stats", "arguments": {"draws": 20}}
        )
        stats_data = tool_text(stats)
        if len(stats_data.get("numbers", [])) != 49:
            raise AssertionError("expected 49 number statistics")
        print(
            f"hk_marksix_stats OK: {stats_data['draws_analysed']} draws, "
            f"odd {stats_data['odd_even']['odd']} / even {stats_data['odd_even']['even']}"
        )

        invalid = client.send(
            "tools/call",
            {
                "name": "hk_race_results",
                "arguments": {"date": "2020-01-01", "venue": "HV", "race": 1},
            },
        )
        if not invalid.get("isError"):
            raise AssertionError("a non-race day should produce a tool error")
        print("non-race day returns a tool error OK")
        print("MCP smoke test passed.")
        return 0
    finally:
        client.close()


if __name__ == "__main__":
    raise SystemExit(main())
