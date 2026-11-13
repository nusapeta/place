#!/usr/bin/env python3
"""Smoke test a full index through the HTTP API."""

import json
import sys
import urllib.error
import urllib.parse
import urllib.request

FORWARD = [
    "jakarta",
    "bandung",
    "surabaya",
    "medan",
    "semarang",
    "makassar",
    "yogyakarta",
    "denpasar",
    "palembang",
    "balikpapan",
    "jalan sudirman",
    "jalan gatot subroto",
    "jalan malioboro",
    "monumen nasional",
    "borobudur",
    "danau toba",
    "bromo",
    "indomaret",
    "bandara soekarno hatta",
    "pelabuhan tanjung priok",
]

REVERSE = [
    (-6.2000, 106.8200),
    (-6.9175, 107.6191),
    (-7.2575, 112.7521),
    (3.5952, 98.6722),
    (-6.9667, 110.4167),
    (-5.1477, 119.4327),
    (-7.7956, 110.3695),
    (-8.6500, 115.2167),
    (-2.9761, 104.7754),
    (-1.2379, 116.8529),
]

BBOX = (-12.0, 80.0, 11.0, 142.0)


def get(host, path):
    with urllib.request.urlopen(f"{host}{path}", timeout=10) as response:
        return response.status, json.load(response)


def rows(value):
    return [value] if isinstance(value, dict) else value


def valid_place(place):
    try:
        lat = float(place["lat"])
        lon = float(place["lon"])
        place["place_id"]
        place["display_name"]
    except (KeyError, TypeError, ValueError):
        return False
    return BBOX[0] <= lat <= BBOX[2] and BBOX[1] <= lon <= BBOX[3]


def main():
    host = sys.argv[1]
    forward = []
    for query in FORWARD:
        status, results = get(host, "/search?" + urllib.parse.urlencode({"q": query, "limit": 5}))
        forward.append((query, status, len(results), all(valid_place(row) for row in results)))
    reverse = []
    for lat, lon in REVERSE:
        path = "/reverse?" + urllib.parse.urlencode({"lat": lat, "lon": lon, "limit": 3})
        try:
            status, results = get(host, path)
        except urllib.error.HTTPError as error:
            reverse.append((lat, lon, error.code, 0, error.code == 404))
            continue
        reverse.append((lat, lon, status, len(rows(results)), all(valid_place(row) for row in rows(results))))
    failures = [row for row in forward if row[1] != 200 or row[2] == 0 or not row[3]]
    failures += [row for row in reverse if row[2] not in (200, 404) or not row[4]]
    print(json.dumps({"forward": forward, "reverse": reverse, "failures": failures}, indent=2))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
