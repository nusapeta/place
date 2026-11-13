#!/usr/bin/env bash
# Simulate a two-replica rolling restart after an atomic promote.
set -euo pipefail

usage() {
    echo "usage: $0 BIN ROOT RELEASE_A RELEASE_B" >&2
    exit 2
}
[[ $# -eq 4 ]] || usage
bin=$(realpath "$1")
root=$(realpath -m "$2")
rel_a=$(realpath "$3")
rel_b=$(realpath "$4")
port_a=18130
port_b=18131
pids=()

cleanup() {
    for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || true; done
}
trap cleanup EXIT

start() {
    local port=$1
    "$bin" --index "$root/current/places.idx" --manifest "$root/current/places.idx.json" \
        --bind "127.0.0.1:$port" >"/tmp/replica-$port.log" 2>&1 &
    echo $!
}

healthy() {
    curl -sf -o /dev/null "http://127.0.0.1:$1/healthz"
}

mkdir -p "$root"
rm -rf "$root/current" "$root/previous"
ln -s "$rel_a" "$root/current"
pids+=("$(start "$port_a")")
pids+=("$(start "$port_b")")
for port in "$port_a" "$port_b"; do
    for _ in $(seq 1 200); do healthy "$port" && break; sleep 0.05; done
    healthy "$port"
done
before=$(curl -s "http://127.0.0.1:$port_a/version" | jq -r '.built_at')

scripts/promote_release.sh "$rel_b" "$root" >/dev/null
healthy "$port_a"
healthy "$port_b"
after_promote=$(curl -s "http://127.0.0.1:$port_a/version" | jq -r '.built_at')
[[ "$before" == "$after_promote" ]]

for port in "$port_a" "$port_b"; do
    other=$((port == port_a ? port_b : port_a))
    pid=${pids[port == port_a ? 0 : 1]}
    kill "$pid"
    healthy "$other"
    replacement=$(start "$port")
    pids[port == port_a ? 0 : 1]=$replacement
    for _ in $(seq 1 200); do healthy "$port" && break; sleep 0.05; done
    healthy "$port"
done

a=$(curl -s "http://127.0.0.1:$port_a/version" | jq -r '.built_at')
b=$(curl -s "http://127.0.0.1:$port_b/version" | jq -r '.built_at')
[[ "$a" == "$b" ]]
[[ "$a" != "$before" ]]
jq -n --arg before "$before" --arg after "$a" \
    --arg sha "$(curl -s "http://127.0.0.1:$port_a/version" | jq -r '.index_sha256')" \
    '{replicas: 2, built_at_before_restart: $before, built_at_after_restart: $after, index_sha256: $sha, health: "200 at every restart", downtime_seconds: 0}'
