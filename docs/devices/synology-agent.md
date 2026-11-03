# Synology agent (DSM 7)

A small package that runs the DumbMonit agent on a Synology NAS. Besides CPU,
memory, disks and network, it reads the NAS itself: RAID arrays, volumes, disk
temperatures, installed packages, file services and the UPS (see
[What is monitored](#what-is-monitored)). Hyper Backup tasks, DSM updates and the
SMART verdict of DSM need [Synology DSM](synology.md), which uses the DSM API. Both
can run on the same NAS; [Package or API?](synology.md#package-or-api) compares them.

The agent only makes outbound connections to your DumbMonit server: the package
opens no port on the NAS. It runs under a dedicated package user, not as root.

## Install

1. In DumbMonit, open **Add a monitor**, choose **Synology DSM**, then
   **DumbMonit Agent package**. The page creates an enrollment token and shows
   the server URL and certificate pin the wizard asks for. Keep the page open:
   the token is shown there.
2. Download the `.spk` matching your NAS (`x86_64` or `aarch64`) from the same
   page, or from the DumbMonit releases page on GitHub.
3. In DSM, open **Package Center**, click **Manual Install**, and select the file.
   DSM warns that the publisher is unknown: the package is not signed yet.
4. In the wizard, enter the server URL (HTTPS only, for example
   `https://192.168.1.10:8080`), paste the token, and paste the server
   certificate fingerprint (pin, `sha256//...`) shown in **Settings > System >
   HTTPS**. The pin can only be left empty if the server has a certificate from
   a trusted authority; with the default self-signed certificate the
   installation fails until you paste it.
5. Finish. The NAS appears in DumbMonit within a minute. If the server is
   unreachable, the pin does not match or the token is refused, the installation stops with a message:
   fix the address or the pin, or create a new token and retry.

The agent still verifies its own updates against the DumbMonit signing key.

## What is monitored

The agent detects DSM by `/etc.defaults/VERSION` and `/usr/syno`, then reads,
without root:

| What | Source | Shown as |
|---|---|---|
| Model and DSM version | `/etc.defaults/VERSION`, `synoinfo.conf` (`upnpmodelname`) | Header of the NAS panel, NAS fleet |
| RAID arrays (data, system, swap, SSD cache) | `/proc/mdstat`: missing or failed member, rebuild, resync, check | RAID arrays, alert **NAS RAID degraded** |
| Volumes | `/proc/mounts` (`/volumeN`) and the usual disk usage | Volumes, alerts **NAS volume read-only** and **Disk almost full** |
| Disks | `/sys/block` (model, size, HDD/SSD/NVMe); temperature, remaining SSD life and unreadable sectors from `/run/synostorage/disks` when DSM exposes them | Disks, alerts **NAS disk temperature high**, **NAS disk bad sectors growing**, **NAS SSD wearing out** |
| Packages (Drive, Photos, Hyper Backup, Active Backup, Container Manager…) | `/var/packages/*/INFO` and the `enabled` marker; running state from `synopkg status` when DSM lets the package user run it | Packages, alert **NAS package stopped** |
| SMB, NFS, SSH | Their process (`smbd`, `nfsd`, `sshd`) | On or Off in the panel, no alert |
| UPS | `upsc`, when the DSM UPS server runs (UPS on USB, or network UPS server on) | UPS in the panel, the [UPS behind NUT](../alerting/rules.md#ups-behind-nut) alerts |
| CPU temperature | `/sys/class/hwmon` | Temperature in the panel |

The serial number is never read. A source that is missing or unreadable is
skipped: nothing is invented, and no alert fires on what was not read. The
storage health of the NAS (0 healthy, 1 attention, 2 critical) feeds the NAS
fleet view, like the DSM API monitor.

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

## Uninstall

In **Package Center**, open **DumbMonit Agent**, choose **Uninstall**. The
configuration and the token are deleted with it. Then delete the device in
DumbMonit if you no longer want it listed.

## Limits

- DSM 7.0 or later only. DSM 6 is not supported.
- No Hyper Backup or Active Backup results, no DSM or package updates, no
  SMART verdict from DSM, no active connections: the package user cannot read
  them. Use the [DSM API](synology.md) for these.
- The wizard does not ask for the encrypted agent channel settings yet.
