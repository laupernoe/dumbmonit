# Passkeys

A passkey lets you sign in with your fingerprint, your face, a device PIN or a
security key (YubiKey…) instead of typing a password. It replaces the password
**and** the second factor in a single gesture: DumbMonit requires the device to
verify you (biometrics or PIN) and never asks for the TOTP code after a passkey.

Passkeys are **in addition** to the password: you keep signing in with your
password (and TOTP) when you want to or when no passkey is at hand.

## Requirements

- **HTTPS.** Browsers only offer passkeys on secure pages: put DumbMonit behind
  a reverse proxy with TLS. `http://localhost` is the one exception (handy to
  try it).
- **A domain name**, not an IP address. A passkey is bound to the domain it was
  created on (the *relying party ID*, here the host name you type in the browser).
  Open `https://monit.example.org`, not `https://192.168.1.10`.
- Set `DUMBMONIT_PUBLIC_URL` (or **Settings → Notifications → Public URL**) to
  that address: DumbMonit then refuses to create passkeys from any other address,
  instead of letting you register keys that would only work there. Without it, the
  address of the page you are on is used.
- Local accounts only. Accounts signing in through LDAP / Active Directory or SSO
  keep using their provider, which can revoke them.

If you change the domain, existing passkeys stop working (the browser will not
offer them); the password is still there to sign in and add new ones.

## Add a passkey

**Settings → Account & security → Passkeys → Add a passkey.** Give it a name
(Laptop, Phone, YubiKey), confirm with your password, then follow the browser's
prompt. Up to 20 passkeys per account: register one per device, and a security key
as a spare.

Passkeys kept in a password manager or the system keychain (iCloud, Google) are
marked **Synced**: they follow you from one device to another.

## Sign in

On the sign-in page, **Sign in with a passkey** (shown as soon as one passkey exists
on the instance and the browser supports them). No username to type: the browser
lists your passkeys for this site.

## Manage

In the same section: **rename** or **remove** a passkey. Removing a lost device's
passkey is enough to cut its access. Adding, removing and using a passkey are in
the security audit log (admins).

## Under the hood

- WebAuthn, algorithm ES256 (ECDSA P-256), attestation `none`, discoverable
  credentials, user verification required.
- DumbMonit stores the *public* key only: a copy of the database signs nobody in.
- The signature counter is checked when the authenticator keeps one (a counter that
  goes back means a cloned key); synced passkeys report 0, which is accepted.
- The verification is done in DumbMonit itself, with no OpenSSL: the usual
  `webauthn-rs` crate depends on it, which the static musl image does not allow.
- Passkeys are not part of the instance backup (they depend on the domain): after
  a restore on another address, register them again.
