# External VictoriaMetrics / cluster

By default DumbMonit starts its own VictoriaMetrics inside the container. You
can point it at one you run yourself instead.

## When it makes sense

* More than about **5 000 monitors** (see [Sizing](sizing.md)).
* **High availability**, or storage on its own host or disk.
* You **already run** VictoriaMetrics and want one place for retention, backups
  and monitoring.
* You want to read DumbMonit's series from **Grafana** or other tools too.

For a homelab or a small team the embedded one is simpler and needs nothing.

## Single-node instance

```yaml
    environment:
      DUMBMONIT_VM_URL: http://victoriametrics:8428
```

DumbMonit writes to `<URL>/api/v1/import/prometheus` and reads from
`<URL>/api/v1/query`, `query_range`, `series` and `label/…`. Nothing else is
needed on the VictoriaMetrics side; its series are prefixed `dumbmonit_`, so it
can be shared.

When `DUMBMONIT_VM_URL` is set, **the embedded VictoriaMetrics is not started**
and `/data/vm` stays empty. `GET /api/health` reports `victoria.embedded: false`.

## What changes

* **Retention is managed on the external instance** (`-retentionPeriod`). In
  **Settings → System** the raw-retention field becomes read-only.
* **Memory** (`DUMBMONIT_VM_MEMORY`) and the listen address do not apply: size
  the external instance yourself.
* **Deleting series** (when you remove a monitor or clean up) calls
  `/api/v1/admin/tsdb/delete_series`. Single-node VictoriaMetrics allows it by
  default; a `-deleteAuthKey` on the instance makes DumbMonit's call fail, the
  rest keeps working and the series simply age out.
* **Backups**: the DumbMonit backup no longer covers metrics. Back the
  instance up with `vmbackup` or the volume snapshots of your platform.
* The server buffers samples and retries when the instance is down, within the
  limits of its in-memory buffer; a long outage loses data.

## Cluster (vminsert / vmselect)

A VictoriaMetrics cluster serves writes and reads on different services and
under tenant paths:

| Operation | Cluster URL |
|---|---|
| Write | `http://vminsert:8480/insert/0/prometheus/api/v1/import/prometheus` |
| Read | `http://vmselect:8481/select/0/prometheus/api/v1/query_range` |

DumbMonit builds every URL as `DUMBMONIT_VM_URL` + `/api/v1/…`, and a single
URL must serve both writes and reads. It therefore cannot talk to `vminsert`
and `vmselect` directly. Put **vmauth** in front and route by path:

```yaml
# vmauth config (-auth.config)
unauthorized_user:
  url_map:
    - src_paths: ["/api/v1/import/.*"]
      url_prefix: "http://vminsert:8480/insert/0/prometheus/"
    - src_paths: ["/api/v1/admin/tsdb/delete_series"]
      url_prefix: "http://vmselect:8481/delete/0/prometheus/"
    - src_paths: ["/api/v1/(query|query_range|series|labels|label/.*|export)"]
      url_prefix: "http://vmselect:8481/select/0/prometheus/"
```

Then `DUMBMONIT_VM_URL: http://vmauth:8427`. Keep vmauth on a private network,
or give it `users:` with credentials; a shared secret in the URL is stored in
the environment of the container, so treat it like any other secret. Tenant `0`
is only an example: use the account id you want.

## Long-term store

The 5-minute aggregates live in a second VictoriaMetrics. With the embedded
setup it is started for you under `/data/vm-long`. With an external main
instance there is none unless you name one:

```yaml
    environment:
      DUMBMONIT_VM_URL: http://victoriametrics:8428
      DUMBMONIT_VM_LONG_URL: http://victoriametrics-long:8428
```

Use a **separate instance** (or tenant behind vmauth, same routing as above):
the aggregates use the same metric names as the raw data, plus `:5m_min`,
`:5m_max`, `:5m_sum` and `:5m_count` suffixes, and their retention (730 days by
default) differs from the raw one. Set `-retentionPeriod` on that instance
accordingly. Without `DUMBMONIT_VM_LONG_URL`, an external main instance means
**no long-term history**: **Settings → System** says so, and graphs are limited
to the retention of the external instance.

Setting `DUMBMONIT_VM_LONG_URL` with the embedded main instance also works: the
embedded long-term store is then not started.
