#!/usr/bin/env python3
"""Pilote du test de charge (voir run.sh). Bibliothèque standard seulement."""

import argparse
import http.cookiejar
import json
import re
import secrets
import statistics
import subprocess
import sys
import time
import urllib.error
import urllib.request

HEADERS = {"X-Requested-With": "DumbMonit", "Content-Type": "application/json"}
SLUG = "load-test"


class Api:
    def __init__(self, base):
        self.base = base
        jar = http.cookiejar.CookieJar()
        self.opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))

    def call(self, method, path, body=None, timeout=60):
        data = None if body is None else json.dumps(body).encode()
        req = urllib.request.Request(self.base + path, data=data, method=method, headers=HEADERS)
        with self.opener.open(req, timeout=timeout) as resp:
            raw = resp.read()
            return json.loads(raw) if raw and "json" in resp.headers.get("Content-Type", "") else raw


def sh(*cmd):
    return subprocess.run(cmd, check=True, capture_output=True, text=True).stdout


def to_mib(text):
    m = re.match(r"([\d.]+)\s*([KMGT]?i?B)", text.strip())
    value, unit = float(m.group(1)), m.group(2)
    scale = {"B": 1 / 1048576, "KiB": 1 / 1024, "MiB": 1, "GiB": 1024, "TiB": 1048576,
             "kB": 1 / 1000, "MB": 1, "GB": 1000}
    return value * scale.get(unit, 1)


def docker_stats(container):
    out = sh("docker", "stats", "--no-stream", "--format", "{{json .}}", container)
    row = json.loads(out)
    return {"cpu_percent": float(row["CPUPerc"].rstrip("%")),
            "mem_mib": to_mib(row["MemUsage"].split("/")[0])}


def disk_usage(volume, alpine):
    """Octets par dossier de /data, mesurés depuis alpine (l'image n'a pas de shell)."""
    out = sh("docker", "run", "--rm", "-v", f"{volume}:/data:ro", alpine,
             "sh", "-c", "du -sk /data/* 2>/dev/null; du -sk /data")
    sizes = {}
    for line in out.splitlines():
        kib, path = line.split("\t", 1)
        sizes[path.replace("/data/", "") if path != "/data" else "total"] = int(kib) * 1024
    return sizes


def percentile(values, q):
    if not values:
        return None
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int(round(q * (len(ordered) - 1))))]


def wait_ready(api, timeout=180):
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            api.call("GET", "/api/health", timeout=5)
            return
        except Exception:
            time.sleep(2)
    raise SystemExit("The server did not become ready in time.")


def setup(api, code, password):
    try:
        api.call("POST", "/api/auth/setup",
                 {"setup_code": code, "username": "admin", "password": password})
    except urllib.error.HTTPError as err:
        raise SystemExit(f"setup failed: {err.code} {err.read().decode()[:200]}")
    api.call("POST", "/api/auth/login", {"username": "admin", "password": password})


def create_monitors(api, count):
    ids = []
    started = time.time()
    for i in range(count):
        view = api.call("POST", "/api/targets", {
            "name": f"load-{i:05d}", "address": "demo", "kind": "dummy",
            "interval_secs": 30, "credential": {"type": "none"},
            "group_name": f"load-{i // 100:03d}",
        })
        ids.append(view["id"])
        if (i + 1) % 500 == 0:
            print(f"  {i + 1}/{count} monitors created", flush=True)
    return ids, time.time() - started


def make_status_page(api, ids):
    page = api.call("POST", "/api/status-pages",
                    {"title": "Load test", "slug": SLUG, "published": True})
    api.call("PUT", f"/api/status-pages/{page['id']}/items",
             [{"target_id": t} for t in ids[:50]])


def timed(api, path):
    started = time.perf_counter()
    try:
        api.call("GET", path, timeout=60)
        return (time.perf_counter() - started) * 1000, True
    except Exception:
        return (time.perf_counter() - started) * 1000, False


