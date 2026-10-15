//! Démarrage en root : remise en ordre du répertoire de données, puis abandon
//! définitif des privilèges.
//!
//! L'image tourne en 65532:65532, ce qui suffit à un volume nommé. Un bind mount
//! (`./data:/data`, `/mnt/user/appdata/…` sur Unraid) garde, lui, le
//! propriétaire de l'hôte — souvent root, quand Docker crée lui-même le dossier.
//! Lancé en root (`user: "0:0"`, ce que fait docker-compose.yml), le binaire
//! fait alors, avant toute autre chose (base, VictoriaMetrics, réseau) :
//!
//! 1. le choix de l'identité cible : 65532:65532, ou `PUID`/`PGID` ;
//! 2. la correction de la propriété du répertoire de données, entrée par entrée,
//!    sans jamais suivre un lien symbolique (tout est relatif à un descripteur
//!    de répertoire ouvert avec `O_NOFOLLOW`, les changements se font par
//!    `fchown` ou `fchownat(AT_SYMLINK_NOFOLLOW)`) : rien hors de ce répertoire
//!    ne peut être atteint, même si son contenu change pendant le parcours ;
//! 3. `setgroups([])`, `setgid`, `setuid`, dans cet ordre, puis la preuve que le
//!    retour à root est impossible.
//!
//! Sous un autre uid (le défaut de l'image, ou `--user 1000:1000`), il ne touche
//! à rien. Les sous-commandes ne font jamais de chown ; `healthcheck` abandonne
//! seulement root (`drop_if_root`).

use std::ffi::OsStr;
use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use rustix::fs::{AtFlags, Dir, FileType, Gid, Mode, OFlags, Uid};
use tracing::{info, warn};

/// Identité par défaut du serveur dans l'image (uid et gid).
pub const DEFAULT_ID: u32 = 65532;

/// Profondeur au-delà de laquelle le parcours s'arrête : `/data` n'a pas dix
/// niveaux, et la récursion garde deux descripteurs par niveau.
const MAX_DEPTH: usize = 64;

/// Nombre d'erreurs détaillées dans le journal ; les suivantes sont comptées.
const MAX_REPORTED_ERRORS: usize = 10;

// Numéros des capacités Linux utiles ici (linux/capability.h).
const CAP_CHOWN: u32 = 0;
const CAP_DAC_OVERRIDE: u32 = 1;
const CAP_DAC_READ_SEARCH: u32 = 2;
const CAP_SETGID: u32 = 6;
const CAP_SETUID: u32 = 7;

/// Capacités à rendre au conteneur quand le fichier Compose les retire toutes.
pub const REQUIRED_CAPS: &str = "CHOWN, DAC_READ_SEARCH, SETUID, SETGID";

/// Répertoires système qu'un `DUMBMONIT_DATA_DIR` mal réglé ne doit jamais
/// faire changer de propriétaire (utile hors de l'image, lancé en root).
const FORBIDDEN_ROOTS: &[&str] = &[
    "/", "/bin", "/boot", "/dev", "/etc", "/home", "/lib", "/lib64", "/opt", "/proc", "/root",
    "/run", "/sbin", "/srv", "/sys", "/tmp", "/usr", "/var", "/agents",
];

/// Identité sous laquelle le serveur tourne une fois les privilèges abandonnés.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub uid: u32,
    pub gid: u32,
}

/// Ce que le démarrage a fait, pour expliquer plus tard un `/data` illisible.
#[derive(Debug, Clone, Copy, Default)]
pub struct Startup {
    /// Le processus a démarré en root et a corrigé (ou tenté de corriger) `/data`.
    pub started_as_root: bool,
    /// Root s'est vu refuser l'accès à une entrée : SELinux, AppArmor, NFS…
    pub denied_as_root: bool,
}

/// Lit un uid ou un gid : décimal, ni 0 (root) ni 4294967295 (le « -1 » des
/// appels système, qui veut dire « ne change rien »).
///
/// 0 est refusé sans échappatoire : tout l'intérêt de ce module est qu'aucun
/// code du serveur ne tourne en root, et un fichier appartenant à root se
/// corrige tout seul au démarrage. Qui veut vraiment root peut lancer le
/// conteneur avec `--user 0:0`… ce qui revient au même : correction puis
/// abandon des privilèges vers 65532.
pub fn parse_id(name: &str, raw: &str) -> Result<u32, String> {
    let value = raw.trim();
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("{name}={raw:?} is not a numeric id (e.g. {name}=1000)"));
    }
    let id: u32 = value.parse().map_err(|_| format!("{name}={raw:?} is out of range"))?;
    if id == 0 {
        return Err(format!(
            "{name}=0 would run the server as root, which is not supported: use the uid/gid \
             that should own the files (e.g. 1000), or leave it unset for 65532"
        ));
    }
    if id == u32::MAX {
        return Err(format!("{name}={raw:?} is reserved"));
    }
    Ok(id)
}

