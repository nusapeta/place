#!/usr/bin/env python3
"""Evaluate a running nusapeta-place server against a labeled query file."""

import argparse
import json
import re
import sys
import time
import unicodedata
import urllib.parse
import urllib.request


def normalize(value):
    value = unicodedata.normalize("NFKC", value).casefold()
    value = re.sub(r"[^\w\s]", " ", value, flags=re.UNICODE)
    return " ".join(value.split())


def percentile(values, fraction):
    ordered = sorted(values)
    index = max(0, int(len(ordered) * fraction + 0.999999) - 1)
    return ordered[index]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("host")
    parser.add_argument("queries")
    parser.add_argument("--label", default="eval")
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--split", choices=["all", "tune", "holdout"], default="all")
    args = parser.parse_args()

    latencies = []
    top1 = 0
    top5 = 0
    empty = 0
    duplicated = 0
    count = 0
    failures = []
    categories = {}
    with open(args.queries, encoding="utf-8") as source:
        for line in source:
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            fields = line.split("\t")
            query, expected = fields[0], fields[1]
            split = fields[2] if len(fields) > 2 else "tune"
            category = fields[3] if len(fields) > 3 else "all"
            if args.split != "all" and split != args.split:
                continue
            accepted = {normalize(value) for value in expected.split("|")}
            url = f"{args.host}/search?{urllib.parse.urlencode({'q': query, 'limit': args.limit})}"
            started = time.perf_counter()
            with urllib.request.urlopen(url, timeout=10) as response:
                results = json.load(response)
            latencies.append((time.perf_counter() - started) * 1000.0)
            count += 1
            bucket = categories.setdefault(category, {"queries": 0, "top1": 0, "top5": 0})
            bucket["queries"] += 1
            names = [normalize(result.get("name", "")) for result in results]
            if not names:
                empty += 1
                failures.append((query, "empty"))
                continue
            if len(set(result["place_id"] for result in results)) != len(results):
                duplicated += 1
            if names[0] in accepted:
                top1 += 1
                bucket["top1"] += 1
            elif len(failures) < 20:
                failures.append((query, names[0]))
            if accepted.intersection(names[:5]):
                top5 += 1
                bucket["top5"] += 1
    print(json.dumps({
        "label": args.label,
        "queries": count,
        "top1": top1,
        "top1_rate": round(top1 / count, 4) if count else 0.0,
        "top5": top5,
        "top5_rate": round(top5 / count, 4) if count else 0.0,
        "empty": empty,
        "duplicated_responses": duplicated,
        "p50_ms": round(percentile(latencies, 0.50), 3),
        "p95_ms": round(percentile(latencies, 0.95), 3),
        "p99_ms": round(percentile(latencies, 0.99), 3),
        "categories": categories,
        "failures": failures,
    }, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
