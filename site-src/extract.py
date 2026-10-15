#!/usr/bin/env python3
"""Extracts the device catalogue of the website from the real sources of the project.

    python3 site-src/extract.py          # writes site-src/data/devices.json
    python3 site-src/extract.py --check  # fails if that file is out of date

No dependency. Sources, all in this repository:
  crates/server/tests/fixtures/collectors_catalog.json  the catalogue served by GET /api/collectors
                                                          (labels, summaries, credentials, setup notice)
  docs/devices/index.md                                   the documentation page and anchor of each type
  docs/devices/<page>.md                                  which built-in rules apply to the type
  docs/alerting/rules.md                                  the built-in rules (name, description, severity)
  crates/server/src/notify/catalog.rs                     the notification channels
  crates/server/src/notify/channel.rs                     their order (CHANNEL_KINDS)

Device texts (labels, summaries, rule descriptions) are English, as in the catalogue and the docs.
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = Path(__file__).resolve().parent / "data" / "devices.json"

# Families of the website, in display order. Every catalogue type must appear exactly once
# (the build fails otherwise, so a new collector cannot be forgotten).
FAMILIES = [
    ("servers", ["agent", "redfish", "nut", "activedirectory"]),
    ("network", ["snmp", "mikrotik", "unifi", "tailscale", "openvpn"]),
    ("firewalls", ["opnsense", "pfsense", "fortigate", "sophos"]),
    ("hypervisors", ["proxmox", "pdm", "vsphere", "xcpng", "nutanix", "kubernetes"]),
    ("storage", ["synology", "truenas", "unraid", "pbs", "veeam"]),
    ("checks", ["http", "tcp", "dns", "ping", "tls", "ntp", "websocket", "push", "domain", "website", "webchange"]),
    ("proxies", ["traefik", "caddy", "nginx", "apache", "npm"]),
    ("apps", ["nextcloud", "immich", "paperless", "jellyfin", "plex", "homeassistant", "gitlab", "forgejo"]),
    ("mail", ["pihole", "adguard", "crowdsec", "smtp", "mdaemon", "securitygateway", "pmg"]),
    ("data", ["postgres", "mysql", "mongodb", "redis", "rabbitmq", "mqtt", "victoriametrics", "victorialogs", "loki", "graylog"]),
]
# Not shown: a test entry of the catalogue, the demo device, and the services an agent
# finds (created by the agent, never added by hand).
SKIP = {"zz-inconnu", "dummy", "agent_service"}

# Types whose doc has no "built-in rules" sentence: the sections of rules.md that apply.
RULE_SECTIONS = {
    "snmp": ["Every device", "Network ports (SNMP)"],
    "proxmox": ["Proxmox VE"],
    "pbs": ["Proxmox Backup Server"],
    "pdm": ["Proxmox Datacenter Manager"],
    "pmg": ["Proxmox Mail Gateway"],
    "opnsense": ["OPNsense"],
    "truenas": ["TrueNAS"],
    "synology": ["Synology DSM"],
    "agent": ["Every device", "Agent disks, ZFS and sensors", "Docker containers"],
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


def parse_rules():
    """{section title: [rule]} from the tables of docs/alerting/rules.md."""
    text = (ROOT / "docs/alerting/rules.md").read_text()
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


def parse_index():
    """{kind: {page, anchor, typical, credential}} from the table of docs/devices/index.md."""
    out = {}
    for line in (ROOT / "docs/devices/index.md").read_text().splitlines():
        m = re.match(r"\|\s*\[[^\]]+\]\(([a-z0-9_-]+)\.md(#[^)]*)?\)\s*\|\s*`([a-z]+)`\s*\|([^|]*)\|(.*)\|\s*$", line)
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


def rules_for(kind, page, sections, by_name):
    chosen = []
    doc = ROOT / "docs/devices" / f"{page}.md"
    block = ""
    if doc.exists() and kind not in ("agent",):
        text = doc.read_text()
        m = re.search(r"built-in rules?\]", text)
        if m:
            chunks = re.split(r"\n\s*\n", text[m.start():])
            kept = [chunks[0]] + [c for c in chunks[1:3] if c.lstrip().startswith(("-", "*"))]
            block = squash(" ".join(kept))
    if block:
        low = block.lower()
        for name in by_name:
            if name != "Device unreachable" and listed(low, name.lower()):
                chosen.append(by_name[name])
        chosen.append(by_name["Device unreachable"])
    for name in RULE_NAMES.get(kind, []):
        chosen.append(by_name[name])
    for title in RULE_SECTIONS.get(kind, []):
        chosen.extend(sections.get(title, []))
    if not chosen:
        chosen.append(by_name["Device unreachable"])
    seen, out = set(), []
    for r in chosen:
        if r["name"] not in seen:
            seen.add(r["name"])
            out.append(r)
    return out


def parse_channels():
    order = re.search(r"CHANNEL_KINDS: &\[&str\] = &\[(.*?)\];", (ROOT / "crates/server/src/notify/channel.rs").read_text(), re.S)
    kinds = re.findall(r'"([a-z]+)"', re.sub(r"//[^\n]*", "", order.group(1)))
    src = (ROOT / "crates/server/src/notify/catalog.rs").read_text()
    labels = {}
    for m in re.finditer(r'"([a-z]+)" => info\(\s*kind,\s*"((?:[^"\\]|\\.)*)"', src):
        labels[m.group(1)] = m.group(2)
    missing = [k for k in kinds if k not in labels]
    if missing:
        raise SystemExit(f"notify/catalog.rs: no label found for {missing}")
    return [{"kind": k, "label": labels[k]} for k in kinds]


def build():
    catalog = {d["kind"]: d for d in json.loads((ROOT / "crates/server/tests/fixtures/collectors_catalog.json").read_text())}
    index = parse_index()
    sections = parse_rules()
    by_name = {r["name"]: r for rows in sections.values() for r in rows}
    placed = [k for _f, ks in FAMILIES for k in ks]
    expected = set(catalog) - SKIP
    if set(placed) != expected or len(placed) != len(set(placed)):
        raise SystemExit(
            f"families do not match the catalogue: missing {sorted(expected - set(placed))}, "
            f"unknown {sorted(set(placed) - expected)}"
        )
    devices, families = [], []
    for fam, kinds in FAMILIES:
        families.append({"id": fam, "kinds": kinds})
        for kind in kinds:
            d = catalog[kind]
            idx = index.get(kind)
            if not idx:
                raise SystemExit(f"{kind}: not in docs/devices/index.md")
            setup = d.get("setup") or {}
            creds = []
            for c in d.get("credentials", []):
                creds.append({
                    "kind": c["kind"],
                    "label": c["label"],
                    "help": c.get("help", ""),
                })
            devices.append({
                "kind": kind,
                "label": d["label"],
                "family": fam,
                "summary": d["summary"],
                "examples": d.get("examples", []),
                "typical": idx["typical"],
                "credential": idx["credential"],
                "credentials": creds,
                "default_port": d.get("default_port", 0),
                "setup": {"title": setup.get("title", ""), "steps": setup.get("steps", []), "warning": setup.get("warning", "")},
                "doc": {"page": idx["page"], "anchor": idx["anchor"]},
                "rules": rules_for(kind, idx["page"], sections, by_name),
            })
    return {"devices": devices, "families": families, "channels": parse_channels()}


def main():
    text = json.dumps(build(), indent=1, ensure_ascii=False) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            raise SystemExit("site-src/data/devices.json is out of date, run site-src/extract.py")
        return
    OUT.parent.mkdir(exist_ok=True)
    OUT.write_text(text)
    d = json.loads(text)
    print(f"{len(d['devices'])} device types, {len(d['families'])} families, {len(d['channels'])} channels")


if __name__ == "__main__":
    main()
