#!/usr/bin/env python3
"""Evaluate reverse geocoding distance and hit rate on labeled points."""

import json
import math
import re
import sys
import time
import unicodedata
import urllib.error
import urllib.parse
import urllib.request


def normalize(value):
    value = unicodedata.normalize("NFKC", value).casefold()
    value = re.sub(r"[^\w\s]", " ", value, flags=re.UNICODE)
    return " ".join(value.split())


def percentile(values, fraction):
    ordered = sorted(values)
    return ordered[max(0, math.ceil(len(ordered) * fraction) - 1)]


def distance_m(left, right):
    lat1, lon1 = math.radians(left[0]), math.radians(left[1])
    lat2, lon2 = math.radians(right[0]), math.radians(right[1])
    a = math.sin((lat2 - lat1) / 2) ** 2 + math.cos(lat1) * math.cos(lat2) * math.sin((lon2 - lon1) / 2) ** 2
    return 6_371_000.0 * 2 * math.asin(math.sqrt(a))


def main():
    host, path = sys.argv[1], sys.argv[2]
    split = sys.argv[3] if len(sys.argv) > 3 else "all"
    errors = []
    latencies = []
    hits = 0
    count = 0
    failures = []
    categories = {}
    for line in open(path, encoding="utf-8"):
        if not line.strip() or line.startswith("#"):
            continue
        fields = line.rstrip("\n").split("\t")
        lat, lon, accepted = float(fields[0]), float(fields[1]), fields[2]
        row_split = fields[3] if len(fields) > 3 else "all"
        category = fields[4] if len(fields) > 4 else "all"
        if split != "all" and row_split != split:
            continue
        count += 1
        bucket = categories.setdefault(category, {"queries": 0, "hits": 0, "distances": []})
        bucket["queries"] += 1
        url = f"{host}/reverse?{urllib.parse.urlencode({'lat': lat, 'lon': lon})}"
        started = time.perf_counter()
        try:
            with urllib.request.urlopen(url, timeout=10) as response:
                result = json.load(response)
            status = 200
        except urllib.error.HTTPError as error:
            status = error.code
            result = json.loads(error.read() or b"{}")
        latencies.append((time.perf_counter() - started) * 1000.0)
        expected = normalize(accepted)
        if status == 404 and expected.startswith("not"):
            hits += 1
            bucket["hits"] += 1
            continue
        if status != 200:
            failures.append((lat, lon, status, accepted))
            continue
        got = normalize(result.get("name", ""))
        if got == expected:
            hits += 1
            bucket["hits"] += 1
        else:
            failures.append((lat, lon, got, accepted))
        distance = distance_m((lat, lon), (float(result["lat"]), float(result["lon"])))
        errors.append(distance)
        bucket["distances"].append(distance)
    for bucket in categories.values():
        bucket["distance_p95_m"] = round(percentile(bucket.pop("distances"), 0.95), 1) if bucket.get("distances") else None
    print(json.dumps({
        "queries": count,
        "hits": hits,
        "hit_rate": round(hits / count, 4) if count else 0,
        "distance_p50_m": round(percentile(errors, 0.50), 1) if errors else None,
        "distance_p95_m": round(percentile(errors, 0.95), 1) if errors else None,
        "distance_max_m": round(max(errors), 1) if errors else None,
        "p50_ms": round(percentile(latencies, 0.50), 3),
        "p95_ms": round(percentile(latencies, 0.95), 3),
        "categories": categories,
        "failures": failures,
    }, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