def endpoints():
    now = int(time.time() * 1000)
    q = "avg(dumbmonit_cpu_usage_percent)"
    return {
        "GET /api/health": "/api/health",
        "GET /api/targets": "/api/targets",
        f"GET /api/public/status/{SLUG}": f"/api/public/status/{SLUG}",
        "GET /api/metrics/query_range (1 h)":
            f"/api/metrics/query_range?query={q}&start={now - 3600_000}&end={now}&step=60",
        "GET /api/metrics/query_range (24 h)":
            f"/api/metrics/query_range?query={q}&start={now - 86400_000}&end={now}&step=300",
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", required=True)
    ap.add_argument("--setup-code", required=True)
    ap.add_argument("--container", required=True)
    ap.add_argument("--volume", required=True)
    ap.add_argument("--alpine", required=True)
    ap.add_argument("--image", default="")
    ap.add_argument("--monitors", type=int, required=True)
    ap.add_argument("--duration-min", type=float, required=True)
    ap.add_argument("--sample-seconds", type=int, default=60)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()

    api = Api(a.base)
    wait_ready(api)
    setup(api, a.setup_code, secrets.token_urlsafe(24))
    print(f"Creating {a.monitors} monitors...", flush=True)
    ids, create_s = create_monitors(api, a.monitors)
    make_status_page(api, ids)

    t0 = time.time()
    disk0 = disk_usage(a.volume, a.alpine)
    samples, latency = [], {}
    failures = {}
    deadline = t0 + a.duration_min * 60
    while True:
        stats = docker_stats(a.container)
        for name, path in endpoints().items():
            for _ in range(10):
                ms, ok = timed(api, path)
                latency.setdefault(name, []).append(ms)
                if not ok:
                    failures[name] = failures.get(name, 0) + 1
        stats["t"] = round(time.time() - t0)
        samples.append(stats)
        print(f"  t={stats['t']}s cpu={stats['cpu_percent']}% mem={stats['mem_mib']:.0f} MiB",
              flush=True)
        if time.time() >= deadline:
            break
        time.sleep(a.sample_seconds)

    disk1 = disk_usage(a.volume, a.alpine)
    elapsed_h = max((time.time() - t0) / 3600, 1e-6)
    growth = {k: disk1.get(k, 0) - disk0.get(k, 0) for k in disk1}
    cpus = [s["cpu_percent"] for s in samples]
    mems = [s["mem_mib"] for s in samples]
    summary = {
        "image": a.image, "monitors": a.monitors, "duration_min": a.duration_min,
        "create_seconds": round(create_s, 1),
        "cpu_percent": {"mean": round(statistics.mean(cpus), 1),
                        "p95": round(percentile(cpus, 0.95), 1), "max": max(cpus)},
        "mem_mib": {"mean": round(statistics.mean(mems)), "max": round(max(mems))},
        "disk_bytes": {"start": disk0, "end": disk1, "growth": growth,
                       "growth_per_day_extrapolated": {
                           k: int(v / elapsed_h * 24) for k, v in growth.items()}},
        "latency_ms": {n: {"p50": round(percentile(v, 0.5), 1), "p95": round(percentile(v, 0.95), 1),
                           "requests": len(v), "failures": failures.get(n, 0)}
                       for n, v in latency.items()},
        "samples": samples,
    }
    with open(f"{a.out}/summary.json", "w") as f:
        json.dump(summary, f, indent=2)
    with open(f"{a.out}/summary.md", "w") as f:
        f.write(markdown(summary))


def mib(n):
    return f"{n / 1048576:.1f}"


def markdown(s):
    lines = [
        f"# DumbMonit load test: {s['monitors']} monitors, {s['duration_min']:g} min",
        "", f"Image: `{s['image']}`. Monitors created in {s['create_seconds']} s "
        "(kind `dummy`, 30 s interval, 2 series each plus `up`).", "",
        "## Container", "",
        "| | mean | p95 | max |", "|---|---|---|---|",
        f"| CPU (% of one core) | {s['cpu_percent']['mean']} | {s['cpu_percent']['p95']} "
        f"| {s['cpu_percent']['max']} |",
        f"| Memory (MiB) | {s['mem_mib']['mean']} | | {s['mem_mib']['max']} |", "",
        "## Disk (`/data`)", "",
        "| Directory | end (MiB) | growth (MiB) | per day, extrapolated (MiB) |",
        "|---|---|---|---|",
    ]
    d = s["disk_bytes"]
    for k in sorted(d["end"]):
        lines.append(f"| {k} | {mib(d['end'][k])} | {mib(d['growth'].get(k, 0))} "
                     f"| {mib(d['growth_per_day_extrapolated'].get(k, 0))} |")
    lines += ["", "Short runs overstate the daily figure: VictoriaMetrics writes in parts and "
              "merges them later. Trust runs of an hour or more.", "",
              "## API latency", "", "| Endpoint | p50 (ms) | p95 (ms) | requests | failures |",
              "|---|---|---|---|---|"]
    for n, v in s["latency_ms"].items():
        lines.append(f"| `{n}` | {v['p50']} | {v['p95']} | {v['requests']} | {v['failures']} |")
    return "\n".join(lines) + "\n"


if __name__ == "__main__":
    sys.exit(main())
