//! Géolocalisation IP → pays, sans compte et sans service tiers.
//!
//! La base est « IP to Country Lite » de DB-IP (CC BY 4.0 : l'interface cite
//! « IP Geolocation by DB-IP » avec un lien partout où elle s'en sert), un CSV
//! gzippé de quelques mégaoctets, publié chaque mois à une adresse sans
//! inscription. Le serveur la télécharge la première fois qu'une carte en
//! direct en a besoin, puis quand elle a plus d'un mois ; elle est rangée dans
//! `<data>/geoip/` et chargée en mémoire (quelques mégaoctets). Une adresse
//! n'est jamais envoyée à personne : la recherche est locale.
//!
//! Désactivable dans les réglages (`geoip.enabled`) : plus aucun
//! téléchargement, et la carte ne compte plus que « réseau local » et
//! « inconnu ».

use std::io::Read;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing::{info, warn};

use crate::auth::settings;

pub const SOURCE: &str = "DB-IP";
pub const SOURCE_URL: &str = "https://db-ip.com";
const DOWNLOAD: &str = "https://download.db-ip.com/free/dbip-country-lite";
const FILE: &str = "dbip-country-lite.csv.gz";
const SETTING: &str = "geoip.enabled";
/// La base compressée fait 4 à 5 Mo ; au-delà de 64 Mo, ce n'est pas elle.
const MAX_DOWNLOAD: usize = 64 * 1024 * 1024;
/// Une base de plus d'un mois est remplacée (DB-IP publie au début du mois).
const MAX_AGE: Duration = Duration::from_secs(32 * 24 * 3600);
/// Après un échec, on ne réessaie pas avant six heures.
const RETRY: Duration = Duration::from_secs(6 * 3600);

/// D'où vient une adresse, pour la carte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    /// Réseau privé, boucle locale, lien local, CGNAT, multidiffusion.
    Local,
    Country([u8; 2]),
    Unknown,
}

impl Origin {
    /// `CH`, `local` ou `unknown` : la clé que l'API rend.
    pub fn code(self) -> String {
        match self {
            Origin::Local => "local".into(),
            Origin::Unknown => "unknown".into(),
            Origin::Country(cc) => String::from_utf8_lossy(&cc).into_owned(),
        }
    }
}

/// La base en mémoire : des plages contiguës triées, réduites à leur début.
#[derive(Debug, Default)]
pub struct GeoDb {
    v4_starts: Vec<u32>,
    v4_codes: Vec<[u8; 2]>,
    v6_starts: Vec<u128>,
    v6_codes: Vec<[u8; 2]>,
}

/// Code « pas de pays » : réservé, ou un trou entre deux plages.
const NONE: [u8; 2] = *b"ZZ";

