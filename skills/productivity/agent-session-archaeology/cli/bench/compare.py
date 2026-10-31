#!/usr/bin/env python3
"""Compare two release binaries using isolated synthetic stores; no real logs."""
import argparse
import json
import os
from pathlib import Path
import statistics
import subprocess
import tempfile
import time


def preserves_existing_fields(before, after):
    if isinstance(before, dict):
        return isinstance(after, dict) and all(
            k in after and preserves_existing_fields(v, after[k]) for k, v in before.items())
    if isinstance(before, list):
        return isinstance(after, list) and len(before) == len(after) and all(
            preserves_existing_fields(a, b) for a, b in zip(before, after))
    return before == after


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    parser.add_argument("--runs", type=int, default=5)
    args = parser.parse_args()
    if args.runs < 1:
        parser.error("--runs must be positive")
    binaries = [args.before.resolve(), args.after.resolve()]
    with tempfile.TemporaryDirectory(prefix="search-bench-") as tmp:
        root = Path(tmp)
        project = root / "project"
        project.mkdir()
        home = root / "stores"
        store = home / ".codex/sessions"
        store.mkdir(parents=True)
        original = store / "synthetic.jsonl"
        header = {"type": "session_meta", "payload": {
            "id": "synthetic", "cwd": str(project)}}
        event = {"type": "response_item", "payload": {
            "type": "message", "role": "user", "content": [{
                "type": "input_text", "text": "synthetic context " * 64}]}}
        original.write_text(json.dumps(header) + "\n" +
                            (json.dumps(event) + "\n") * 30000)
        known = store / "known.jsonl"
        header["payload"]["id"] = "known"
        known.write_text(json.dumps(header) + "\n" + json.dumps(event) + "\n")
        page = root / "page.txt"
        page.write_text(("synthetic context " * 4096 +
                         " SESSDATA=abcdefghijklmnop\n") * 100)
        env = dict(os.environ, SESSION_ARCHAEOLOGY_HOME=str(home))
        cases = {
            "scan_30000_events": ["sessions", "scan"],
            "resolve_one_of_two_sessions": ["sessions", "resolve", "--source", "codex", "--id", "known"],
            "read_100_large_lines": ["read", "--file", str(page),
                                     "--from", "1", "--to", "100"],
        }
        report = {}
        for name, command in cases.items():
            samples = [[], []]
            expected = None
            # Warm both binaries, then alternate order to reduce timing bias.
            for run in range(args.runs + 1):
                for i in ([0, 1] if run % 2 == 0 else [1, 0]):
                    started = time.perf_counter()
                    output = subprocess.run(
                        [str(binaries[i]), "--json", *command], cwd=project,
                        env=env, check=True, stdout=subprocess.PIPE,
                        stderr=subprocess.PIPE).stdout
                    elapsed = time.perf_counter() - started
                    if expected is None:
                        expected = json.loads(output)
                    if not preserves_existing_fields(expected, json.loads(output)):
                        raise RuntimeError("CLI output changed: " + name)
                    if run:
                        samples[i].append(elapsed)
            before, after = [statistics.median(s) for s in samples]
            report[name] = {"before_seconds": round(before, 4),
                            "after_seconds": round(after, 4),
                            "speedup": round(before / after, 2),
                            "preserved_existing_fields": True, "runs": args.runs}
        inventory = subprocess.check_output([str(binaries[1]), "--json", "sessions", "scan"],
                                            cwd=project, env=env)
        baseline = root / "inventory.json"
        baseline.write_bytes(inventory)
        expected = json.loads(inventory)["data"]["sessions"]
        samples = []
        for _ in range(args.runs):
            started = time.perf_counter()
            delta = json.loads(subprocess.check_output(
                [str(binaries[1]), "--json", "sessions", "scan", "--since", str(baseline)],
                cwd=project, env=env))["data"]
            samples.append(time.perf_counter() - started)
            assert not delta["sessions"] and not delta["not_seen"]
            key = lambda row: (row["source"], row["id"])
            assert sorted(delta["unchanged"], key=key) == sorted(expected, key=key)
        elapsed = statistics.median(samples)
        report["incremental_unchanged_scan"] = {
            "after_seconds": round(elapsed, 4), "runs": args.runs,
            "speedup_vs_current_full_scan": round(
                report["scan_30000_events"]["after_seconds"] / elapsed, 2),
            "same_inventory_rows": True}
        print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
