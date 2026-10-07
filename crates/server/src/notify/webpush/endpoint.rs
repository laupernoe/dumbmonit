//! Garde contre la falsification de requête côté serveur (SSRF).
//!
//! L'adresse d'un abonnement Web Push vient du navigateur, donc du client de
//! l'API : rien n'empêche un compte de déposer `https://127.0.0.1:8428/…` ou
//! `https://192.168.1.1/…` à la place de l'adresse de son service de push, puis
//! de demander un « envoi de test ». Les services de push (Google, Mozilla,
//! Apple, Microsoft) sont tous publics ; la règle est donc plus stricte que
//! celle des moniteurs ([`dumbmonit_collectors::uptime::guard`], qui laisse
//! passer les plages privées d'un homelab) : **HTTPS, et uniquement des adresses
//! publiques**.
//!
//! La vérification a lieu deux fois : à l'enregistrement de l'abonnement
//! ([`validate`], puis [`vet_resolution`]) et au moment de chaque connexion, par
//! le résolveur du client d'envoi ([`PublicOnlyResolver`]) — ce qui ferme la
//! porte au « DNS rebinding », un nom qui changerait de réponse entre les deux.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use dumbmonit_collectors::uptime::guard;
use reqwest::Url;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};

/// Longueur maximale d'une adresse d'abonnement. Les services connus restent
/// sous 500 caractères ; au-delà, ce n'est pas un abonnement.
pub const MAX_ENDPOINT_LEN: usize = 2048;

/// Suffixes de noms qui ne désignent que le réseau local : refusés d'emblée,
/// sans même les résoudre.
const LOCAL_SUFFIXES: &[&str] =
    &[".localhost", ".local", ".internal", ".lan", ".home.arpa", ".localdomain"];

/// Vrai si l'adresse IP est joignable sur l'internet public.
///
/// Refusées : tout ce que refuse déjà le garde des moniteurs (boucle locale,
/// lien local, métadonnées de cloud, adresse non spécifiée), plus les plages
/// privées (RFC 1918, ULA), l'espace partagé des opérateurs (100.64.0.0/10), la
/// multidiffusion, la diffusion, les plages de documentation, de test et
/// réservées.
pub fn is_public(ip: IpAddr) -> bool {
    if guard::is_forbidden(ip) {
        return false;
    }
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_public_v4(mapped);
            }
            is_public_v6(v6)
        }
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_documentation()
        || a == 0
        // Espace partagé des opérateurs (CGNAT).
        || (a == 100 && (64..128).contains(&b))
        // Affectations du protocole IETF.
        || (a == 192 && b == 0 && c == 0)
        // Bancs d'essai.
        || (a == 198 && (b == 18 || b == 19))
        // Réservé pour un usage futur.
        || a >= 240)
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    let first = ip.segments()[0];
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_unicast_link_local()
        // Adresses locales uniques (fc00::/7) et anciennes « site-local ».
        || (first & 0xfe00) == 0xfc00
        || (first & 0xffc0) == 0xfec0
        // Documentation (2001:db8::/32).
        || (first == 0x2001 && ip.segments()[1] == 0x0db8)
        // NAT64 bien connu : traduit vers n'importe quelle adresse IPv4.
        || (first == 0x0064 && ip.segments()[1] == 0xff9b))
}

