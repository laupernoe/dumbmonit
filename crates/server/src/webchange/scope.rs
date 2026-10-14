//! Quelles adresses font partie du site surveillé, et sous quelle forme.
//!
//! Une même page s'écrit de dix façons (`/a`, `/a#haut`, `HTTP://Site/a`,
//! `../a`) : chacune deviendrait une page à part, avec ses propres
//! « nouveautés ». Toute adresse passe donc par [`Scope::accept`], qui la
//! résout, la normalise et décide si elle est dans le périmètre : même origine
//! que l'adresse de départ, sous le préfixe de chemin, et pas un fichier
//! manifestement autre qu'une page.

use reqwest::Url;

/// Extensions qui ne désignent jamais une page HTML : les suivre coûterait une
/// requête pour rien (le type de contenu les écarterait de toute façon).
const ASSET_EXTENSIONS: &[&str] = &[
    "7z", "aac", "apk", "atom", "avi", "avif", "bin", "bmp", "bz2", "css", "csv", "deb", "dmg",
    "doc", "docx", "eot", "epub", "exe", "flac", "gif", "gz", "heic", "ico", "iso", "jpeg", "jpg",
    "js", "json", "m4a", "map", "mjs", "mkv", "mov", "mp3", "mp4", "msi", "odp", "ods", "odt",
    "ogg", "otf", "pdf", "png", "ppt", "pptx", "rar", "rpm", "rss", "svg", "tar", "tgz", "tif",
    "tiff", "ttf", "txt", "wasm", "wav", "webm", "webp", "woff", "woff2", "xls", "xlsx", "xml",
    "xz", "zip", "zst",
];

/// Le périmètre d'une surveillance de site.
/// Paramètres d'URL de suivi publicitaire ou analytique : ils ne changent
/// jamais la page, seulement l'adresse, et en feraient autant de pages.
const TRACKING_PARAMS: &[&str] = &[
    "fbclid", "gclid", "dclid", "gbraid", "wbraid", "msclkid", "yclid", "igshid", "mc_cid",
    "mc_eid", "_ga", "_gl", "_hsenc", "_hsmi", "mkt_tok",
];

#[derive(Debug, Clone)]
pub struct Scope {
    start: Url,
    prefix: String,
    /// Origine où la page de départ a mené, quand une redirection a changé
    /// d'hôte (`ex.fr` → `www.ex.fr`) : c'est là qu'est le site.
    redirected: Option<Url>,
    /// Retirer toute la requête (`?…`) des adresses suivies.
    ignore_query: bool,
}

impl Scope {
    /// `prefix` : chemin sous lequel rester ; `None` prend le répertoire de
    /// l'adresse de départ.
    pub fn new(start: &Url, prefix: Option<&str>) -> Self {
        let prefix = match prefix.map(str::trim).filter(|p| !p.is_empty()) {
            Some(prefix) if prefix.starts_with('/') => prefix.to_string(),
            Some(prefix) => format!("/{prefix}"),
            None => default_prefix(start),
        };
        Self { start: normalise(start.clone()), prefix, redirected: None, ignore_query: false }
    }

    /// Les adresses qui ne diffèrent que par leur requête sont une même page.
    pub fn with_ignore_query(mut self, ignore: bool) -> Self {
        self.ignore_query = ignore;
        self
    }

    /// La page de départ a été servie depuis `final_url` : si l'origine a
    /// changé, le site est là, et les liens vers l'origine de départ y sont
    /// ramenés plutôt que lus deux fois.
    pub fn follow(&mut self, final_url: &Url) {
        if final_url.origin() != self.start.origin() {
            self.redirected = Some(final_url.clone());
        }
    }

    /// `url`, de l'origine de départ, ramenée à celle de la redirection.
    fn on_site(&self, mut url: Url) -> Url {
        if let Some(site) = &self.redirected {
            let _ = url.set_scheme(site.scheme());
            let _ = url.set_host(site.host_str());
            let _ = url.set_port(site.port());
        }
        url
    }

