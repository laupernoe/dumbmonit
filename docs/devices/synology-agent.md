# Synology agent (DSM 7)

A small package that runs the DumbMonit agent on a Synology NAS. Besides CPU,
memory, disks and network, it reads the NAS itself: RAID arrays, volumes, disk
temperatures, installed packages, file services and the UPS (see
[What is monitored](#what-is-monitored)). Hyper Backup tasks, DSM updates and the
SMART verdict of DSM need [Synology DSM](synology.md), which uses the DSM API. Both
can run on the same NAS; [Package or API?](synology.md#package-or-api) compares them.

The agent only makes outbound connections to your DumbMonit server: the package
opens no port on the NAS. It runs under a dedicated package user, not as root.

## Before you start

- A Synology NAS on **DSM 7.0 or later** (DSM 6 is not supported; use the
  [DSM API](synology.md) there).
- You know the NAS architecture, `x86_64` or `aarch64`. The package page offers
  both downloads. If unsure, look up the model in Synology's specifications.
- The NAS can open an HTTPS connection to the DumbMonit server (usually
  `https://<server>:8080`). Nothing needs to reach the NAS.

## Install

1. In DumbMonit, click **Add a monitor**, pick a site (or **No site**), choose
   **Synology DSM**, then **Use the package** (*DumbMonit Agent package*).
   You should see a page with **Download the package for your NAS**, a
   **Server URL**, a **Certificate pin** and a token. Keep this page open: the
   token is only shown here.
2. Click the download for your NAS (`x86_64` or `aarch64`), or take the `.spk`
   from the DumbMonit releases page on GitHub.
3. In DSM, open **Package Center**, click **Manual Install**, and select the
   file. DSM warns that the publisher is unknown: the package is not signed yet.
   Accept to continue.
4. The wizard asks for three values. Copy each one from the DumbMonit page:
    - **Server URL**: HTTPS only, for example `https://192.168.1.10:8080`.
    - **Enrollment token**: the value shown on the page.
    - **Certificate pin** (`sha256//...`): paste it exactly as shown. Leave it
      empty only if the server has a certificate from a trusted authority. With
      the default self-signed certificate the installation fails without it.
5. Finish. The NAS appears in DumbMonit within a minute, with its model and DSM
   version.

If the server is unreachable, the pin does not match or the token is refused,
the installation stops with a message and nothing stays installed.

### If the installation fails

| Message or symptom | What to do |
|---|---|
| The URL is refused by the wizard | It must start with `https://`. DumbMonit only serves HTTPS. |
| The installation stops, server unreachable | From the NAS, the server address must answer. Check the address, the port (8080 by default) and any firewall between them. |
| The installation stops, pin does not match | Copy the pin again from the add page, without spaces. If you replaced the server certificate, the pin changed. |
| The installation stops, token refused | The token was used, revoked or mistyped. Go back to the add page, create a new one, and retry. |
| The package is installed but the NAS never appears | In **Package Center**, check that **DumbMonit Agent** is running, then wait a minute. |

**Changed port or reverse proxy.** If you changed the web port (for example
`8090`) or reach DumbMonit through a reverse proxy (such as the DSM one), open the
add page at the exact address the NAS will use: DumbMonit then shows the pin of
the certificate that address really presents. By hand, use that same URL in the
wizard, and leave the pin empty if the address has a trusted certificate.

The agent still verifies its own updates against the DumbMonit signing key.

## What is monitored

The agent detects DSM by `/etc.defaults/VERSION` and `/usr/syno`, then reads,
without root:

| What | Source | Shown as |
|---|---|---|
| Model and DSM version | `/etc.defaults/VERSION`, `synoinfo.conf` (`upnpmodelname`) | Header of the NAS panel |
| RAID arrays (data, system, swap, SSD cache) | `/proc/mdstat`: missing or failed member, rebuild, resync, check | RAID arrays, alert **NAS RAID degraded** |
| Volumes | `/proc/mounts` (`/volumeN`) and the usual disk usage | Volumes, alerts **NAS volume read-only** and **Disk almost full** |
| Disks | `/sys/block` (model, size, HDD/SSD/NVMe); temperature, remaining SSD life and unreadable sectors from `/run/synostorage/disks` when DSM exposes them | Disks, alerts **NAS disk temperature high**, **NAS disk bad sectors growing**, **NAS SSD wearing out** |
| Packages (Drive, Photos, Hyper Backup, Active Backup, Container Manager…) | `/var/packages/*/INFO` and the `enabled` marker; running state from `synopkg status` when DSM lets the package user run it | Packages, alert **NAS package stopped** |
| SMB, NFS, SSH | Their process (`smbd`, `nfsd`, `sshd`) | On or Off in the panel, no alert |
| UPS | `upsc`, when the DSM UPS server runs (UPS on USB, or network UPS server on) | UPS in the panel, the [UPS behind NUT](../alerting/rules.md#ups-behind-nut) alerts |
| CPU temperature | `/sys/class/hwmon` | Temperature in the panel |

The serial number is never read. A source that is missing or unreadable is
skipped: nothing is invented, and no alert fires on what was not read.

### What needs more than the package user

The package runs as its own user (`conf/privilege`: `run-as: package`), and the
package does not ask for more. Without root:

- **SMART** of the disks (`smartctl`) cannot open the disk devices: no SMART
  verdict, only the temperature and wear DSM publishes in `/run/synostorage`.
- **Package states**: on some DSM versions `synopkg status` is refused to the
  package user. The panel then shows only whether each package is enabled, and
  the **NAS package stopped** alert stays silent.
- **Hyper Backup**, **DSM and package updates**, **SMB connections**: only the DSM
  API reports them. Add a [Synology DSM](synology.md) monitor with a read-only
  account next to the package.

## Update

The agent is the usual DumbMonit agent: it follows your server's version through
[automatic updates](agent.md#automatic-updates) and checks each update against
the DumbMonit signing key. You do not install a new `.spk` after each DumbMonit
upgrade: the package updates itself with the server, no reinstall needed. The
updated binary lives in the package's writable `var` folder and is the one
started by Package Center. A NAS still running a package older than 0.17.1
cannot do this yet: install the current `.spk` once over it (the configuration
is kept), and it follows the server from then on.

## Uninstall

1. In **Package Center**, open **DumbMonit Agent**.
2. Choose **Uninstall**. The configuration and the token are deleted with it.
3. In DumbMonit, delete the monitor if you no longer want it listed.
   Until then it shows as unreachable.

## Limits

- DSM 7.0 or later only. DSM 6 is not supported.
- No Hyper Backup or Active Backup results, no DSM or package updates, no
  SMART verdict from DSM, no active connections: the package user cannot read
  them. Use the [DSM API](synology.md) for these.
- The wizard does not ask for the encrypted agent channel settings yet.