impl GeoDb {
    /// Lit le CSV `début,fin,pays` de DB-IP (trié par adresse). Une ligne
    /// illisible est sautée ; les plages voisines du même pays sont fusionnées.
    pub fn parse(text: &str) -> Self {
        let mut db = GeoDb::default();
        let mut v4_end: Option<u32> = None;
        let mut v6_end: Option<u128> = None;
        for line in text.lines() {
            let mut fields = line.split(',');
            let (Some(start), Some(end), Some(code)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            let code = code.trim().trim_matches('"').as_bytes();
            let code = if code.len() == 2 && code.iter().all(u8::is_ascii_alphabetic) {
                [code[0].to_ascii_uppercase(), code[1].to_ascii_uppercase()]
            } else {
                NONE
            };
            match (start.trim().parse::<IpAddr>(), end.trim().parse::<IpAddr>()) {
                (Ok(IpAddr::V4(s)), Ok(IpAddr::V4(e))) => {
                    let (s, e) = (u32::from(s), u32::from(e));
                    push_range(&mut db.v4_starts, &mut db.v4_codes, &mut v4_end, s, e, code);
                }
                (Ok(IpAddr::V6(s)), Ok(IpAddr::V6(e))) => {
                    let (s, e) = (u128::from(s), u128::from(e));
                    push_range(&mut db.v6_starts, &mut db.v6_codes, &mut v6_end, s, e, code);
                }
                _ => continue,
            }
        }
        close(&mut db.v4_starts, &mut db.v4_codes, v4_end, u32::MAX);
        close(&mut db.v6_starts, &mut db.v6_codes, v6_end, u128::MAX);
        db
    }

    pub fn len(&self) -> usize {
        self.v4_starts.len() + self.v6_starts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Le pays d'une adresse publique, s'il est connu.
    pub fn country(&self, ip: IpAddr) -> Option<[u8; 2]> {
        let code = match ip {
            IpAddr::V4(v4) => find(&self.v4_starts, &self.v4_codes, u32::from(v4)),
            IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
                Some(v4) => find(&self.v4_starts, &self.v4_codes, u32::from(v4)),
                None => find(&self.v6_starts, &self.v6_codes, u128::from(v6)),
            },
        }?;
        (code != NONE).then_some(code)
    }
}

/// Ajoute une plage, en bouchant un trou éventuel par « pas de pays » et en
/// fusionnant avec la précédente si le pays est le même.
fn push_range<T>(
    starts: &mut Vec<T>,
    codes: &mut Vec<[u8; 2]>,
    end: &mut Option<T>,
    s: T,
    e: T,
    code: [u8; 2],
) where
    T: Copy + Ord + std::ops::Add<Output = T> + std::ops::Sub<Output = T> + From<u8>,
{
    if e < s || end.is_some_and(|prev| s <= prev) {
        return;
    }
    if let Some(prev) = *end
        && s - prev > T::from(1)
    {
        starts.push(prev + T::from(1));
        codes.push(NONE);
    }
    if codes.last() != Some(&code) {
        starts.push(s);
        codes.push(code);
    }
    *end = Some(e);
}

/// Après la dernière plage, plus aucun pays.
fn close<T>(starts: &mut Vec<T>, codes: &mut Vec<[u8; 2]>, end: Option<T>, max: T)
where
    T: Copy + Ord + std::ops::Add<Output = T> + From<u8>,
{
    if let Some(end) = end
        && end < max
        && codes.last() != Some(&NONE)
    {
        starts.push(end + T::from(1));
        codes.push(NONE);
    }
}

fn find<T: Copy + Ord>(starts: &[T], codes: &[[u8; 2]], ip: T) -> Option<[u8; 2]> {
    let index = starts.partition_point(|s| *s <= ip).checked_sub(1)?;
    Some(codes[index])
}

/// Une adresse qui ne sort pas sur Internet : elle ne se géolocalise pas.
pub fn is_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                // 100.64.0.0/10, l'adressage partagé des opérateurs (CGNAT).
                || (o[0] == 100 && (o[1] & 0xC0) == 64)
        }
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_local(IpAddr::V4(v4)),
            None => {
                let first = v6.segments()[0];
                v6.is_loopback()
                    || v6.is_unspecified()
                    || v6.is_multicast()
                    || (first & 0xfe00) == 0xfc00 // fc00::/7, adresses uniques locales
                    || (first & 0xffc0) == 0xfe80 // fe80::/10, lien local
            }
        },
    }
}

/// D'où vient `text` (une adresse telle que le journal l'écrit).
pub fn classify(text: &str, db: Option<&GeoDb>) -> Origin {
    let Ok(ip) = text.trim().parse::<IpAddr>() else { return Origin::Unknown };
    if is_local(ip) {
        return Origin::Local;
    }
    db.and_then(|db| db.country(ip)).map_or(Origin::Unknown, Origin::Country)
}

// ------------------------------------------------------------- service

/// État de la base, tel que l'interface le montre.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GeoStatus {
    pub enabled: bool,
    /// `ready`, `loading`, `missing`, `failed` ou `disabled`.
    pub state: &'static str,
    /// Mois de la base chargée (`2026-10`), d'après son nom au téléchargement.
    pub month: Option<String>,
    pub ranges: usize,
    pub error: Option<String>,
    pub source: &'static str,
    pub source_url: &'static str,
}

#[derive(Default)]
struct Inner {
    db: Option<Arc<GeoDb>>,
    month: Option<String>,
    loading: bool,
    error: Option<String>,
    last_attempt: Option<Instant>,
}

static GEO: LazyLock<Mutex<Inner>> = LazyLock::new(Mutex::default);

#[derive(Debug, Serialize, Deserialize)]
struct Setting {
    enabled: bool,
}

pub async fn enabled(pool: &SqlitePool) -> bool {
    settings::get::<Setting>(pool, SETTING).await.ok().flatten().is_none_or(|s| s.enabled)
}

pub async fn set_enabled(pool: &SqlitePool, on: bool) -> anyhow::Result<()> {
    settings::set(pool, SETTING, &Setting { enabled: on }).await?;
    if !on && let Ok(mut inner) = GEO.lock() {
        inner.db = None;
        inner.month = None;
    }
    Ok(())
}

