#!/usr/bin/env python3
"""Measure top-1, p95 latency, and idle RSS of a running placeid server."""

import json
import sys
import time
import urllib.parse
import urllib.request


def main(base, queries, pid):
    latencies = []
    top1 = 0
    count = 0
    with open(queries, encoding="utf-8") as source:
        for line in source:
            query, expected = line.rstrip("\n").split("\t", 1)
            url = f"{base}/search?q={urllib.parse.quote(query)}&limit=10"
            started = time.monotonic()
            with urllib.request.urlopen(url) as response:
                rows = json.load(response)
            latencies.append((time.monotonic() - started) * 1000)
            count += 1
            if rows and rows[0]["name"].lower() == expected:
                top1 += 1
    latencies.sort()
    rss = "unknown"
    with open(f"/proc/{pid}/status", encoding="utf-8") as status:
        for line in status:
            if line.startswith("VmRSS:"):
                rss = line.split()[1] + " KiB"
    print(json.dumps({
        "queries": count,
        "top1": top1,
        "p95_ms": round(latencies[max(0, int(len(latencies) * 0.95) - 1)], 3),
        "mean_ms": round(sum(latencies) / len(latencies), 3),
        "rss": rss,
    }))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], sys.argv[3])
