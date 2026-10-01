//! Le lien que l'on envoie aux murs (« Play on the wall »).
//!
//! L'interface le transforme en lecteur intégré (`web/src/lib/wall/music.ts`)
//! et le revalide à chaque lecture : ce contrôle-ci n'est qu'un premier tri,
//! pour qu'un lien d'ailleurs ne soit jamais enregistré ni montré aux autres
//! murs. Même liste d'hôtes que l'interface ; garder les deux en phase.

/// Longueur maximale acceptée : un lien de partage tient en 200 caractères.
pub const MAX_LEN: usize = 500;

const HOSTS: &[&str] = &[
    "open.spotify.com",
    "play.spotify.com",
    "deezer.com",
    "www.deezer.com",
    "widget.deezer.com",
    "youtu.be",
    "youtube.com",
    "www.youtube.com",
    "m.youtube.com",
    "music.youtube.com",
    "youtube-nocookie.com",
    "www.youtube-nocookie.com",
];

pub const REFUSED: &str =
    "Paste a Spotify, Deezer or YouTube link: a track, album, playlist, episode or video.";

/// Rend le lien nettoyé, ou la phrase à montrer.
pub fn validate(input: &str) -> Result<String, &'static str> {
    let link = input.trim();
    if link.is_empty() {
        return Err("Paste a link first.");
    }
    if link.len() > MAX_LEN || link.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(REFUSED);
    }
    // L'application Spotify copie aussi des URI : spotify:playlist:<id>.
    if let Some(rest) = link.strip_prefix("spotify:") {
        let mut parts = rest.split(':');
        let ok = matches!(
            (parts.next(), parts.next(), parts.next()),
            (Some(kind), Some(id), None)
                if !kind.is_empty() && kind.bytes().all(|b| b.is_ascii_lowercase())
                    && !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric())
        );
        return if ok { Ok(link.to_string()) } else { Err(REFUSED) };
    }
    let has_scheme = link.split_once("://").is_some_and(|(scheme, _)| {
        !scheme.is_empty() && scheme.bytes().all(|b| b.is_ascii_alphabetic())
    });
    let absolute = if has_scheme { link.to_string() } else { format!("https://{link}") };
    let url = reqwest::Url::parse(&absolute).map_err(|_| REFUSED)?;
    if !matches!(url.scheme(), "https" | "http")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|port| port != 443 && port != 80)
    {
        return Err(REFUSED);
    }
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if !HOSTS.contains(&host.as_str()) {
        return Err(REFUSED);
    }
    Ok(link.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_links_of_the_three_providers_are_kept_as_pasted() {
        for link in [
            "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M?si=abc",
            "open.spotify.com/track/37i9dQZF1DXcBWIGoYBM5M",
            "spotify:album:37i9dQZF1DXcBWIGoYBM5M",
            "https://www.deezer.com/fr/playlist/1313621735",
            "https://youtu.be/dQw4w9WgXcQ",
            "https://music.youtube.com/watch?v=dQw4w9WgXcQ&list=PL1",
        ] {
            assert_eq!(validate(&format!("  {link} ")).as_deref(), Ok(link), "{link}");
        }
    }

    #[test]
    fn anything_else_is_refused() {
        for link in [
            "",
            "javascript:alert(1)",
            "data:text/html,x",
            "https://open.spotify.com.evil.test/track/x",
            "https://evil.test/open.spotify.com/track/x",
            "https://user:pw@open.spotify.com/track/x",
            "https://open.spotify.com:8443/track/x",
            "ftp://open.spotify.com/track/x",
            "spotify:track:abc:def",
            "spotify:track:<script>",
            "https://open.spotify.com/track/a b",
        ] {
            assert!(validate(link).is_err(), "{link}");
        }
        assert!(validate(&format!("https://youtu.be/{}", "a".repeat(MAX_LEN))).is_err());
    }
}
