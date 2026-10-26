//! Relevé d'une configuration par SSH.
//!
//! # Clé d'hôte
//!
//! Confiance au premier usage : la première connexion accepte la clé
//! présentée et en rend l'empreinte, que le serveur mémorise. Ensuite, une
//! clé différente fait couper la connexion **avant toute authentification** —
//! aucun mot de passe ne part vers un équipement qui pourrait être un
//! imposteur — et l'erreur [`Failure::HostKeyMismatch`] fait lever une alerte.
//! Seul un administrateur peut accepter la nouvelle clé, dans l'interface.
//!
//! # Commandes
//!
//! Seules celles du profil sont envoyées, telles quelles : aucun champ saisi
//! par l'utilisateur n'y est interpolé. Le seul texte d'origine utilisateur
//! qui transite par le terminal est le mot de passe `enable`, envoyé en
//! réponse à sa demande ; un saut de ligne ou un caractère de contrôle y est
//! refusé, faute de quoi il pourrait enchaîner une commande.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use dumbmonit_proto::ProbeError;
use regex::Regex;
use russh::client::{self, KeyboardInteractiveAuthResponse};
use russh::keys::{HashAlg, PublicKeyOrCertificate};
use russh::{Channel, ChannelMsg};

use super::profile::{Profile, Reply, Transport};
use super::text;
use super::{Failure, append_bounded};

/// Identifiants d'une session SSH.
pub struct Login<'a> {
    pub username: &'a str,
    pub password: &'a str,
    /// Mot de passe `enable` ; vide : celui du compte.
    pub enable_secret: &'a str,
}

/// Ce qu'un relevé SSH réussi rend.
pub struct Fetched {
    pub config: String,
    pub host_key: String,
}

/// Vérification de la clé d'hôte, et empreinte vue pour l'appelant.
struct HostKeyCheck {
    pinned: Option<String>,
    seen: Arc<Mutex<Option<String>>>,
}

impl client::Handler for HostKeyCheck {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let fingerprint = fingerprint(server_public_key);
        let accepted = accepts_host_key(self.pinned.as_deref(), &fingerprint);
        if let Ok(mut seen) = self.seen.lock() {
            *seen = Some(fingerprint);
        }
        Ok(accepted)
    }
}

/// Jamais d'acceptation silencieuse d'une clé différente : sans empreinte
/// mémorisée, c'est le premier usage, que l'appelant enregistre ; avec, elle
/// doit être identique, octet pour octet.
pub fn accepts_host_key(pinned: Option<&str>, seen: &str) -> bool {
    match pinned {
        None => true,
        Some(pinned) => pinned == seen,
    }
}

/// Empreinte SHA-256 de la clé (ou de la clé d'un certificat), au format
/// d'OpenSSH : `SHA256:…`.
fn fingerprint(key: &PublicKeyOrCertificate) -> String {
    let data = match key {
        PublicKeyOrCertificate::PublicKey { key, .. } => key.key_data(),
        PublicKeyOrCertificate::Certificate(certificate) => certificate.public_key(),
    };
    data.fingerprint(HashAlg::Sha256).to_string()
}

/// Un texte d'origine utilisateur envoyé dans le terminal doit tenir sur une
/// ligne, sans caractère de contrôle.
pub fn check_terminal_safe(value: &str, what: &str) -> Result<(), ProbeError> {
    if value.chars().any(char::is_control) {
        return Err(ProbeError::Config(format!(
            "the {what} contains a line break or a control character, which is refused"
        )));
    }
    Ok(())
}

fn ssh_error(error: russh::Error) -> ProbeError {
    match error {
        russh::Error::IO(error) => ProbeError::Unreachable(error.to_string()),
        russh::Error::ConnectionTimeout | russh::Error::InactivityTimeout => {
            ProbeError::Unreachable(error.to_string())
        }
        other => ProbeError::Protocol(format!("SSH: {other}")),
    }
}

