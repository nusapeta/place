#!/usr/bin/env python3
"""Measure two read-only passes over an OSM PBF.

This is a spike, not the production builder. It counts named features and
builds a normalized-name set in pass one, then checks the same set in pass two.
Output is JSON on stdout.
"""

import json
import resource
import sys
import time
import unicodedata

import osmium


def normalize(value):
    text = unicodedata.normalize("NFKC", value).lower()
    text = "".join(
        char
        for char in unicodedata.normalize("NFD", text)
        if unicodedata.category(char) != "Mn"
    )
    return " ".join(
        "".join(char if char.isalnum() else " " for char in text).split()
    )


def main(path):
    started = time.monotonic()
    names = set()
    counts = {"node": 0, "way": 0, "relation": 0}

    for obj in osmium.FileProcessor(path):
        name = obj.tags.get("name")
        if not name:
            continue
        if obj.is_node():
            counts["node"] += 1
        elif obj.is_way():
            counts["way"] += 1
        elif obj.is_relation():
            counts["relation"] += 1
        else:
            raise RuntimeError("unknown OSM object type")
        names.add(normalize(name))
    first_seconds = time.monotonic() - started

    started = time.monotonic()
    seen = 0
    for obj in osmium.FileProcessor(path):
        name = obj.tags.get("name")
        if name and normalize(name) in names:
            seen += 1
    second_seconds = time.monotonic() - started

    print(json.dumps({
        "features": sum(counts.values()),
        "counts": counts,
        "unique_normalized_names": len(names),
        "second_pass_features": seen,
        "first_pass_seconds": round(first_seconds, 3),
        "second_pass_seconds": round(second_seconds, 3),
        "peak_rss_kib": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
    }))


if __name__ == "__main__":
    main(sys.argv[1])