fn dir(data_dir: &Path) -> PathBuf {
    data_dir.join("geoip")
}

/// La base chargée, si elle l'est. Ne bloque jamais : le chargement et le
/// téléchargement partent en tâche de fond ([`ensure`]).
pub fn current() -> Option<Arc<GeoDb>> {
    GEO.lock().ok().and_then(|inner| inner.db.clone())
}

pub async fn status(pool: &SqlitePool) -> GeoStatus {
    let on = enabled(pool).await;
    let inner = GEO
        .lock()
        .map(|i| (i.db.as_ref().map(|d| d.len()), i.month.clone(), i.loading, i.error.clone()));
    let (ranges, month, loading, error) = inner.unwrap_or((None, None, false, None));
    let state = match (on, ranges, loading, &error) {
        (false, _, _, _) => "disabled",
        (true, Some(_), _, _) => "ready",
        (true, None, true, _) => "loading",
        (true, None, false, Some(_)) => "failed",
        (true, None, false, None) => "missing",
    };
    GeoStatus {
        enabled: on,
        state,
        month: if on { month } else { None },
        ranges: if on { ranges.unwrap_or(0) } else { 0 },
        error: if on { error } else { None },
        source: SOURCE,
        source_url: SOURCE_URL,
    }
}

/// Charge la base depuis le disque, ou la télécharge si elle manque ou date
/// de plus d'un mois — en arrière-plan, une seule fois à la fois.
pub async fn ensure(pool: &SqlitePool, data_dir: &Path) {
    if !enabled(pool).await {
        return;
    }
    let path = dir(data_dir).join(FILE);
    let age = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok());
    let stale = age.is_none_or(|a| a > MAX_AGE);
    {
        let Ok(mut inner) = GEO.lock() else { return };
        if inner.loading || (inner.db.is_some() && !stale) {
            return;
        }
        if inner.last_attempt.is_some_and(|t| t.elapsed() < RETRY) {
            return;
        }
        inner.loading = true;
        inner.last_attempt = Some(Instant::now());
    }
    let data_dir = data_dir.to_path_buf();
    tokio::spawn(async move {
        let outcome = refresh(&data_dir, stale).await;
        let Ok(mut inner) = GEO.lock() else { return };
        inner.loading = false;
        match outcome {
            Ok((db, month)) => {
                info!(ranges = db.len(), %month, "base de géolocalisation chargée");
                inner.db = Some(Arc::new(db));
                inner.month = Some(month);
                inner.error = None;
            }
            Err(error) => {
                warn!(%error, "base de géolocalisation indisponible");
                inner.error = Some(error);
            }
        }
    });
}

async fn refresh(data_dir: &Path, download: bool) -> Result<(GeoDb, String), String> {
    let folder = dir(data_dir);
    let path = folder.join(FILE);
    let month_path = folder.join("month");
    if download {
        match fetch().await {
            Ok((bytes, month)) => {
                tokio::fs::create_dir_all(&folder).await.map_err(|e| e.to_string())?;
                let partial = folder.join(format!("{FILE}.part"));
                tokio::fs::write(&partial, &bytes).await.map_err(|e| e.to_string())?;
                tokio::fs::rename(&partial, &path).await.map_err(|e| e.to_string())?;
                let _ = tokio::fs::write(&month_path, &month).await;
            }
            // Pas de réseau, mais une ancienne base sur le disque : on la garde.
            Err(error) if path.exists() => warn!(%error, "base de géolocalisation non mise à jour"),
            Err(error) => return Err(error),
        }
    }
    let month = tokio::fs::read_to_string(&month_path).await.unwrap_or_default().trim().to_string();
    let db = tokio::task::spawn_blocking(move || load_file(&path))
        .await
        .map_err(|e| e.to_string())??;
    Ok((db, month))
}

fn load_file(path: &Path) -> Result<GeoDb, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut text = String::new();
    flate2::read::GzDecoder::new(file)
        .take(512 * 1024 * 1024)
        .read_to_string(&mut text)
        .map_err(|e| format!("base illisible : {e}"))?;
    let db = GeoDb::parse(&text);
    if db.is_empty() {
        return Err("the geolocation file holds no range".into());
    }
    Ok(db)
}

/// Les deux derniers mois, du plus récent : au tout début d'un mois, la base
/// du mois n'est pas toujours encore publiée.
pub fn candidate_months(now: chrono::DateTime<chrono::Utc>) -> [String; 2] {
    use chrono::Datelike;
    let (y, m) = (now.year(), now.month());
    let (py, pm) = if m == 1 { (y - 1, 12) } else { (y, m - 1) };
    [format!("{y:04}-{m:02}"), format!("{py:04}-{pm:02}")]
}