/// Relève la configuration de `host:port` selon `profile`.
pub async fn fetch(
    host: &str,
    port: u16,
    login: &Login<'_>,
    profile: &Profile,
    pinned: Option<&str>,
    step: Duration,
) -> Result<Fetched, Failure> {
    check_terminal_safe(login.password, "password")?;
    check_terminal_safe(login.enable_secret, "enable secret")?;

    let seen = Arc::new(Mutex::new(None));
    let handler = HostKeyCheck { pinned: pinned.map(str::to_owned), seen: seen.clone() };
    let config =
        Arc::new(client::Config { inactivity_timeout: Some(step), ..client::Config::default() });
    let connected =
        tokio::time::timeout(step, client::connect(config, (host, port), handler)).await;
    let seen_key = seen.lock().ok().and_then(|seen| seen.clone());
    let mut handle = match connected {
        Err(_) => return Err(ProbeError::Timeout(step).into()),
        Ok(Err(error)) => {
            if let (Some(pinned), Some(seen)) = (pinned, seen_key.as_deref())
                && pinned != seen
            {
                return Err(Failure::HostKeyMismatch { seen: seen.to_owned() });
            }
            return Err(ssh_error(error).into());
        }
        Ok(Ok(handle)) => handle,
    };
    let host_key =
        seen_key.ok_or_else(|| ProbeError::Protocol("the device presented no host key".into()))?;

    authenticate(&mut handle, login, step).await?;

    let config = match profile.transport {
        Transport::SshExec => {
            let mut total = 0usize;
            let mut out = String::new();
            for command in &profile.commands {
                out.push_str(&exec(&handle, command, step, &mut total).await?);
            }
            out
        }
        Transport::SshShell => shell(&handle, login, profile, step).await?,
        Transport::Http => {
            return Err(ProbeError::Config("this profile does not use SSH".into()).into());
        }
    };
    let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "en").await;
    Ok(Fetched { config, host_key })
}

/// Mot de passe, puis clavier interactif (que certains équipements sont seuls
/// à proposer) avec le même mot de passe.
async fn authenticate(
    handle: &mut client::Handle<HostKeyCheck>,
    login: &Login<'_>,
    step: Duration,
) -> Result<(), ProbeError> {
    let refused = || ProbeError::Auth("the device refused the user name or password".into());
    let password =
        tokio::time::timeout(step, handle.authenticate_password(login.username, login.password))
            .await
            .map_err(|_| ProbeError::Protocol("no answer to the authentication".into()))?
            .map_err(ssh_error)?;
    if password.success() {
        return Ok(());
    }

    let mut answer = tokio::time::timeout(
        step,
        handle.authenticate_keyboard_interactive_start(login.username, None::<String>),
    )
    .await
    .map_err(|_| ProbeError::Protocol("no answer to the authentication".into()))?
    .map_err(ssh_error)?;
    // Quelques tours au plus : un serveur qui redemande sans fin n'acceptera
    // jamais ce mot de passe.
    for _ in 0..3 {
        match answer {
            KeyboardInteractiveAuthResponse::Success => return Ok(()),
            KeyboardInteractiveAuthResponse::Failure { .. } => return Err(refused()),
            KeyboardInteractiveAuthResponse::InfoRequest { prompts, .. } => {
                let responses = prompts.iter().map(|_| login.password.to_owned()).collect();
                answer = tokio::time::timeout(
                    step,
                    handle.authenticate_keyboard_interactive_respond(responses),
                )
                .await
                .map_err(|_| ProbeError::Protocol("no answer to the authentication".into()))?
                .map_err(ssh_error)?;
            }
        }
    }
    Err(refused())
}

