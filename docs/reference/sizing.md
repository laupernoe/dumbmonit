# Sizing

What a DumbMonit container needs for 100, 1 000 and 5 000 monitors, and how
long the data can be kept. **The figures below are estimates** from the
storage model described further down. They are being confirmed with the
[load-test workflow](#measure-it-yourself); until that is done, treat them as a
starting point and watch the disk graph in **Settings → System**.

## At a glance

Assumptions: the 30 s default polling interval and about 20 series per monitor
(an HTTP check writes fewer, a Proxmox host or a switch with many interfaces
writes many more). Raw data is kept 12 months, the 5-minute aggregates 730 days.

| | 100 monitors | 1 000 monitors | 5 000 monitors |
|---|---|---|---|
| Active series | ~2 000 | ~20 000 | ~100 000 |
| CPU (whole container) | 0.1 core | 0.5 core | 2 to 3 cores |
| RAM, server + VictoriaMetrics | 250 MB | 500 MB | 1.5 to 2 GB |
| RAM, long-term VictoriaMetrics (`DUMBMONIT_VM_LONG_MEMORY`) | 64 MB | 64 MB | 128 to 256 MB |
| RAM, VictoriaLogs (optional) | 64 MB | 64 MB | 128 MB |
| Raw data, per day | ~6 MB | ~60 MB | ~300 MB |
| Raw data, 12 months | ~2 GB | ~21 GB | ~105 GB |
| 5-minute aggregates, per day | ~1.5 MB | ~15 MB | ~75 MB |
| 5-minute aggregates, 730 days | ~1 GB | ~11 GB | ~55 GB |
| API latency, p95 (`/api/targets`, graphs) | < 50 ms | < 150 ms | < 500 ms |

Past roughly **5 000 monitors**, or when you need high availability, use an
[external VictoriaMetrics](external-victoriametrics.md).

## Disk

VictoriaMetrics compresses a sample to about one byte on average (less for
slowly changing values, more for noisy ones):

```text
bytes per day = series x samples per day per series x ~1 byte
samples per day per series = 86 400 / polling interval in seconds   (2 880 at 30 s)
```

Example: 20 000 series at 30 s is 20 000 x 2 880 x 1 byte, about 58 MB per day.
Keep a 30 % margin for background merges and indexes, which VictoriaMetrics
needs while it compacts.

The 5-minute aggregates hold 288 samples per day per series, but a gauge is
stored three times (average, minimum, maximum), and availability metrics add a
sum and a count. In practice they weigh about a quarter of the raw data per day,
and they are kept much longer: that is why raw retention can stay short.

## Retention advice

* **Raw data** (30 s points) answers "what happened at 14:03 yesterday".
  30 to 90 days is enough for most teams; keep more only if you read graphs at
  full resolution that far back.
* **Long-term history** (5-minute aggregates, 730 days by default) answers
  trends, monthly availability and reports. A graph, a status page or a report
  whose range goes past the raw retention is answered from it automatically.
* Both are set in **Settings → System → Data & retention**. Changing a value
  restarts the embedded VictoriaMetrics for a few seconds; samples are buffered
  meanwhile.
* The page shows the disk growth and the number of days left before `/data` is
  full.

## Measure it yourself

The **Load test** workflow starts the published image on a GitHub runner,
creates the monitors (kind "Demo device", 30 s interval), lets them run, and
reports CPU, memory, disk growth per directory and API latency.

1. In the repository, open **Actions → Load test → Run workflow**.
2. Choose 1 000 or 5 000 monitors, a duration (60 minutes or more gives
   meaningful disk figures) and the image tag (`edge` by default).
3. When it ends, read the job summary, or download the `load-test-<n>` artifact
   (`summary.md` and `summary.json`).

How to read it:

* **CPU** is a percentage of one core; the runner has 4 vCPUs, so a value near
  400 % means saturation.
* **Disk, "per day, extrapolated"** multiplies the growth measured during the
  run by the time left in the day. Short runs overstate it, because
  VictoriaMetrics merges its parts later. `vm` is the raw data, `vm-long` the
  aggregates (it fills after the first 5-minute window), `dumbmonit.db` the
  configuration and state.
* **Latency** is measured from the runner against `localhost`, so it excludes
  the network.

The same script runs anywhere with Docker and Python 3:
`MONITORS=1000 DURATION_MIN=60 scripts/load/run.sh`.
