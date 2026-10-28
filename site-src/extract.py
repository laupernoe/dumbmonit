#!/usr/bin/env python3
"""Extracts the device catalogue of the website from the real sources of the project.

The website build (build.py) calls build() itself: there is no catalogue file to keep up to date.
What the site announces as available is read from the LATEST STABLE RELEASE TAG (vX.Y.Z), never
from the branch being built, so a push to dev cannot advertise an unreleased integration.

    python3 site-src/extract.py              # prints the summary for the release the site would use
    python3 site-src/extract.py --json       # prints the whole catalogue (JSON)
    DUMBMONIT_SITE_REF=worktree python3 ...  # reads the working tree instead (local preview)
    DUMBMONIT_SITE_REF=v0.14.1 python3 ...   # reads one given tag or commit

No dependency. Sources, all in this repository at that release:
  crates/server/tests/fixtures/collectors_catalog.json  the catalogue served by GET /api/collectors
                                                          (labels, summaries, credentials, setup notice)
  docs/devices/index.md                                   the documentation page and anchor of each type
  docs/devices/<page>.md                                  which built-in rules apply to the type
  docs/alerting/rules.md                                  the built-in rules (name, description, severity)
  crates/server/src/notify/catalog.rs                     the notification channels
  crates/server/src/notify/channel.rs                     their order (CHANNEL_KINDS)
  Cargo.toml                                              the version shown on the site

Device texts (labels, summaries, rule descriptions) are English, as in the catalogue and the docs.
Badges come from the catalogue itself: "(beta)" in the label or a summary starting with "Beta.",
and a summary starting with "Needs the agent" (collected through the agent).
"""
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STABLE = re.compile(r"^v(\d+)\.(\d+)\.(\d+)$")


class Source:
    """Reads the project files at one ref (a release tag) or in the working tree."""

    def __init__(self, ref=None):
        self.ref = ref  # None: the working tree

    def read(self, path):
        """The text of path, or None when the file does not exist there."""
        if self.ref is None:
            f = ROOT / path
            return f.read_text() if f.exists() else None
        r = subprocess.run(["git", "-C", str(ROOT), "show", f"{self.ref}:{path}"], capture_output=True)
        return r.stdout.decode() if r.returncode == 0 else None

    def exists(self, path):
        return self.read(path) is not None

    def label(self):
        return self.ref or "working tree"


def latest_release():
    """The highest stable release tag (vX.Y.Z, no -rc), or None (no git, no tags)."""
    try:
        r = subprocess.run(["git", "-C", str(ROOT), "tag", "--list", "v*"], capture_output=True, text=True, check=True)
    except (OSError, subprocess.CalledProcessError):
        return None
    tags = [t for t in r.stdout.split() if STABLE.match(t)]
    return max(tags, key=lambda t: tuple(int(x) for x in STABLE.match(t).groups()), default=None)


def source():
    """The Source the site describes: DUMBMONIT_SITE_REF if set, else the latest release tag."""
    ref = os.environ.get("DUMBMONIT_SITE_REF", "").strip()
    if ref == "worktree":
        return Source(None)
    if ref:
        return Source(ref)
    tag = latest_release()
    if tag is None:
        print("warning: no release tag found (git fetch --tags?), reading the working tree", file=sys.stderr)
        return Source(None)
    return Source(tag)


# Families of the website, in display order. A catalogue type missing here is not lost: it lands
# in the "more" family (shown only when not empty), so a new collector always appears on the site.
# A type listed here but absent from the release is simply skipped.
FAMILIES = [
    ("servers", ["agent", "winrm", "redfish", "nut", "activedirectory", "prometheus", "otlp"]),
    ("network", ["snmp", "snmp_oid", "mikrotik", "unifi", "meraki", "omada", "arubacentral", "tailscale", "openvpn", "radius", "netconfig"]),
    ("cloud", ["m365", "entra", "azure", "aws", "cloudflare", "hetzner", "ovh", "infomaniak"]),
    ("firewalls", ["opnsense", "pfsense", "fortigate", "sophos", "paloalto", "sonicwall"]),
    ("hypervisors", ["proxmox", "pdm", "vsphere", "xcpng", "nutanix", "kubernetes"]),
    ("storage", ["synology", "truenas", "unraid", "qnap", "ceph", "minio", "pbs", "veeam", "kopia", "duplicati"]),
    ("checks", ["http", "tcp", "udp", "dns", "ping", "tls", "ntp", "ssh", "grpc", "websocket", "push", "domain", "provider", "website", "webchange"]),
    ("proxies", ["traefik", "caddy", "nginx", "apache", "npm"]),
    ("apps", ["nextcloud", "immich", "paperless", "jellyfin", "plex", "homeassistant", "gitlab", "forgejo", "keycloak", "authentik", "vaultwarden", "unifiprotect", "frigate", "sonarr", "radarr", "lidarr", "prowlarr", "qbittorrent"]),
    ("mail", ["pihole", "adguard", "crowdsec", "smtp", "imap", "pop3", "mdaemon", "securitygateway", "pmg", "mailcow", "mail_blacklist", "ip_blacklist"]),
    ("data", ["postgres", "mysql", "mssql", "oracle", "mongodb", "redis", "rabbitmq", "kafka", "mqtt", "elasticsearch", "influxdb", "victoriametrics", "victorialogs", "loki", "graylog"]),
]
FALLBACK_FAMILY = "more"
# Not shown: a test entry of the catalogue, the demo monitor, and the services an agent
# finds (created by the agent, never added by hand).
SKIP = {"zz-inconnu", "dummy", "agent_service"}