/// Une commande sans terminal ; rend sa sortie standard nettoyée.
async fn exec(
    handle: &client::Handle<HostKeyCheck>,
    command: &str,
    step: Duration,
    total: &mut usize,
) -> Result<String, ProbeError> {
    let mut channel = handle.channel_open_session().await.map_err(ssh_error)?;
    channel.exec(true, command).await.map_err(ssh_error)?;
    let mut out: Vec<u8> = Vec::new();
    let mut errors: Vec<u8> = Vec::new();
    let mut status = None;
    let mut eof = false;
    loop {
        let message = match tokio::time::timeout(step, channel.wait()).await {
            Ok(Some(message)) => message,
            Ok(None) => break,
            // Fin de flux reçue mais canal jamais fermé : la sortie est complète.
            Err(_) if eof => break,
            Err(_) => {
                return Err(ProbeError::Protocol(format!(
                    "the command sent nothing for {}s",
                    step.as_secs()
                )));
            }
        };
        match message {
            ChannelMsg::Data { data } => append_bounded(&mut out, total, &data)?,
            ChannelMsg::ExtendedData { data, .. } => {
                if errors.len() < 4096 {
                    errors.extend_from_slice(&data);
                }
            }
            ChannelMsg::ExitStatus { exit_status } => status = Some(exit_status),
            ChannelMsg::Eof => eof = true,
            ChannelMsg::Close => break,
            _ => {}
        }
    }
    let _ = channel.close().await;
    if let Some(code) = status.filter(|code| *code != 0)
        && out.iter().all(u8::is_ascii_whitespace)
    {
        let stderr = String::from_utf8_lossy(&errors);
        let first = stderr.lines().next().unwrap_or("").chars().take(200).collect::<String>();
        return Err(ProbeError::Protocol(format!(
            "the command failed (exit status {code}): {first}"
        )));
    }
    Ok(text::clean_terminal(&String::from_utf8_lossy(&out)))
}

/// Un terminal ouvert sur l'équipement.
struct Terminal {
    channel: Channel<client::Msg>,
    buf: Vec<u8>,
    total: usize,
    step: Duration,
    /// Dernière ligne qui a clos une attente : l'invite courante.
    last_prompt: String,
}

/// Octets de fin de tampon examinés pour reconnaître une invite.
const TAIL: usize = 2048;
/// Au-delà de ce qui a été reçu, une erreur ne cite plus la dernière ligne.
const TAIL_SHOWN_UNTIL: usize = 1024;

impl Terminal {
    async fn send(&self, text: &str) -> Result<(), ProbeError> {
        self.channel.data(text.as_bytes()).await.map_err(ssh_error)
    }

    /// Dernière ligne reçue, nettoyée.
    fn tail(&self) -> String {
        let start = self.buf.len().saturating_sub(TAIL);
        let cleaned = text::clean_terminal(&String::from_utf8_lossy(&self.buf[start..]));
        text::last_line(&cleaned).to_owned()
    }

    /// Lit jusqu'à ce que la dernière ligne corresponde à l'une de `wanted` ;
    /// rend son rang et tout ce qui a été reçu depuis l'appel précédent. Les
    /// `replies` (pagination, « appuyez sur une touche ») sont répondues au
    /// passage, une fois par arrivée de données.
    async fn expect(
        &mut self,
        wanted: &[&Regex],
        replies: &[(Regex, String)],
    ) -> Result<(usize, String), ProbeError> {
        let mut answered = 0usize;
        loop {
            let tail = self.tail();
            if let Some(index) = wanted.iter().position(|re| re.is_match(&tail)) {
                self.last_prompt = tail;
                let raw = std::mem::take(&mut self.buf);
                return Ok((index, String::from_utf8_lossy(&raw).into_owned()));
            }
            if self.buf.len() > answered
                && let Some((_, send)) = replies.iter().find(|(re, _)| re.is_match(&tail))
            {
                answered = self.buf.len();
                self.send(send).await?;
            }
            let message = match tokio::time::timeout(self.step, self.channel.wait()).await {
                Ok(Some(message)) => message,
                Ok(None) => {
                    return Err(ProbeError::Protocol("the device closed the session".into()));
                }
                Err(_) => {
                    // La dernière ligne aide à corriger une invite non reconnue
                    // à la connexion ; passé le début de session, ce serait un
                    // morceau de configuration, peut-être un secret, dans un
                    // message que tous les rôles lisent : elle est tue.
                    let seconds = self.step.as_secs();
                    return Err(ProbeError::Protocol(if self.total <= TAIL_SHOWN_UNTIL {
                        let shown: String = tail.chars().take(80).collect();
                        format!("no expected prompt within {seconds}s (last line: \"{shown}\")")
                    } else {
                        format!("no expected prompt within {seconds}s")
                    }));
                }
            };
            match message {
                ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                    append_bounded(&mut self.buf, &mut self.total, &data)?;
                }
                ChannelMsg::Eof | ChannelMsg::Close => {
                    return Err(ProbeError::Protocol("the device closed the session".into()));
                }
                _ => {}
            }
        }
    }
}