/// Identité cible d'après l'environnement : `DUMBMONIT_PUID`/`DUMBMONIT_PGID`,
/// sinon `PUID`/`PGID` (convention linuxserver.io, reprise par Unraid), sinon
/// 65532. Chaque valeur est indépendante : régler `PUID` seul garde le gid 65532.
pub fn target_from(get: impl Fn(&str) -> Option<String>) -> Result<Target, String> {
    let pick = |names: [&str; 2]| -> Result<u32, String> {
        for name in names {
            if let Some(raw) = get(name).filter(|value| !value.trim().is_empty()) {
                return parse_id(name, &raw);
            }
        }
        Ok(DEFAULT_ID)
    };
    Ok(Target { uid: pick(["DUMBMONIT_PUID", "PUID"])?, gid: pick(["DUMBMONIT_PGID", "PGID"])? })
}

fn overrides_present(get: impl Fn(&str) -> Option<String>) -> bool {
    ["DUMBMONIT_PUID", "PUID", "DUMBMONIT_PGID", "PGID"]
        .iter()
        .any(|name| get(name).is_some_and(|value| !value.trim().is_empty()))
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// À appeler par le serveur seul, avant d'ouvrir quoi que ce soit : si le
/// processus est root, corrige `data_dir` puis devient définitivement `PUID:PGID`.
pub fn prepare(data_dir: &Path) -> anyhow::Result<Startup> {
    let euid = unsafe { libc::geteuid() };
    if euid != 0 {
        if overrides_present(env) {
            warn!(
                uid = euid,
                "PUID/PGID ignored: they only apply when the container starts as root \
                 (`user: \"0:0\"`); it runs as the image or `user:` says"
            );
        }
        return Ok(Startup::default());
    }

    let target = target_from(env).map_err(anyhow::Error::msg)?;
    let caps = std::fs::read_to_string("/proc/self/status").ok().and_then(|s| effective_caps(&s));
    if let Some(caps) = caps
        && !(has_cap(caps, CAP_SETUID) && has_cap(caps, CAP_SETGID))
    {
        anyhow::bail!(
            "the container runs as root (`user: \"0:0\"`) so that DumbMonit gives {dir} to \
             {uid}:{gid} and then drops root, but its capabilities were all dropped \
             (`cap_drop: [ALL]`), so it cannot switch user. Either give these back under the \
             service in docker-compose.yml:\n  cap_add: [{REQUIRED_CAPS}]\n(--cap-add for each \
             with `docker run`), or remove `user: \"0:0\"` to run as 65532 directly (the files \
             must then already belong to 65532).",
            dir = data_dir.display(),
            uid = target.uid,
            gid = target.gid,
        );
    }

    let mut startup = Startup { started_as_root: true, denied_as_root: false };
    match safe_data_dir(data_dir) {
        Err(reason) => warn!(dir = %data_dir.display(), "ownership not fixed: {reason}"),
        Ok(()) => {
            if let Err(error) = std::fs::create_dir_all(data_dir) {
                warn!(dir = %data_dir.display(), %error, "cannot create the data directory");
            }
            if caps.is_some_and(|caps| !has_cap(caps, CAP_CHOWN)) {
                warn!(
                    "ownership of the data directory not checked: the CHOWN capability is missing"
                );
            } else {
                let can_read_all = caps.is_none_or(|caps| {
                    has_cap(caps, CAP_DAC_OVERRIDE) || has_cap(caps, CAP_DAC_READ_SEARCH)
                });
                let report = fix_ownership(data_dir, target);
                // Root qui a les droits de lecture partout et se voit tout de même
                // refuser l'accès : un module de sécurité (ou NFS) s'en mêle.
                startup.denied_as_root = report.access_denied && can_read_all;
                report.log(data_dir, target);
            }
        }
    }

    drop_privileges(target).map_err(anyhow::Error::msg)?;
    info!(uid = target.uid, gid = target.gid, "root privileges dropped");
    Ok(startup)
}

/// Pour une sous-commande lancée en root (`healthcheck` par le HEALTHCHECK de
/// l'image) : abandonne les privilèges vers `PUID:PGID` sans rien toucher sur
/// disque. Sans root, ne fait rien.
pub fn drop_if_root() -> Result<(), String> {
    // SAFETY: appel sans argument.
    if unsafe { libc::geteuid() } != 0 {
        return Ok(());
    }
    drop_privileges(target_from(env)?)
}

fn safe_data_dir(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("the path is not absolute".into());
    }
    let normalized: PathBuf = path.components().collect();
    if FORBIDDEN_ROOTS.iter().any(|root| normalized == Path::new(root)) {
        return Err("refusing to change the owner of a system directory".into());
    }
    Ok(())
}

