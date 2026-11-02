# Synology agent (DSM 7)

A small package that runs the DumbMonit agent on a Synology NAS. It reports CPU,
memory, disks and network. For volumes, storage pools, disk health and backup
tasks, see [Synology DSM](synology.md), which uses the DSM API.

The agent only makes outbound connections to your DumbMonit server: the package
opens no port on the NAS. It runs under a dedicated package user, not as root.

## Install

1. In DumbMonit, open **Add monitor**, choose **Server with the agent** and
   create an enrollment token. Keep the page open: the token is shown there.
2. Download the `.spk` matching your NAS (`x86_64` or `aarch64`) from the
   DumbMonit releases.
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

## Uninstall

In **Package Center**, open **DumbMonit Agent**, choose **Uninstall**. The
configuration and the token are deleted with it. Then delete the device in
DumbMonit if you no longer want it listed.

## Limits

- DSM 7.0 or later only. DSM 6 is not supported.
- Only what the agent collects on any Linux machine; no Synology-specific
  volumes, RAID or backup data from this package.
- The wizard does not ask for the encrypted agent channel settings yet.