/// Contrôle d'une adresse d'abonnement, sans réseau.
///
/// Le message d'erreur est rédigé pour l'utilisateur qui verrait échouer
/// « Enable on this device » : il dit ce qui est attendu.
pub fn validate(endpoint: &str) -> Result<Url, String> {
    if endpoint.len() > MAX_ENDPOINT_LEN {
        return Err("The push endpoint is too long to be a browser subscription.".into());
    }
    let url =
        Url::parse(endpoint).map_err(|_| "The push endpoint is not a valid URL.".to_string())?;
    if url.scheme() != "https" {
        return Err("The push endpoint must use HTTPS.".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("The push endpoint must not carry credentials.".into());
    }
    let Some(host) = url.host_str() else {
        return Err("The push endpoint has no host.".into());
    };
    let host = host.trim_matches(|c| c == '[' || c == ']');
    if let Ok(ip) = host.parse::<IpAddr>() {
        if !is_public(ip) {
            return Err(refusal(host, ip));
        }
    } else if !host.contains('.')
        || LOCAL_SUFFIXES.iter().any(|suffix| host.to_ascii_lowercase().ends_with(suffix))
    {
        return Err(format!(
            "\"{host}\" is not a public push service. Browsers subscribe through their \
             vendor's service (Google, Mozilla, Apple, Microsoft)."
        ));
    }
    Ok(url)
}

fn refusal(host: &str, ip: IpAddr) -> String {
    format!(
        "\"{host}\" resolves to {ip}, which is not a public address. Push services are \
         always on the internet; DumbMonit does not send push messages to private, \
         loopback or link-local addresses."
    )
}

/// Résout le nom de l'adresse et refuse si l'une de ses adresses n'est pas
/// publique. Une résolution qui échoue n'est pas un refus : l'envoi échouera
/// de lui-même, et le dira.
pub async fn vet_resolution(url: &Url) -> Result<(), String> {
    let Some(host) = url.host_str() else { return Ok(()) };
    let host = host.trim_matches(|c| c == '[' || c == ']');
    if host.parse::<IpAddr>().is_ok() {
        return Ok(()); // déjà jugée par `validate`
    }
    let port = url.port_or_known_default().unwrap_or(443);
    let lookup = tokio::net::lookup_host((host, port));
    match tokio::time::timeout(Duration::from_secs(5), lookup).await {
        Ok(Ok(addresses)) => vet(host, &addresses.collect::<Vec<_>>()),
        Ok(Err(_)) | Err(_) => Ok(()),
    }
}

/// Toutes les adresses comptent : un nom qui en mêle une publique et une
/// privée est justement le tour que ce garde doit déjouer.
pub fn vet(host: &str, addresses: &[SocketAddr]) -> Result<(), String> {
    match addresses.iter().map(SocketAddr::ip).find(|ip| !is_public(*ip)) {
        Some(ip) => Err(refusal(host, ip)),
        None => Ok(()),
    }
}

/// Résolveur DNS du client d'envoi : celui du système, suivi du garde.
pub struct PublicOnlyResolver;

impl Resolve for PublicOnlyResolver {
    fn resolve(&self, name: Name) -> Resolving {
        Box::pin(async move {
            let host = name.as_str().to_string();
            let addresses: Vec<SocketAddr> =
                tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
            vet(&host, &addresses)?;
            Ok(Box::new(addresses.into_iter()) as Addrs)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    #[test]
    fn les_adresses_non_publiques_sont_refusees() {
        for address in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "0.1.2.3",
            "255.255.255.255",
            "224.0.0.1",
            "192.0.2.1",
            "198.18.0.1",
            "240.0.0.1",
            "::1",
            "::",
            "fe80::1",
            "fd00::1",
            "fc00::5",
            "fec0::1",
            "ff02::1",
            "2001:db8::1",
            "64:ff9b::7f00:1",
            "::ffff:10.0.0.1",
            "::ffff:127.0.0.1",
        ] {
            assert!(!is_public(ip(address)), "{address} should be refused");
        }
    }

    #[test]
    fn les_adresses_publiques_passent() {
        for address in ["142.250.74.106", "8.8.8.8", "2a00:1450:4007:80e::200a", "17.253.144.10"] {
            assert!(is_public(ip(address)), "{address} should pass");
        }
    }

    #[test]
    fn seules_les_adresses_https_de_services_publics_sont_acceptees() {
        assert!(validate("https://fcm.googleapis.com/fcm/send/abc:def").is_ok());
        assert!(validate("https://updates.push.services.mozilla.com/wpush/v2/gAAA").is_ok());
        assert!(validate("https://web.push.apple.com/QGx").is_ok());
        assert!(validate("https://wns2-par02p.notify.windows.com/w/?token=x").is_ok());

        for (endpoint, why) in [
            ("http://fcm.googleapis.com/fcm/send/abc", "HTTPS"),
            ("ftp://fcm.googleapis.com/x", "HTTPS"),
            ("https://127.0.0.1:8428/api/v1/admin", "not a public address"),
            ("https://[::1]/x", "not a public address"),
            ("https://192.168.1.1/x", "not a public address"),
            ("https://169.254.169.254/latest/meta-data", "not a public address"),
            ("https://[::ffff:127.0.0.1]/x", "not a public address"),
            ("https://localhost/x", "not a public push service"),
            ("https://nas.local/x", "not a public push service"),
            ("https://router/x", "not a public push service"),
            ("https://user:pw@fcm.googleapis.com/x", "credentials"),
            ("not a url", "not a valid URL"),
        ] {
            let error = validate(endpoint).expect_err(endpoint);
            assert!(error.contains(why), "{endpoint}: {error}");
        }
        let long = format!("https://fcm.googleapis.com/{}", "a".repeat(MAX_ENDPOINT_LEN));
        assert!(validate(&long).is_err());
    }

    #[test]
    fn une_resolution_melangee_est_refusee() {
        let mixed: Vec<SocketAddr> =
            vec!["142.250.74.106:443".parse().unwrap(), "10.0.0.1:443".parse().unwrap()];
        assert!(vet("evil.example", &mixed).is_err());
        assert!(vet("fcm.googleapis.com", &mixed[..1]).is_ok());
    }
}