/// Bilan d'un parcours de correction.
#[derive(Debug, Default)]
pub struct Report {
    /// Entrées examinées (le répertoire de départ compris).
    pub checked: u64,
    /// Entrées dont le propriétaire a été changé.
    pub changed: u64,
    /// Premières erreurs, avec le chemin concerné.
    pub errors: Vec<String>,
    pub error_count: u64,
    /// Au moins un refus `EACCES`/`EPERM`.
    pub access_denied: bool,
}

impl Report {
    fn fail(&mut self, path: &Path, action: &str, error: io::Error) {
        if error.kind() == io::ErrorKind::PermissionDenied {
            self.access_denied = true;
        }
        self.error_count += 1;
        if self.errors.len() < MAX_REPORTED_ERRORS {
            self.errors.push(format!("{action} {}: {error}", path.display()));
        }
    }

    fn log(&self, root: &Path, target: Target) {
        if self.changed > 0 {
            info!(
                dir = %root.display(),
                changed = self.changed,
                checked = self.checked,
                "data directory handed over to {}:{}", target.uid, target.gid
            );
        }
        for error in &self.errors {
            warn!("fixing ownership: {error}");
        }
        if self.error_count > self.errors.len() as u64 {
            warn!("fixing ownership: {} more errors", self.error_count - self.errors.len() as u64);
        }
    }
}

/// Faut-il changer le propriétaire de cette entrée ?
pub fn needs_change(uid: u32, gid: u32, target: Target) -> bool {
    uid != target.uid || gid != target.gid
}

fn dir_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

/// Donne `root` et tout ce qu'il contient à `target`, sans suivre aucun lien
/// symbolique : un lien change lui-même de propriétaire, jamais sa cible, et un
/// répertoire n'est ouvert que relativement à son parent, avec `O_NOFOLLOW`.
/// Seules les entrées qui n'appartiennent pas déjà à `target` sont modifiées,
/// et une erreur sur une entrée n'arrête pas le parcours.
///
/// Les liens physiques ne sont pas écartés : `fs.protected_hardlinks` (actif
/// par défaut) interdit sur l'hôte d'en créer vers un fichier dont on n'est pas
/// propriétaire, et VictoriaMetrics en crée lui-même pour ses instantanés.
pub fn fix_ownership(root: &Path, target: Target) -> Report {
    let mut report = Report::default();
    match rustix::fs::open(root, dir_flags(), Mode::empty()) {
        Ok(dir) => fix_dir(&dir, root, target, 0, &mut report),
        Err(error) => report.fail(root, "opening", error.into()),
    }
    report
}