fn rule(pattern: &str) -> Result<Regex, ProbeError> {
    Regex::new(pattern).map_err(|e| ProbeError::Config(format!("profile expression: {e}")))
}

fn replies(list: &[Reply]) -> Result<Vec<(Regex, String)>, ProbeError> {
    list.iter().map(|reply| Ok((rule(&reply.expect)?, reply.send.clone()))).collect()
}

/// Session interactive : invite, `enable`, pagination, commandes.
async fn shell(
    handle: &client::Handle<HostKeyCheck>,
    login: &Login<'_>,
    profile: &Profile,
    step: Duration,
) -> Result<String, ProbeError> {
    let prompt = rule(profile.prompt.as_deref().unwrap_or(r"[>#$] ?$"))?;
    let login_replies = replies(&profile.login_replies)?;
    let more = replies(profile.more.as_slice())?;

    let channel = handle.channel_open_session().await.map_err(ssh_error)?;
    channel.request_pty(false, "vt100", 511, 0, 0, 0, &[]).await.map_err(ssh_error)?;
    channel.request_shell(false).await.map_err(ssh_error)?;
    let mut terminal =
        Terminal { channel, buf: Vec::new(), total: 0, step, last_prompt: String::new() };

    terminal.expect(&[&prompt], &login_replies).await?;
    let mut current = terminal.last_prompt.clone();

    if let Some(enable) = &profile.enable {
        let unprivileged = rule(&enable.unprivileged)?;
        if unprivileged.is_match(&current) {
            let password_prompt = rule(&enable.password_prompt)?;
            terminal.send(&format!("{}\n", enable.command)).await?;
            let (hit, _) = terminal.expect(&[&password_prompt, &prompt], &[]).await?;
            if hit == 0 {
                let secret = if login.enable_secret.is_empty() {
                    login.password
                } else {
                    login.enable_secret
                };
                terminal.send(&format!("{secret}\n")).await?;
                terminal.expect(&[&prompt], &[]).await?;
            }
            current = terminal.last_prompt.clone();
            if unprivileged.is_match(&current) {
                return Err(ProbeError::Auth("the device refused the enable password".into()));
            }
        }
    }

    // L'invite exacte de la session : la fin d'une commande se reconnaît à
    // elle, pas à une ligne de configuration qui finirait par `#` ou `>`.
    let exact = rule(&format!("^{}\\s*$", regex::escape(current.trim_end())))?;

    for command in &profile.setup {
        terminal.send(&format!("{command}\n")).await?;
        terminal.expect(&[&exact], &more).await?;
    }
    let more_rule = profile.more.as_ref().map(|more| rule(&more.expect)).transpose()?;
    let mut out = String::new();
    for command in &profile.commands {
        terminal.send(&format!("{command}\n")).await?;
        let (_, step_output) = terminal.expect(&[&exact], &more).await?;
        let mut output = text::command_output(&step_output, command);
        if let Some(rule) = &more_rule {
            output = rule.replace_all(&output, "").into_owned();
        }
        out.push_str(&output);
    }
    if let Some(exit) = &profile.exit {
        let _ = terminal.send(&format!("{exit}\n")).await;
    }
    let _ = terminal.channel.close().await;
    Ok(out)
}
