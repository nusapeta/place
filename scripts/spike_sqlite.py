#!/usr/bin/env python3
"""Build and query a SQLite FTS5 index for the engine spike."""

import json
import os
import resource
import sqlite3
import sys
import time


def rss_kib():
    with open("/proc/self/status", encoding="utf-8") as status:
        for line in status:
            if line.startswith("VmRSS:"):
                return int(line.split()[1])
    return 0


def main(names, queries, database):
    if os.path.exists(database):
        os.remove(database)
    connection = sqlite3.connect(database)
    connection.executescript(
        """
        PRAGMA journal_mode=OFF;
        PRAGMA synchronous=OFF;
        CREATE TABLE docs (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
        CREATE VIRTUAL TABLE fts USING fts5(
            name, content='docs', content_rowid='id',
            tokenize='unicode61 remove_diacritics 2'
        );
        """
    )
    started = time.monotonic()
    batch = []
    with open(names, encoding="utf-8") as source:
        for index, line in enumerate(source, start=1):
            name = line.split("\t", 1)[0]
            batch.append((index, name))
            if len(batch) == 50_000:
                connection.executemany("INSERT INTO docs VALUES (?, ?)", batch)
                batch.clear()
        if batch:
            connection.executemany("INSERT INTO docs VALUES (?, ?)", batch)
    connection.execute("INSERT INTO fts(fts) VALUES('rebuild')")
    connection.commit()
    build_seconds = time.monotonic() - started
    documents = connection.execute("SELECT count(*) FROM docs").fetchone()[0]
    latencies = []
    top1 = 0
    count = 0
    with open(queries, encoding="utf-8") as source:
        for line in source:
            query, expected = line.rstrip("\n").split("\t", 1)
            started = time.monotonic()
            rows = connection.execute(
                """SELECT d.name FROM fts JOIN docs d ON d.id = fts.rowid
                   WHERE fts MATCH ? ORDER BY rank LIMIT 10""",
                (f'"{query}"',),
            ).fetchall()
            latencies.append((time.monotonic() - started) * 1000)
            count += 1
            if rows and rows[0][0] == expected:
                top1 += 1
    latencies.sort()
    p95 = latencies[max(0, int(len(latencies) * 0.95) - 1)]
    print(json.dumps({
        "documents": documents,
        "build_seconds": round(build_seconds, 2),
        "index_bytes": os.path.getsize(database),
        "queries": count,
        "top1": top1,
        "p95_ms": round(p95, 3),
        "rss_kib": rss_kib(),
    }))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], sys.argv[3])