fn fix_dir(dir: &OwnedFd, path: &Path, target: Target, depth: usize, report: &mut Report) {
    let owner = Some(Uid::from_raw(target.uid));
    let group = Some(Gid::from_raw(target.gid));

    // Le répertoire lui-même, par son descripteur : c'est bien celui qu'on lit.
    match rustix::fs::fstat(dir) {
        Ok(stat) => {
            report.checked += 1;
            if needs_change(stat.st_uid, stat.st_gid, target) {
                match rustix::fs::fchown(dir, owner, group) {
                    Ok(()) => report.changed += 1,
                    Err(error) => report.fail(path, "changing the owner of", error.into()),
                }
            }
        }
        Err(error) => report.fail(path, "reading", error.into()),
    }

    if depth >= MAX_DEPTH {
        report.fail(path, "descending into", io::Error::other("too many nested directories"));
        return;
    }
    let entries = match Dir::read_from(dir) {
        Ok(entries) => entries,
        Err(error) => return report.fail(path, "listing", error.into()),
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                report.fail(path, "listing", error.into());
                break;
            }
        };
        let name = entry.file_name();
        if name == c"." || name == c".." {
            continue;
        }
        let child = path.join(OsStr::from_bytes(name.to_bytes()));
        let stat = match rustix::fs::statat(dir, name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(stat) => stat,
            Err(error) => {
                report.fail(&child, "reading", error.into());
                continue;
            }
        };
        if FileType::from_raw_mode(stat.st_mode) == FileType::Directory {
            // Remplacé par un lien entre-temps ? `O_NOFOLLOW` fait échouer
            // l'ouverture plutôt que de sortir de l'arborescence.
            match rustix::fs::openat(dir, name, dir_flags(), Mode::empty()) {
                Ok(sub) => fix_dir(&sub, &child, target, depth + 1, report),
                Err(error) => report.fail(&child, "opening", error.into()),
            }
        } else {
            report.checked += 1;
            if needs_change(stat.st_uid, stat.st_gid, target) {
                match rustix::fs::chownat(dir, name, owner, group, AtFlags::SYMLINK_NOFOLLOW) {
                    Ok(()) => report.changed += 1,
                    Err(error) => report.fail(&child, "changing the owner of", error.into()),
                }
            }
        }
    }
}

/// Devient `target` pour de bon : groupes supplémentaires vidés, puis gid, puis
/// uid (l'ordre compte : sans root, `setgid` ne serait plus permis). `setuid`
/// en root change les trois uid (réel, effectif, sauvegardé) ; la libc les
/// applique à tous les threads, et il n'y en a de toute façon qu'un à ce stade.
fn drop_privileges(target: Target) -> Result<(), String> {
    let failed = |call: &str| format!("{call} failed: {}", io::Error::last_os_error());
    // SAFETY: appels système sans pointeur, sauf `setgroups(0, NULL)`, permis
    // par la norme pour une liste vide.
    unsafe {
        if libc::setgroups(0, std::ptr::null()) != 0 {
            return Err(failed("setgroups([])"));
        }
        if libc::setgid(target.gid) != 0 {
            return Err(failed("setgid"));
        }
        if libc::setuid(target.uid) != 0 {
            return Err(failed("setuid"));
        }
    }
    verify_dropped(target)?;
    // Plus aucun exécutable ne pourra rendre de privilèges (VictoriaMetrics hérite).
    #[cfg(target_os = "linux")]
    {
        let (one, zero): (libc::c_ulong, libc::c_ulong) = (1, 0);
        // SAFETY: prctl à arguments entiers.
        if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, one, zero, zero, zero) } != 0 {
            warn!(error = %io::Error::last_os_error(), "cannot set no_new_privs");
        }
    }
    Ok(())
}

fn verify_dropped(target: Target) -> Result<(), String> {
    let (mut ruid, mut euid, mut suid) = (0, 0, 0);
    let (mut rgid, mut egid, mut sgid) = (0, 0, 0);
    // SAFETY: pointeurs vers des variables locales valides.
    let (uids_ok, gids_ok, groups) = unsafe {
        (
            libc::getresuid(&mut ruid, &mut euid, &mut suid) == 0,
            libc::getresgid(&mut rgid, &mut egid, &mut sgid) == 0,
            libc::getgroups(0, std::ptr::null_mut()),
        )
    };
    if !uids_ok || [ruid, euid, suid] != [target.uid; 3] {
        return Err(format!("uid not dropped (real {ruid}, effective {euid}, saved {suid})"));
    }
    if !gids_ok || [rgid, egid, sgid] != [target.gid; 3] {
        return Err(format!("gid not dropped (real {rgid}, effective {egid}, saved {sgid})"));
    }
    if groups != 0 {
        return Err(format!("{groups} supplementary groups left"));
    }
    // La preuve par l'essai : redevenir root doit être refusé.
    // SAFETY: appels système sans pointeur.
    let regained = unsafe { libc::setuid(0) == 0 || libc::seteuid(0) == 0 || libc::setgid(0) == 0 };
    if regained {
        return Err("root privileges could be regained after dropping them".into());
    }
    Ok(())
}

/// Capacités effectives, lues sur la ligne `CapEff:` de `/proc/self/status`.
pub fn effective_caps(status: &str) -> Option<u64> {
    status
        .lines()
        .find_map(|line| line.strip_prefix("CapEff:"))
        .and_then(|hex| u64::from_str_radix(hex.trim(), 16).ok())
}

fn has_cap(caps: u64, cap: u32) -> bool {
    caps & (1 << cap) != 0
}