# Types whose doc has no "built-in rules" sentence: the sections of rules.md that apply.
RULE_SECTIONS = {
    "snmp": ["Every monitor", "Network ports (SNMP)"],
    "proxmox": ["Proxmox VE"],
    "pbs": ["Proxmox Backup Server"],
    "pdm": ["Proxmox Datacenter Manager"],
    "pmg": ["Proxmox Mail Gateway"],
    "opnsense": ["OPNsense"],
    "truenas": ["TrueNAS"],
    "synology": ["Synology DSM"],
    "agent": ["Every monitor", "Agent disks, ZFS and sensors", "Docker containers"],
    "redfish": ["Server hardware (Redfish)"],
    "webchange": ["Website changes"],
}
# Uptime-monitor rules that apply to each probe type (rules.md lists them together).
_DOWN = ["Service down", "Service flapping", "Slow service"]
_CERT = ["Certificate expiring soon", "Certificate expired"]
RULE_NAMES = {
    "http": _DOWN + _CERT, "tls": ["Service down"] + _CERT, "tcp": _DOWN, "dns": _DOWN, "ping": _DOWN,
    "ntp": _DOWN, "websocket": _DOWN + _CERT, "mqtt": _DOWN, "smtp": _DOWN + _CERT, "postgres": _DOWN,
    "mysql": _DOWN, "push": ["Heartbeat missed"], "website": ["Service down", "Website changed"],
}


def squash(s):
    return re.sub(r"\s+", " ", s.replace("**", "").replace("`", "")).strip()


def parse_rules(src):
    """{section title: [rule]} from the tables of docs/alerting/rules.md."""
    text = src.read("docs/alerting/rules.md") or ""
    text = text.split("## Editing a rule in the UI")[0]
    sections, cur = {}, None
    for line in text.splitlines():
        if line.startswith("### "):
            cur = line[4:].strip()
            sections[cur] = []
        elif cur and line.startswith("|") and not line.startswith("|---") and not line.startswith("| Rule "):
            cells = [c.strip() for c in re.split(r"(?<!\\)\|", line.strip().strip("|"))]
            if len(cells) < 6:
                continue
            name, what, _thr, _hold, sev = cells[:5]
            desc = re.sub(r"\s*\(`[^)]*`[^)]*\)", "", what)  # drop the "(`metric`)" asides
            desc = desc.replace("`", "").strip()
            sev = re.match(r"([A-Za-z]+)", sev)
            sections[cur].append({"name": name, "description": desc, "severity": (sev.group(1) if sev else "").lower()})
    return sections


def parse_index(src):
    """{kind: {page, anchor, typical, credential}} from the table of docs/devices/index.md."""
    out = {}
    for line in (src.read("docs/devices/index.md") or "").splitlines():
        m = re.match(r"\|\s*\[[^\]]+\]\(([a-z0-9_-]+)\.md(#[^)]*)?\)\s*\|\s*`([a-z0-9_]+)`\s*\|([^|]*)\|(.*)\|\s*$", line)
        if m:
            page, anchor, kind, cred, typical = m.groups()
            typical = re.sub(r"\[([^\]]+)\]\([^)]*\)", r"\1", typical).strip()
            out[kind] = {"page": page, "anchor": anchor or "", "credential": cred.strip(), "typical": typical}
    return out


def listed(text, name):
    """True when `name` is one item of a list in `text`, not the tail of a longer rule name."""
    for m in re.finditer(re.escape(name), text):
        before = text[: m.start()].rstrip()
        if not before or before[-1] in ":,;(-." or before.endswith(("and", "or", "plus", "apply", "applies")):
            return True
    return False


def rules_for(src, kind, page, sections, by_name):
    chosen = []
    text = src.read(f"docs/devices/{page}.md")
    block = ""
    if text and kind not in ("agent",):
        m = re.search(r"built-in rules?\]", text)
        if m:
            chunks = re.split(r"\n\s*\n", text[m.start():])
            kept = [chunks[0]] + [c for c in chunks[1:3] if c.lstrip().startswith(("-", "*"))]
            block = squash(" ".join(kept))
    if block:
        low = block.lower()
        for name in by_name:
            if name != "Monitor unreachable" and listed(low, name.lower()):
                chosen.append(by_name[name])
    unreachable = by_name.get("Monitor unreachable")
    if block and unreachable:
        chosen.append(unreachable)
    for name in RULE_NAMES.get(kind, []):
        if name in by_name:
            chosen.append(by_name[name])
    for title in RULE_SECTIONS.get(kind, []):
        chosen.extend(sections.get(title, []))
    if not chosen and unreachable:
        chosen.append(unreachable)
    seen, out = set(), []
    for r in chosen:
        if r["name"] not in seen:
            seen.add(r["name"])
            out.append(r)
    return out