async fn fetch() -> Result<(Vec<u8>, String), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .user_agent(concat!("DumbMonit/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let mut last = String::new();
    for month in candidate_months(chrono::Utc::now()) {
        let url = format!("{DOWNLOAD}-{month}.csv.gz");
        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                let bytes = response.bytes().await.map_err(|e| e.to_string())?;
                if bytes.len() > MAX_DOWNLOAD {
                    return Err("the geolocation download is unexpectedly large".into());
                }
                return Ok((bytes.to_vec(), month));
            }
            Ok(response) => last = format!("{url}: {}", response.status()),
            Err(error) => last = format!("{url}: {error}"),
        }
    }
    Err(format!("could not download the DB-IP country database ({last})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CSV: &str = "0.0.0.0,0.255.255.255,ZZ\n\
        1.0.0.0,1.0.0.255,AU\n\
        1.0.1.0,1.0.3.255,CN\n\
        1.0.4.0,1.0.7.255,AU\n\
        5.0.0.0,5.0.0.255,CH\n\
        5.0.1.0,5.0.1.255,CH\n\
        pas une ligne\n\
        2a02:1210::,2a02:1210:ffff:ffff:ffff:ffff:ffff:ffff,CH\n\
        2a02:1211::,2a02:1211:ffff:ffff:ffff:ffff:ffff:ffff,fr\n";

    #[test]
    fn une_adresse_trouve_son_pays() {
        let db = GeoDb::parse(CSV);
        let at = |ip: &str| {
            db.country(ip.parse().unwrap()).map(|c| String::from_utf8_lossy(&c).to_string())
        };
        assert_eq!(at("1.0.0.42").as_deref(), Some("AU"));
        assert_eq!(at("1.0.2.1").as_deref(), Some("CN"));
        assert_eq!(at("5.0.1.200").as_deref(), Some("CH"));
        assert_eq!(at("0.1.2.3"), None, "ZZ : pas de pays");
        assert_eq!(at("1.0.8.1"), None, "un trou entre deux plages n'est d'aucun pays");
        assert_eq!(at("9.9.9.9"), None, "après la dernière plage");
        assert_eq!(at("2a02:1210:5c00::1d").as_deref(), Some("CH"));
        assert_eq!(at("2a02:1211::1").as_deref(), Some("FR"));
        assert_eq!(at("::ffff:1.0.0.1").as_deref(), Some("AU"), "IPv4 dans IPv6");
    }

    #[test]
    fn les_plages_voisines_du_meme_pays_sont_fusionnees() {
        let db = GeoDb::parse(CSV);
        // ZZ, AU, CN, AU, (trou) ZZ, CH (fusionnée), puis la fin : 7 en IPv4.
        assert_eq!(db.v4_starts.len(), 7);
        // CH, FR, la fin.
        assert_eq!(db.v6_starts.len(), 3);
    }

    #[test]
    fn les_adresses_privees_sont_le_reseau_local() {
        let db = GeoDb::parse(CSV);
        for ip in [
            "192.168.1.23",
            "10.0.0.1",
            "172.16.5.4",
            "100.64.1.1",
            "127.0.0.1",
            "169.254.1.1",
            "224.0.0.18",
            "fd00::1",
            "fe80::1",
            "::1",
            "::ffff:192.168.1.1",
        ] {
            assert_eq!(classify(ip, Some(&db)), Origin::Local, "{ip}");
        }
        assert_eq!(classify("1.0.0.1", Some(&db)), Origin::Country(*b"AU"));
        assert_eq!(classify("1.0.0.1", None), Origin::Unknown, "sans base, inconnu");
        assert_eq!(classify("n'importe quoi", Some(&db)), Origin::Unknown);
        assert_eq!(Origin::Country(*b"CH").code(), "CH");
        assert_eq!(Origin::Local.code(), "local");
    }

    #[test]
    fn le_mois_precedent_sert_de_repli() {
        use chrono::TimeZone;
        let jan = chrono::Utc.with_ymd_and_hms(2027, 1, 2, 0, 0, 0).unwrap();
        assert_eq!(candidate_months(jan), ["2027-01".to_string(), "2026-12".to_string()]);
        let oct = chrono::Utc.with_ymd_and_hms(2026, 10, 19, 0, 0, 0).unwrap();
        assert_eq!(candidate_months(oct)[0], "2026-10");
    }
}
