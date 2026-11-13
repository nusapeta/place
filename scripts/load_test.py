#!/usr/bin/env python3
"""Fixed-rate mixed load test for a running placeid server."""

import argparse
import http.client
import json
import random
import statistics
import threading
import time
import urllib.parse

MIX = [("search", 0.70), ("reverse", 0.20), ("lookup", 0.10)]


def pick(rng):
    value = rng.random()
    total = 0.0
    for name, share in MIX:
        total += share
        if value < total:
            return name
    return MIX[-1][0]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("host", help="host:port")
    parser.add_argument("--rate", type=float, default=20.0)
    parser.add_argument("--seconds", type=float, default=60.0)
    parser.add_argument("--warmup", type=float, default=10.0)
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--queries", default="docs/research/spikes/queries-285.tsv")
    args = parser.parse_args()
    host, port = args.host.split(":")
    queries = [line.split("\t")[0] for line in open(args.queries, encoding="utf-8") if line.strip() and not line.startswith("#")]
    connection = http.client.HTTPConnection(host, int(port), timeout=10)
    connection.request("GET", "/search?" + urllib.parse.urlencode({"q": "jakarta", "limit": 5}))
    ids = [row["place_id"] for row in json.load(connection.getresponse())]
    lock = threading.Lock()
    latencies = {"search": [], "reverse": [], "lookup": []}
    errors = []
    stop = time.monotonic() + args.warmup + args.seconds
    started = time.monotonic() + args.warmup

    def worker(seed):
        rng = random.Random(seed)
        connection = http.client.HTTPConnection(host, int(port), timeout=10)
        while True:
            now = time.monotonic()
            if now >= stop:
                return
            if now < started:
                time.sleep(0.01)
                continue
            kind = pick(rng)
            if kind == "search":
                path = "/search?" + urllib.parse.urlencode({"q": rng.choice(queries), "limit": 5})
            elif kind == "reverse":
                path = "/reverse?" + urllib.parse.urlencode({"lat": rng.uniform(-11, 6), "lon": rng.uniform(95, 141)})
            else:
                path = "/lookup?" + urllib.parse.urlencode({"place_ids": ",".join(str(value) for value in rng.sample(ids, min(2, len(ids))))})
            begin = time.perf_counter()
            try:
                connection.request("GET", path)
                response = connection.getresponse()
                response.read()
                elapsed = (time.perf_counter() - begin) * 1000.0
                if response.status >= 400 and not (kind == "reverse" and response.status == 404):
                    with lock:
                        errors.append((kind, response.status))
                else:
                    with lock:
                        latencies[kind].append(elapsed)
            except Exception as error:
                with lock:
                    errors.append((kind, repr(error)))
            delay = max(0.0, (now + (args.threads / args.rate)) - time.monotonic())
            time.sleep(delay)

    threads = [threading.Thread(target=worker, args=(index,)) for index in range(args.threads)]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    all_latencies = [value for values in latencies.values() for value in values]

    def summary(values):
        if not values:
            return None
        ordered = sorted(values)
        return {
            "requests": len(values),
            "p50_ms": round(statistics.median(ordered), 3),
            "p95_ms": round(ordered[max(0, int(len(ordered) * 0.95 + 0.999) - 1)], 3),
            "p99_ms": round(ordered[max(0, int(len(ordered) * 0.99 + 0.999) - 1)], 3),
        }

    print(json.dumps({
        "seconds": args.seconds,
        "target_rps": args.rate,
        "achieved_rps": round(len(all_latencies) / args.seconds, 2),
        "errors": len(errors),
        "error_rate": round(len(errors) / max(1, len(all_latencies) + len(errors)), 4),
        "overall": summary(all_latencies),
        "search": summary(latencies["search"]),
        "reverse": summary(latencies["reverse"]),
        "lookup": summary(latencies["lookup"]),
        "error_sample": errors[:5],
    }, indent=2))


if __name__ == "__main__":
    main()
