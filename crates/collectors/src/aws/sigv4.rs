//! Signature AWS « Signature Version 4 », écrite à la main avec `ring`.
//!
//! Le SDK officiel tirerait des dizaines de dépendances pour signer une
//! requête POST à corps JSON : l'algorithme tient en une page. Il est vérifié
//! par le vecteur de test officiel `get-vanilla` de la suite de tests AWS.
//!
//! Étapes (documentation « Signature Version 4 signing process ») :
//!
//! 1. requête canonique : méthode, chemin, paramètres, en-têtes signés (noms en
//!    minuscules, triés), liste des en-têtes signés, empreinte SHA-256 du corps ;
//! 2. chaîne à signer : algorithme, date, portée, empreinte de la requête canonique ;
//! 3. clé de signature : HMAC en cascade (date, région, service, `aws4_request`) ;
//! 4. en-tête `Authorization`.

use std::fmt::Write;

use ring::{digest, hmac};

const ALGORITHM: &str = "AWS4-HMAC-SHA256";

/// Empreinte SHA-256 en hexadécimal minuscule.
pub fn sha256_hex(data: &[u8]) -> String {
    hex(digest::digest(&digest::SHA256, data).as_ref())
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let key = hmac::Key::new(hmac::HMAC_SHA256, key);
    hmac::sign(&key, data).as_ref().to_vec()
}

/// Les éléments fixes d'une signature : qui signe, pour quelle région et quel service.
pub struct Signer<'a> {
    pub access_key: &'a str,
    pub secret_key: &'a str,
    pub region: &'a str,
    pub service: &'a str,
}

impl Signer<'_> {
    /// Valeur de l'en-tête `Authorization`.
    ///
    /// `amz_date` au format `20150830T123600Z`. `headers` : tous les en-têtes à
    /// signer (`host` y compris, `authorization` exclu), dans n'importe quel ordre
    /// et casse ; `query` est la chaîne de paramètres déjà canonique (vide ici).
    pub fn authorization(
        &self,
        amz_date: &str,
        method: &str,
        path: &str,
        query: &str,
        headers: &[(String, String)],
        payload_hash: &str,
    ) -> String {
        let (canonical, signed) = canonical_request(method, path, query, headers, payload_hash);
        let scope = self.scope(amz_date);
        let to_sign = string_to_sign(amz_date, &scope, &canonical);
        let signature = hex(&hmac_sha256(&self.signing_key(amz_date), to_sign.as_bytes()));
        format!(
            "{ALGORITHM} Credential={}/{scope}, SignedHeaders={signed}, Signature={signature}",
            self.access_key
        )
    }

    fn scope(&self, amz_date: &str) -> String {
        let day = amz_date.get(..8).unwrap_or(amz_date);
        format!("{day}/{}/{}/aws4_request", self.region, self.service)
    }

    fn signing_key(&self, amz_date: &str) -> Vec<u8> {
        let day = amz_date.get(..8).unwrap_or(amz_date);
        let secret = format!("AWS4{}", self.secret_key);
        let k_date = hmac_sha256(secret.as_bytes(), day.as_bytes());
        let k_region = hmac_sha256(&k_date, self.region.as_bytes());
        let k_service = hmac_sha256(&k_region, self.service.as_bytes());
        hmac_sha256(&k_service, b"aws4_request")
    }
}

/// Requête canonique et liste des en-têtes signés (`host;x-amz-date`).
fn canonical_request(
    method: &str,
    path: &str,
    query: &str,
    headers: &[(String, String)],
    payload_hash: &str,
) -> (String, String) {
    let mut lines: Vec<(String, String)> = headers
        .iter()
        .map(|(name, value)| {
            // Valeur épurée : espaces de bord retirés, espaces répétés réduits à un.
            let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
            (name.trim().to_ascii_lowercase(), value)
        })
        .collect();
    lines.sort();
    let mut block = String::new();
    for (name, value) in &lines {
        let _ = writeln!(block, "{name}:{value}");
    }
    let signed = lines.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>().join(";");
    (format!("{method}\n{path}\n{query}\n{block}\n{signed}\n{payload_hash}"), signed)
}

fn string_to_sign(amz_date: &str, scope: &str, canonical_request: &str) -> String {
    format!("{ALGORITHM}\n{amz_date}\n{scope}\n{}", sha256_hex(canonical_request.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vecteur `get-vanilla` de la suite de tests officielle de Signature
    /// Version 4 : `GET /` sur `example.amazonaws.com`, service fictif
    /// `service`, région `us-east-1`, 30 août 2015.
    const DATE: &str = "20150830T123600Z";
    const SECRET: &str = "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY";

    fn vanilla_headers() -> Vec<(String, String)> {
        vec![
            ("X-Amz-Date".to_string(), DATE.to_string()),
            ("Host".to_string(), "example.amazonaws.com".to_string()),
        ]
    }

    #[test]
    fn requete_canonique_du_vecteur_officiel() {
        let empty = sha256_hex(b"");
        assert_eq!(empty, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        let (canonical, signed) = canonical_request("GET", "/", "", &vanilla_headers(), &empty);
        assert_eq!(
            canonical,
            "GET\n/\n\nhost:example.amazonaws.com\nx-amz-date:20150830T123600Z\n\n\
             host;x-amz-date\ne3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(signed, "host;x-amz-date");
        let to_sign = string_to_sign(DATE, "20150830/us-east-1/service/aws4_request", &canonical);
        assert_eq!(
            to_sign,
            "AWS4-HMAC-SHA256\n20150830T123600Z\n20150830/us-east-1/service/aws4_request\n\
             bb579772317eb040ac9ed261061d46c1f17a8133879d6129b6e1c25292927e63"
        );
    }

    #[test]
    fn signature_du_vecteur_officiel() {
        let signer = Signer {
            access_key: "AKIDEXAMPLE",
            secret_key: SECRET,
            region: "us-east-1",
            service: "service",
        };
        let authorization =
            signer.authorization(DATE, "GET", "/", "", &vanilla_headers(), &sha256_hex(b""));
        assert_eq!(
            authorization,
            "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request, \
             SignedHeaders=host;x-amz-date, \
             Signature=5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31"
        );
    }

    #[test]
    fn les_valeurs_d_en_tete_sont_epurees() {
        let headers = vec![("Content-Type".to_string(), "  a   b ".to_string())];
        let (canonical, signed) = canonical_request("POST", "/", "", &headers, "h");
        assert!(canonical.contains("\ncontent-type:a b\n"), "{canonical}");
        assert_eq!(signed, "content-type");
    }
}
