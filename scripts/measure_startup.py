#!/usr/bin/env python3
"""Measure time to /healthz and idle RSS for a built index."""

import json
import os
import subprocess
import sys
import time
import urllib.request


def main():
    args = [value for value in sys.argv[1:] if value != "--drop-cache"]
    drop_cache = "--drop-cache" in sys.argv
    binary, index, manifest = args[:3]
    runs = int(args[3]) if len(args) > 3 else 5
    results = []
    for run in range(runs):
        port = 18100 + run
        if drop_cache:
            fd = os.open(index, os.O_RDONLY)
            os.posix_fadvise(fd, 0, 0, os.POSIX_FADV_DONTNEED)
            os.close(fd)
            time.sleep(0.1)
        process = subprocess.Popen([
            binary, "--index", index, "--manifest", manifest,
            "--bind", f"127.0.0.1:{port}",
        ], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        started = time.perf_counter()
        while True:
            try:
                with urllib.request.urlopen(f"http://127.0.0.1:{port}/healthz", timeout=1):
                    break
            except Exception:
                if time.perf_counter() - started > 30:
                    raise SystemExit("healthz timeout")
                time.sleep(0.002)
        elapsed = (time.perf_counter() - started) * 1000.0
        time.sleep(1)
        status = {}
        for line in open(f"/proc/{process.pid}/status"):
            key, _, value = line.partition(":")
            status[key] = value.strip()
        results.append({
            "run": run + 1,
            "startup_ms": round(elapsed, 2),
            "vm_rss_kib": status.get("VmRSS"),
            "cache": "dontneed" if drop_cache else "warm",
        })
        process.terminate()
        process.wait(timeout=5)
    print(json.dumps(results))
    if results:
        ordered = sorted(row["startup_ms"] for row in results)
        print(json.dumps({"startup_p50_ms": ordered[len(ordered) // 2], "startup_max_ms": max(ordered)}))


if __name__ == "__main__":
    main()