// ---------------------------------------------------------------------------
// Diagnostic d'un répertoire de données où l'on ne peut pas écrire.
// ---------------------------------------------------------------------------

/// Nature du montage de `/data`, d'après `/proc/self/mountinfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MountKind {
    /// Volume nommé Docker ou Podman (`…/volumes/<nom>/_data`).
    NamedVolume(String),
    /// Bind mount : chemin de la source sur son système de fichiers.
    Bind(String),
    /// Pas un point de montage : le système de fichiers du conteneur.
    NotAMount,
}

/// Retrouve le montage de `path` dans le contenu de `/proc/self/mountinfo`.
pub fn classify_mount(mountinfo: &str, path: &Path) -> MountKind {
    let wanted = path.as_os_str().as_bytes();
    // Le dernier montage sur ce point est celui qui est visible.
    let root = mountinfo
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(' ').collect();
            let mount_point = unescape(fields.get(4)?);
            (mount_point.as_bytes() == wanted).then(|| unescape(fields[3]))
        })
        .next_back();
    let Some(root) = root else { return MountKind::NotAMount };
    let parts: Vec<&str> = root.split('/').collect();
    if let [.., volumes, name, "_data"] = parts.as_slice()
        && *volumes == "volumes"
        && !name.is_empty()
    {
        return MountKind::NamedVolume((*name).to_string());
    }
    MountKind::Bind(root)
}

