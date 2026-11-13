#!/usr/bin/env python3
"""Run the release builder and report wall time and peak RSS."""

import json
import resource
import subprocess
import sys
import time


def main(command):
    started = time.monotonic()
    result = subprocess.run(command, check=False)
    elapsed = time.monotonic() - started
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    print(json.dumps({
        "exit_code": result.returncode,
        "seconds": round(elapsed, 2),
        "peak_rss_kib": usage.ru_maxrss,
    }), flush=True)
    return result.returncode


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
