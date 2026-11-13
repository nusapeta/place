#!/usr/bin/env python3
"""Resolve accepted label names to place IDs and add them as a fifth column."""

import json
import math
import re
import sys
import unicodedata
import urllib.parse
import urllib.request


def normalize(value):
    value = unicodedata.normalize("NFKC", value).casefold()
    value = re.sub(r"[^\w\s]", " ", value, flags=re.UNICODE)
    return " ".join(value.split())


def main():
    host, path = sys.argv[1], sys.argv[2]
    output = []
    missing = []
    for line in open(path, encoding="utf-8"):
        if not line.strip() or line.startswith("#"):
            output.append(line.rstrip("\n"))
            continue
        fields = line.rstrip("\n").split("\t")
        ids = []
        if fields[1] != "not_found":
            for accepted in fields[1].split("|"):
                url = f"{host}/search?{urllib.parse.urlencode({'q': accepted, 'limit': 20})}"
                with urllib.request.urlopen(url, timeout=10) as response:
                    results = json.load(response)
                for result in results:
                    if normalize(result.get("name", "")) == normalize(accepted):
                        value = str(result["place_id"])
                        if value not in ids:
                            ids.append(value)
                        break
        if not ids and fields[1] != "not_found":
            missing.append(fields[0])
        fields = fields[:4] + ["|".join(ids) if ids else "-"]
        output.append("\t".join(fields))
    open(path, "w", encoding="utf-8").write("\n".join(output) + "\n")
    print(json.dumps({"missing": missing, "count": len(missing)}))


if __name__ == "__main__":
    main()