/// `mountinfo` écrit espace, tabulation, saut de ligne et `\` en octal (`\040`).
fn unescape(field: &str) -> String {
    let bytes = field.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\'
            && i + 3 < bytes.len()
            && bytes[i + 1..i + 4].iter().all(|b| (b'0'..=b'7').contains(b))
        {
            let digit = |k: usize| u32::from(bytes[i + k] - b'0');
            out.push((digit(1) * 64 + digit(2) * 8 + digit(3)) as u8);
            i += 4;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Contexte de sécurité du processus : un contexte SELinux a la forme
/// `utilisateur:rôle:type:niveau` (`system_u:system_r:container_t:s0:c1,c2`),
/// un profil AppArmor non (`docker-default (enforce)`).
pub fn is_selinux_context(current: &str) -> bool {
    let context = current.trim_matches(|c: char| c == '\0' || c.is_whitespace());
    context.split(':').count() >= 3 && !context.contains(' ')
}

/// Ce qu'on sait du répertoire et de l'environnement au moment de l'échec.
#[derive(Debug, Clone)]
pub struct Diagnosis {
    pub uid: u32,
    pub gid: u32,
    /// Propriétaire et mode du répertoire, s'il a pu être lu.
    pub owner: Option<(u32, u32, u32)>,
    pub mount: MountKind,
    pub selinux: bool,
    pub startup: Startup,
}

impl Diagnosis {
    /// Les droits Unix permettraient-ils d'écrire ? `None` si inconnus.
    fn mode_allows(&self) -> Option<bool> {
        let (owner, group, mode) = self.owner?;
        Some(if owner == self.uid {
            mode & 0o300 == 0o300
        } else if group == self.gid {
            mode & 0o030 == 0o030
        } else {
            mode & 0o003 == 0o003
        })
    }
}

/// Message d'erreur qui dit quoi faire, selon la cause probable.
pub fn explain_not_writable(dir: &Path, d: &Diagnosis) -> String {
    let dir_shown = dir.display();
    let (uid, gid) = (d.uid, d.gid);
    // Les droits Unix ne l'expliquent pas (ou root a été refusé) : c'est un
    // contrôle d'accès obligatoire, ou le stockage lui-même.
    let mandatory = d.startup.denied_as_root || d.mode_allows() != Some(false);
    if mandatory && d.selinux {
        return format!(
            "{dir_shown} cannot be written: the host enforces SELinux and the volume has no \
             container label, so access is refused even to root. Add `:Z` to the volume \
             (`./dumbmonit:/data:Z`; `:z` if other containers share the directory), then start \
             the container again."
        );
    }
    if mandatory {
        return format!(
            "{dir_shown} cannot be written by uid {uid} although its permissions do not explain \
             it: a security module (SELinux: add `:Z` to the volume; AppArmor), a read-only mount \
             or network storage that squashes root (NFS root_squash) refuses access."
        );
    }

    let mut text = format!("{dir_shown} is not writable by the server (uid {uid}, gid {gid})");
    if let Some((owner, group, _)) = d.owner {
        text.push_str(&format!("; it belongs to {owner}:{group}"));
    }
    text.push('.');
    if d.startup.started_as_root {
        text.push_str(" Changing its owner at startup failed (see the warnings above). Or:");
    } else {
        text.push_str(&format!(
            "\nEasiest: start the container as root, with `--user 0:0` (`user: \"0:0\"` in \
             docker-compose.yml, plus `cap_add: [{REQUIRED_CAPS}]` if it drops all \
             capabilities). DumbMonit then gives {dir_shown} to 65532 (or PUID/PGID) and drops \
             root for good before doing anything else.\nOr:"
        ));
    }
    match &d.mount {
        MountKind::NamedVolume(name) => text.push_str(&format!(
            "\n  docker run --rm -v {name}:/data alpine chown -R {uid}:{gid} /data\n\
             to hand the Docker volume `{name}` over once."
        )),
        MountKind::Bind(source) => {
            text.push_str(&format!(
                "\n  sudo chown -R {uid}:{gid} {source}\n\
                 on the host, for this bind mount (if the path differs on the host, use the \
                 left-hand side of your `-v …:{dir_shown}`)."
            ));
            if let Some((owner, group, _)) = d.owner
                && owner != 0
            {
                text.push_str(&if d.startup.started_as_root {
                    format!(
                        " Or set PUID={owner} and PGID={group} to run as the owner of the files."
                    )
                } else {
                    format!(" Or run as the owner of the files: `--user {owner}:{group}`.")
                });
            }
            text.push_str(" On a SELinux host (Fedora, RHEL), also add `:Z` to the volume.");
        }
        MountKind::NotAMount => text.push_str(&format!(
            " {dir_shown} is not a volume, so its content would be lost with the container: \
             mount one (`-v dumbmonit-data:{dir_shown}`)."
        )),
    }
    text.trim_end().to_string()
}

/// Rassemble le diagnostic sur le système réel.
pub fn diagnose(dir: &Path, startup: Startup) -> Diagnosis {
    use std::os::unix::fs::MetadataExt;
    // SAFETY: appels sans argument.
    let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
    let owner = std::fs::metadata(dir).ok().map(|m| (m.uid(), m.gid(), m.mode() & 0o7777));
    let mount = std::fs::read_to_string("/proc/self/mountinfo")
        .map(|info| classify_mount(&info, dir))
        .unwrap_or(MountKind::NotAMount);
    let selinux = std::fs::read_to_string("/proc/self/attr/current")
        .is_ok_and(|current| is_selinux_context(&current));
    Diagnosis { uid, gid, owner, mount, selinux, startup }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{MetadataExt, symlink};

    fn vars(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let pairs: Vec<(String, String)> =
            pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |name| pairs.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
    }

    #[test]
    fn identifiants_par_defaut_et_surcharges() {
        assert_eq!(target_from(vars(&[])).unwrap(), Target { uid: 65532, gid: 65532 });
        assert_eq!(
            target_from(vars(&[("PUID", "1000"), ("PGID", " 100 ")])).unwrap(),
            Target { uid: 1000, gid: 100 }
        );
        // PUID seul : le gid reste celui par défaut.
        assert_eq!(target_from(vars(&[("PUID", "99")])).unwrap(), Target { uid: 99, gid: 65532 });
        // Le nom préfixé l'emporte ; une valeur vide compte comme absente.
        assert_eq!(
            target_from(vars(&[("PUID", "1000"), ("DUMBMONIT_PUID", "1001"), ("PGID", "")]))
                .unwrap(),
            Target { uid: 1001, gid: 65532 }
        );
    }

    #[test]
    fn identifiants_refuses() {
        for bad in ["0", "00", "-1", "abc", "1000:1000", "4294967295", "4294967296", "1e3", "+5"] {
            assert!(parse_id("PUID", bad).is_err(), "{bad} devrait être refusé");
        }
        assert!(target_from(vars(&[("PGID", "0")])).is_err());
        assert_eq!(parse_id("PUID", "4294967294").unwrap(), 4294967294);
    }

    #[test]
    fn decision_de_changement() {
        let target = Target { uid: 1000, gid: 1000 };
        assert!(!needs_change(1000, 1000, target));
        assert!(needs_change(0, 1000, target));
        assert!(needs_change(1000, 0, target));
    }

    #[test]
    fn repertoires_systeme_refuses() {
        assert!(safe_data_dir(Path::new("/")).is_err());
        assert!(safe_data_dir(Path::new("/etc/")).is_err());
        assert!(safe_data_dir(Path::new("data")).is_err());
        assert!(safe_data_dir(Path::new("/data")).is_ok());
        assert!(safe_data_dir(Path::new("/var/lib/dumbmonit")).is_ok());
    }

    /// Sans être root, on ne peut « changer » que vers sa propre identité : le
    /// parcours doit alors tout examiner sans rien modifier, et ne jamais suivre
    /// un lien — ni vers un fichier, ni vers un répertoire hors de l'arbre.
    #[test]
    fn parcours_sans_suivre_les_liens() {
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), b"x").unwrap();
        std::fs::create_dir(outside.path().join("sub")).unwrap();
        std::fs::write(outside.path().join("sub/deep"), b"x").unwrap();

        let data = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(data.path().join("vm/parts")).unwrap();
        std::fs::write(data.path().join("dumbmonit.db"), b"").unwrap();
        std::fs::write(data.path().join("vm/parts/a"), b"").unwrap();
        symlink(outside.path(), data.path().join("evil-dir")).unwrap();
        symlink(outside.path().join("secret"), data.path().join("evil-file")).unwrap();
        symlink("/nonexistent", data.path().join("dangling")).unwrap();

        let me = data.path().metadata().unwrap();
        let target = Target { uid: me.uid(), gid: me.gid() };
        let report = fix_ownership(data.path(), target);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(report.changed, 0);
        // racine, vm, vm/parts, dumbmonit.db, vm/parts/a, trois liens
        assert_eq!(report.checked, 8);
    }

    /// Une cible différente de soi : chaque entrée qui ne lui appartient pas est
    /// tentée (et refusée sans root), les liens comptés pour eux-mêmes et le
    /// contenu de leur cible jamais visité.
    #[test]
    fn parcours_vers_une_autre_identite() {
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("one"), b"x").unwrap();
        std::fs::write(outside.path().join("two"), b"x").unwrap();
        let data = tempfile::tempdir().unwrap();
        std::fs::create_dir(data.path().join("vm")).unwrap();
        std::fs::write(data.path().join("vm/a"), b"").unwrap();
        symlink(outside.path(), data.path().join("link")).unwrap();

        let before = outside.path().join("one").symlink_metadata().unwrap();
        let target = Target { uid: 4242, gid: 4242 };
        let report = fix_ownership(data.path(), target);
        // racine, vm, vm/a, link : quatre entrées, rien d'autre.
        assert_eq!(report.checked, 4);
        assert!(report.errors.iter().all(|e| !e.contains(outside.path().to_str().unwrap())));
        if unsafe { libc::geteuid() } == 0 {
            // En root (conteneur de test), le changement a réellement lieu…
            assert_eq!(report.changed, 4, "{:?}", report.errors);
            let link = data.path().join("link").symlink_metadata().unwrap();
            assert_eq!((link.uid(), link.gid()), (4242, 4242));
            assert_eq!(data.path().join("vm/a").metadata().unwrap().uid(), 4242);
            // … et un second passage n'a plus rien à faire.
            assert_eq!(fix_ownership(data.path(), target).changed, 0);
        } else {
            assert_eq!(report.changed, 0);
            assert_eq!(report.error_count, 4);
            assert!(report.access_denied);
        }
        // La cible du lien n'a pas bougé.
        let after = outside.path().join("one").symlink_metadata().unwrap();
        assert_eq!((before.uid(), before.gid()), (after.uid(), after.gid()));
        assert_eq!(outside.path().metadata().unwrap().uid(), before.uid());
    }

    #[test]
    fn racine_lien_symbolique_refusee() {
        let real = tempfile::tempdir().unwrap();
        let holder = tempfile::tempdir().unwrap();
        let link = holder.path().join("data");
        symlink(real.path(), &link).unwrap();
        let me = real.path().metadata().unwrap();
        let report = fix_ownership(&link, Target { uid: me.uid(), gid: me.gid() });
        assert_eq!(report.checked, 0);
        assert_eq!(report.error_count, 1);
    }

    #[test]
    fn capacites_effectives() {
        let status = "Name:\tdumbmonit\nCapInh:\t0000000000000000\nCapEff:\t00000000a80425fb\n";
        let caps = effective_caps(status).unwrap();
        assert!(has_cap(caps, CAP_CHOWN) && has_cap(caps, CAP_SETUID) && has_cap(caps, CAP_SETGID));
        assert!(has_cap(caps, CAP_DAC_OVERRIDE) && !has_cap(caps, CAP_DAC_READ_SEARCH));
        assert_eq!(effective_caps("CapEff:\t0000000000000000\n"), Some(0));
        assert_eq!(effective_caps("Name: x\n"), None);
    }

    #[test]
    fn nature_du_montage() {
        let info = "\
1 0 0:1 / / rw - overlay overlay rw
2 1 8:1 /var/lib/docker/volumes/dumbmonit-data/_data /data rw - ext4 /dev/sda1 rw
3 1 8:2 /noe/dumbmonit /srv rw - ext4 /dev/sda2 rw
4 1 0:30 /@/home/u/.local/share/containers/storage/volumes/pm/_data /pm rw - btrfs /dev/x rw
5 1 8:2 /mon\\040dossier /esp rw - ext4 /dev/sda2 rw
";
        assert_eq!(
            classify_mount(info, Path::new("/data")),
            MountKind::NamedVolume("dumbmonit-data".into())
        );
        assert_eq!(
            classify_mount(info, Path::new("/srv")),
            MountKind::Bind("/noe/dumbmonit".into())
        );
        assert_eq!(classify_mount(info, Path::new("/pm")), MountKind::NamedVolume("pm".into()));
        assert_eq!(classify_mount(info, Path::new("/esp")), MountKind::Bind("/mon dossier".into()));
        assert_eq!(classify_mount(info, Path::new("/nope")), MountKind::NotAMount);
    }

    #[test]
    fn contexte_selinux() {
        assert!(is_selinux_context("system_u:system_r:container_t:s0:c12,c34\0"));
        assert!(!is_selinux_context("docker-default (enforce)\n"));
        assert!(!is_selinux_context("unconfined"));
        assert!(!is_selinux_context(""));
    }

    fn diagnosis(owner: Option<(u32, u32, u32)>, mount: MountKind) -> Diagnosis {
        Diagnosis {
            uid: 65532,
            gid: 65532,
            owner,
            mount,
            selinux: false,
            startup: Startup::default(),
        }
    }

    #[test]
    fn messages_selon_la_cause() {
        let dir = Path::new("/data");
        let bind = MountKind::Bind("/home/noe/dumbmonit".into());

        // Bind mount à root, image en 65532 : d'abord `--user 0:0`, puis chown sur l'hôte.
        let text = explain_not_writable(dir, &diagnosis(Some((0, 0, 0o755)), bind.clone()));
        assert!(text.contains("--user 0:0"), "{text}");
        assert!(text.find("--user 0:0") < text.find("sudo chown"), "{text}");
        assert!(text.contains("sudo chown -R 65532:65532 /home/noe/dumbmonit"), "{text}");
        assert!(text.contains(":Z"), "{text}");
        assert!(!text.contains("docker run --rm -v"), "{text}");

        // Bind mount à 1000 : proposer aussi de tourner en 1000.
        let text = explain_not_writable(dir, &diagnosis(Some((1000, 1000, 0o755)), bind.clone()));
        assert!(text.contains("--user 1000:1000"), "{text}");

        // Démarré en root, correction échouée : pas de `--user 0:0`, PUID proposé.
        let mut d = diagnosis(Some((1000, 1000, 0o755)), bind.clone());
        d.startup = Startup { started_as_root: true, denied_as_root: false };
        let text = explain_not_writable(dir, &d);
        assert!(!text.contains("--user 0:0") && text.contains("PUID=1000"), "{text}");

        // Volume nommé : la commande vise le volume par son nom.
        let named = MountKind::NamedVolume("dumbmonit-data".into());
        let text = explain_not_writable(dir, &diagnosis(Some((0, 0, 0o755)), named));
        assert!(text.contains("-v dumbmonit-data:/data alpine chown -R 65532:65532"), "{text}");
        assert!(!text.contains("sudo"), "{text}");

        // Droits suffisants mais refus, hôte SELinux : `:Z`.
        let mut d = diagnosis(Some((65532, 65532, 0o755)), bind.clone());
        d.selinux = true;
        assert!(explain_not_writable(dir, &d).contains(":Z"));

        // Root refusé au démarrage malgré ses droits, sur un dossier root:root.
        let mut d = diagnosis(Some((0, 0, 0o755)), bind);
        d.selinux = true;
        d.startup = Startup { started_as_root: true, denied_as_root: true };
        assert!(explain_not_writable(dir, &d).contains("SELinux"));
    }
}