    /// Une adresse de l'origine où se trouve le site (départ, ou redirection).
    pub fn site_root(&self) -> &Url {
        self.redirected.as_ref().unwrap_or(&self.start)
    }

    pub fn start(&self) -> &Url {
        &self.start
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Résout `href` par rapport à `base` et rend l'adresse normalisée si elle
    /// est dans le périmètre.
    pub fn accept(&self, base: &Url, href: &str) -> Option<Url> {
        let href = href.trim();
        if href.is_empty() || href.starts_with('#') {
            return None;
        }
        let url = base.join(href).ok()?;
        self.contains(url)
    }

    /// Rend l'adresse normalisée si elle est dans le périmètre.
    pub fn contains(&self, url: Url) -> Option<Url> {
        if !matches!(url.scheme(), "http" | "https") {
            return None;
        }
        let mut url = normalise(url);
        if self.ignore_query {
            url.set_query(None);
        }
        if url == self.start {
            return Some(url);
        }
        if url.origin() == self.start.origin() {
            url = self.on_site(url);
        }
        if url.origin() != self.site_root().origin() {
            return None;
        }
        // La page de départ, servie depuis l'origine de la redirection.
        if url == self.on_site(self.start.clone()) {
            return Some(self.start.clone());
        }
        if !under_prefix(url.path(), &self.prefix) || looks_like_asset(url.path()) {
            return None;
        }
        Some(url)
    }
}

/// Forme canonique : sans fragment, sans paramètres de suivi (`utm_…`,
/// `fbclid`…), sans `?` vide. `Url` s'occupe déjà de la
/// casse du schéma et de l'hôte, du port par défaut et des `.`/`..`.
pub fn normalise(mut url: Url) -> Url {
    url.set_fragment(None);
    if url.query().is_some() {
        let kept: Vec<(String, String)> = url
            .query_pairs()
            .filter(|(key, _)| !is_tracking(key))
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect();
        let had_tracking = url.query_pairs().count() != kept.len();
        if kept.is_empty() {
            url.set_query(None);
        } else if had_tracking {
            url.query_pairs_mut().clear().extend_pairs(kept);
        }
    }
    url
}

fn is_tracking(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.starts_with("utm_") || TRACKING_PARAMS.contains(&key.as_str())
}

/// Répertoire de l'adresse de départ : `/docs/` pour `/docs/intro.html` comme
/// pour `/docs/`, `/` pour `/docs`.
pub fn default_prefix(start: &Url) -> String {
    let path = start.path();
    match path.rfind('/') {
        Some(at) => path[..=at].to_string(),
        None => "/".to_string(),
    }
}

/// Un préfixe sans barre finale (`/docs`) couvre `/docs` et `/docs/…`, mais pas
/// `/docsearch`.
fn under_prefix(path: &str, prefix: &str) -> bool {
    if prefix.ends_with('/') {
        return path.starts_with(prefix);
    }
    path == prefix || path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/'))
}

/// Vrai pour un chemin qui finit par une extension de fichier non HTML.
pub fn looks_like_asset(path: &str) -> bool {
    let last = path.rsplit('/').next().unwrap_or_default();
    let Some((_, extension)) = last.rsplit_once('.') else { return false };
    ASSET_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Url {
        Url::parse(text).unwrap()
    }

    #[test]
    fn une_redirection_vers_un_autre_hote_deplace_le_site() {
        let mut scope = Scope::new(&url("https://ex.fr/"), None);
        scope.follow(&url("https://www.ex.fr/accueil"));
        let base = url("https://www.ex.fr/accueil");
        let accept = |href: &str| scope.accept(&base, href).map(|u| u.to_string());
        assert_eq!(accept("/tarifs").as_deref(), Some("https://www.ex.fr/tarifs"));
        assert_eq!(accept("https://ex.fr/contact").as_deref(), Some("https://www.ex.fr/contact"));
        assert_eq!(accept("https://www.ex.fr/").as_deref(), Some("https://ex.fr/"), "le départ");
        assert_eq!(accept("https://autre.fr/"), None);

        let mut same = Scope::new(&url("https://ex.fr/"), None);
        same.follow(&url("https://ex.fr/fr/"));
        assert_eq!(same.site_root().as_str(), "https://ex.fr/");
    }

    #[test]
    fn les_parametres_de_suivi_sont_retires() {
        let scope = Scope::new(&url("https://ex.fr/"), None);
        let base = url("https://ex.fr/");
        let accept = |href: &str| scope.accept(&base, href).map(|u| u.to_string());
        assert_eq!(accept("/a?utm_source=x&UTM_medium=y").as_deref(), Some("https://ex.fr/a"));
        assert_eq!(accept("/a?id=3&fbclid=abc").as_deref(), Some("https://ex.fr/a?id=3"));
        assert_eq!(accept("/a?b=1&c=2").as_deref(), Some("https://ex.fr/a?b=1&c=2"), "intact");

        let without = Scope::new(&url("https://ex.fr/"), None).with_ignore_query(true);
        assert_eq!(
            without.accept(&base, "/a?page=2").map(|u| u.to_string()).as_deref(),
            Some("https://ex.fr/a")
        );
    }

    #[test]
    fn le_prefixe_par_defaut_est_le_repertoire_de_depart() {
        assert_eq!(default_prefix(&url("https://ex.fr/docs/intro.html")), "/docs/");
        assert_eq!(default_prefix(&url("https://ex.fr/docs/")), "/docs/");
        assert_eq!(default_prefix(&url("https://ex.fr/docs")), "/");
        assert_eq!(default_prefix(&url("https://ex.fr")), "/");
    }

    #[test]
    fn les_liens_sont_resolus_et_normalises() {
        let scope = Scope::new(&url("https://ex.fr/docs/"), None);
        let base = url("https://ex.fr/docs/guide/page.html");
        let accept = |href: &str| scope.accept(&base, href).map(|u| u.to_string());
        assert_eq!(
            accept("autre.html#section").as_deref(),
            Some("https://ex.fr/docs/guide/autre.html")
        );
        assert_eq!(accept("../index.html").as_deref(), Some("https://ex.fr/docs/index.html"));
        assert_eq!(accept("HTTPS://EX.FR:443/docs/x?").as_deref(), Some("https://ex.fr/docs/x"));
        assert_eq!(accept("/docs/a?b=1").as_deref(), Some("https://ex.fr/docs/a?b=1"));
        assert_eq!(accept("#haut"), None);
        assert_eq!(accept(""), None);
    }

    #[test]
    fn le_perimetre_ecarte_les_autres_origines_chemins_et_fichiers() {
        let scope = Scope::new(&url("https://ex.fr/docs/"), None);
        let base = url("https://ex.fr/docs/");
        for refuse in [
            "https://autre.fr/docs/a",
            "http://ex.fr/docs/a",
            "https://ex.fr:8443/docs/a",
            "/blog/a",
            "/docsearch",
            "mailto:moi@ex.fr",
            "javascript:void(0)",
            "tel:+33100000000",
            "/docs/logo.PNG",
            "/docs/guide.pdf",
            "/docs/style.css?v=3",
        ] {
            assert_eq!(scope.accept(&base, refuse), None, "{refuse}");
        }
        assert!(scope.accept(&base, "/docs/v1.2/").is_some(), "un point dans un répertoire");
        assert!(scope.accept(&base, "/docs/page.html").is_some());
        assert!(scope.accept(&base, "/docs/page.php?id=2").is_some());
    }

    #[test]
    fn un_prefixe_explicite_sans_barre_finale_ne_deborde_pas() {
        let scope = Scope::new(&url("https://ex.fr/"), Some("docs"));
        let base = url("https://ex.fr/");
        assert_eq!(scope.prefix(), "/docs");
        assert!(scope.accept(&base, "/docs").is_some());
        assert!(scope.accept(&base, "/docs/a").is_some());
        assert!(scope.accept(&base, "/docsearch").is_none());
        // L'adresse de départ est toujours dans le périmètre.
        assert!(scope.accept(&base, "/").is_some());
    }
}
