#!/usr/bin/env python3
"""Measure sparse grid cell and candidate counts for reverse radii.

One pass collects named node coordinates. A deterministic sample (every Nth
node) is used as a reverse-geocode center. For each center and radius, the
script counts cells intersecting the radius and the named nodes in those
cells. Output is JSON on stdout.
"""

import json
import math
import sys

import osmium

CELL_E7 = 100_000
EARTH_RADIUS_M = 6_371_008.8


def cell(lat_e7, lon_e7):
    return (lat_e7 // CELL_E7, lon_e7 // CELL_E7)


def main(path):
    points = []
    for obj in osmium.FileProcessor(path):
        if obj.is_node() and obj.tags.get("name") and obj.location.valid():
            points.append((obj.location.y, obj.location.x))
    points.sort()
    sample = points[:: max(1, len(points) // 200)][:200]
    cells = {}
    for lat, lon in points:
        key = cell(round(lat * 1e7), round(lon * 1e7))
        cells[key] = cells.get(key, 0) + 1

    results = {}
    for radius_m in (1_000, 10_000):
        cell_counts = []
        candidate_counts = []
        for lat, lon in sample:
            dlat = math.degrees(radius_m / EARTH_RADIUS_M)
            dlon = math.degrees(
                radius_m / (EARTH_RADIUS_M * math.cos(math.radians(lat)))
            )
            min_lat = math.floor((lat - dlat) * 1e7)
            max_lat = math.floor((lat + dlat) * 1e7)
            min_lon = math.floor((lon - dlon) * 1e7)
            max_lon = math.floor((lon + dlon) * 1e7)
            min_row, max_row = min_lat // CELL_E7, max_lat // CELL_E7
            min_col, max_col = min_lon // CELL_E7, max_lon // CELL_E7
            cell_counts.append((max_row - min_row + 1) * (max_col - min_col + 1))
            candidate_counts.append(sum(
                cells.get((row, col), 0)
                for row in range(min_row, max_row + 1)
                for col in range(min_col, max_col + 1)
            ))
        cell_counts.sort()
        candidate_counts.sort()
        results[str(radius_m)] = {
            "mean_cells": round(sum(cell_counts) / len(cell_counts), 2),
            "p95_cells": cell_counts[int(len(cell_counts) * 0.95) - 1],
            "max_cells": cell_counts[-1],
            "mean_candidates": round(
                sum(candidate_counts) / len(candidate_counts), 2
            ),
            "p95_candidates": candidate_counts[int(len(candidate_counts) * 0.95) - 1],
            "max_candidates": candidate_counts[-1],
        }
    print(json.dumps({
        "named_nodes": len(points),
        "sampled_centers": len(sample),
        "occupied_cells": len(cells),
        "cell_size_e7": CELL_E7,
        "radii": results,
    }, indent=2))


if __name__ == "__main__":
    main(sys.argv[1])