def parse_channels(src):
    order = re.search(r"CHANNEL_KINDS: &\[&str\] = &\[(.*?)\];", src.read("crates/server/src/notify/channel.rs") or "", re.S)
    if not order:
        raise SystemExit("notify/channel.rs: CHANNEL_KINDS not found")
    kinds = re.findall(r'"([a-z]+)"', re.sub(r"//[^\n]*", "", order.group(1)))
    text = src.read("crates/server/src/notify/catalog.rs") or ""
    labels = {}
    for m in re.finditer(r'"([a-z]+)" => info\(\s*kind,\s*"((?:[^"\\]|\\.)*)"', text):
        labels[m.group(1)] = m.group(2)
    missing = [k for k in kinds if k not in labels]
    if missing:
        raise SystemExit(f"notify/catalog.rs: no label found for {missing}")
    return [{"kind": k, "label": labels[k]} for k in kinds]


def version(src):
    """The workspace version at that release (0.9.0-rc.1 shows as 0.9.0)."""
    m = re.search(r'^version\s*=\s*"([^"]+)"', src.read("Cargo.toml") or "", re.M)
    if not m:
        raise SystemExit("Cargo.toml: no version")
    return re.sub(r"-rc\.\d+$", "", m.group(1))


def doc_fallback(src, kind):
    """The doc page of a type missing from docs/devices/index.md: its own page, a section of
    services.md (## UDP, ## IMAP and POP3, ...), or the index."""
    if src.exists(f"docs/devices/{kind}.md"):
        return {"page": kind, "anchor": ""}
    for line in (src.read("docs/devices/services.md") or "").splitlines():
        if line.startswith("## "):
            slug = re.sub(r"[^a-z0-9 -]", "", line[3:].strip().lower()).replace(" ", "-")
            if kind in slug.split("-"):
                return {"page": "services", "anchor": "#" + slug}
    return {"page": "index", "anchor": ""}


def badges(d):
    """(clean label, clean summary, badges) from the catalogue's own wording."""
    label, summary, out = d["label"], d["summary"], []
    if re.search(r"\s*\(beta\)\s*$", label, re.I) or re.match(r"Beta\.\s", summary):
        out.append("beta")
    label = re.sub(r"\s*\(beta\)\s*$", "", label, flags=re.I)
    summary = re.sub(r"^Beta\.\s+", "", summary)
    if re.match(r"Needs the agent\b", summary):
        out.append("agent")
    return label, summary, out


def build(src=None):
    src = src or source()
    text = src.read("crates/server/tests/fixtures/collectors_catalog.json")
    if text is None:
        raise SystemExit(f"{src.label()}: no collectors_catalog.json")
    catalog = {d["kind"]: d for d in json.loads(text)}
    index = parse_index(src)
    sections = parse_rules(src)
    by_name = {r["name"]: r for rows in sections.values() for r in rows}
    shown = [k for k in catalog if k not in SKIP]
    placed = {k for _f, ks in FAMILIES for k in ks}
    fams = [(f, [k for k in ks if k in catalog]) for f, ks in FAMILIES]
    rest = sorted((k for k in shown if k not in placed), key=lambda k: catalog[k]["label"].lower())
    if rest:
        print(f"note: {rest} not in a family of extract.py, shown under '{FALLBACK_FAMILY}'", file=sys.stderr)
        fams.append((FALLBACK_FAMILY, rest))
    devices, families = [], []
    for fam, kinds in fams:
        if not kinds:
            continue
        families.append({"id": fam, "kinds": kinds})
        for kind in kinds:
            d = catalog[kind]
            idx = index.get(kind)
            if not idx:
                idx = {"credential": "", "typical": "", **doc_fallback(src, kind)}
            setup = d.get("setup") or {}
            creds = [{"kind": c["kind"], "label": c["label"], "help": c.get("help", "")} for c in d.get("credentials", [])]
            label, summary, marks = badges(d)
            devices.append({
                "kind": kind,
                "label": label,
                "family": fam,
                "summary": summary,
                "badges": marks,
                "examples": d.get("examples", []),
                "typical": idx["typical"],
                "credential": idx["credential"],
                "credentials": creds,
                "default_port": d.get("default_port", 0),
                "setup": {"title": setup.get("title", ""), "steps": setup.get("steps", []), "warning": setup.get("warning", "")},
                "doc": {"page": idx["page"], "anchor": idx["anchor"]},
                "rules": rules_for(src, kind, idx["page"], sections, by_name),
            })
    return {
        "release": src.label(),
        "version": version(src),
        "devices": devices,
        "families": families,
        "channels": parse_channels(src),
    }


def main():
    d = build()
    if "--json" in sys.argv:
        print(json.dumps(d, indent=1, ensure_ascii=False))
        return
    beta = sum("beta" in x["badges"] for x in d["devices"])
    print(
        f"{d['release']} ({d['version']}): {len(d['devices'])} device types ({beta} beta), "
        f"{len(d['families'])} families, {len(d['channels'])} channels"
    )


if __name__ == "__main__":
    main()
