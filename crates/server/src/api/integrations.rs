//! Fichiers du bandeau d'état pour sites tiers (`integrations/`), proposés en
//! téléchargement dans les réglages d'une page de statut.
//!
//! * `GET /integrations/wordpress.zip` — le plugin WordPress, installable par
//!   Extensions → Ajouter → Téléverser (un dossier racine au nom du plugin).
//! * `GET /integrations/banner.php` — l'aide PHP à inclure dans un site.
//!
//! Les fichiers sont embarqués dans le binaire. Le zip est écrit à la main, sans
//! compression (« stored ») : quelques dizaines de kio, aucune dépendance de plus.
//! Rien de secret ici : ce sont les mêmes fichiers que dans le dépôt.

use axum::Router;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::get;

use crate::state::AppState;

/// Dossier racine du plugin dans l'archive : WordPress en tire le nom du plugin.
const PLUGIN_DIR: &str = "dumbmonit-status-banner";

const PLUGIN_FILES: &[(&str, &[u8])] = &[
    (
        "dumbmonit-status-banner.php",
        include_bytes!(
            "../../../../integrations/wordpress/dumbmonit-status-banner/dumbmonit-status-banner.php"
        ),
    ),
    (
        "uninstall.php",
        include_bytes!("../../../../integrations/wordpress/dumbmonit-status-banner/uninstall.php"),
    ),
    (
        "readme.txt",
        include_bytes!("../../../../integrations/wordpress/dumbmonit-status-banner/readme.txt"),
    ),
    (
        "languages/dumbmonit-status-banner.pot",
        include_bytes!(
            "../../../../integrations/wordpress/dumbmonit-status-banner/languages/dumbmonit-status-banner.pot"
        ),
    ),
];

const BANNER_PHP: &str = include_str!("../../../../integrations/php/banner.php");

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/integrations/wordpress.zip", get(wordpress_zip))
        .route("/integrations/banner.php", get(banner_php))
}

/// Le plugin, rangé sous son dossier racine.
fn plugin_zip() -> Vec<u8> {
    let files: Vec<(String, &[u8])> =
        PLUGIN_FILES.iter().map(|(name, body)| (format!("{PLUGIN_DIR}/{name}"), *body)).collect();
    stored_zip(&files)
}

async fn wordpress_zip() -> Response {
    attachment("application/zip", "dumbmonit-status-banner.zip", plugin_zip())
}

async fn banner_php() -> Response {
    attachment("text/x-php; charset=utf-8", "banner.php", BANNER_PHP.as_bytes().to_vec())
}

fn attachment(content_type: &'static str, name: &str, body: Vec<u8>) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{name}\"")),
            (header::CACHE_CONTROL, "no-cache".to_string()),
        ],
        body,
    )
        .into_response()
}

/// CRC-32 (IEEE), tel que l'exige le format zip.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// Archive zip sans compression. Date fixe (1er janvier 1980) : deux appels
/// donnent exactement les mêmes octets.
fn stored_zip(files: &[(String, &[u8])]) -> Vec<u8> {
    const DOS_DATE: u16 = 0x0021;
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, body) in files {
        let offset = out.len() as u32;
        let crc = crc32(body);
        let size = body.len() as u32;
        let name_len = name.len() as u16;
        // En-tête local.
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes()); // version nécessaire
        out.extend_from_slice(&0x0800u16.to_le_bytes()); // noms en UTF-8
        out.extend_from_slice(&0u16.to_le_bytes()); // méthode : stored
        out.extend_from_slice(&0u16.to_le_bytes()); // heure
        out.extend_from_slice(&DOS_DATE.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(body);
        // Entrée du répertoire central.
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&0x031Eu16.to_le_bytes()); // créé sous Unix, v3.0
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0x0800u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&DOS_DATE.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&name_len.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // extra
        central.extend_from_slice(&0u16.to_le_bytes()); // commentaire
        central.extend_from_slice(&0u16.to_le_bytes()); // disque
        central.extend_from_slice(&0u16.to_le_bytes()); // attributs internes
        central.extend_from_slice(&(0o100_644u32 << 16).to_le_bytes()); // fichier 0644
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let central_offset = out.len() as u32;
    let count = files.len() as u16;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&central_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u16_at(b: &[u8], i: usize) -> usize {
        u16::from_le_bytes([b[i], b[i + 1]]) as usize
    }
    fn u32_at(b: &[u8], i: usize) -> usize {
        u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as usize
    }

    /// Relit l'archive par son répertoire central, comme le ferait un
    /// décompresseur : nom, contenu, CRC.
    fn read_zip(zip: &[u8]) -> Vec<(String, Vec<u8>)> {
        let eocd = zip.len() - 22;
        assert_eq!(u32_at(zip, eocd), 0x0605_4b50, "fin de répertoire central");
        let count = u16_at(zip, eocd + 10);
        let mut pos = u32_at(zip, eocd + 16);
        let mut out = Vec::new();
        for _ in 0..count {
            assert_eq!(u32_at(zip, pos), 0x0201_4b50);
            let crc = u32_at(zip, pos + 16);
            let size = u32_at(zip, pos + 24);
            let name_len = u16_at(zip, pos + 28);
            let local = u32_at(zip, pos + 42);
            let name = String::from_utf8(zip[pos + 46..pos + 46 + name_len].to_vec()).unwrap();
            assert_eq!(u32_at(zip, local), 0x0403_4b50);
            let start = local + 30 + u16_at(zip, local + 26) + u16_at(zip, local + 28);
            let body = zip[start..start + size].to_vec();
            assert_eq!(crc32(&body) as usize, crc, "CRC de {name}");
            out.push((name, body));
            pos += 46 + name_len;
        }
        out
    }

    #[test]
    fn crc32_valeur_connue() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn le_zip_contient_le_plugin_sous_son_dossier() {
        let entries = read_zip(&plugin_zip());
        assert_eq!(entries.len(), PLUGIN_FILES.len());
        let main = entries
            .iter()
            .find(|(name, _)| name == "dumbmonit-status-banner/dumbmonit-status-banner.php")
            .expect("fichier principal du plugin");
        assert!(String::from_utf8_lossy(&main.1).contains("Plugin Name:"));
        assert!(entries.iter().all(|(name, _)| name.starts_with("dumbmonit-status-banner/")));
    }
}
