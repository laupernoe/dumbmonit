//! Historique long terme : agrégats cinq minutes, et bascule des lectures.
//!
//! VictoriaMetrics open source n'a pas de sous-échantillonnage, et une seule
//! rétention par instance. L'historique long terme vit donc dans une seconde
//! instance (`/data/vm-long`), qu'alimente l'agrégateur du serveur
//! (`crate::storage::aggregator`) : toutes les cinq minutes, pour les séries
//! du [`CATALOG`], il écrit
//!
//! - sous le **nom d'origine** : la moyenne de la fenêtre (jauges), ou la
//!   dernière valeur (compteurs — `rate()` et `increase()` restent justes sur
//!   des points espacés de cinq minutes) ;
//! - `nom:5m_min` et `nom:5m_max` : extrêmes de la fenêtre (jauges) ;
//! - `nom:5m_sum` et `nom:5m_count` : somme et nombre de mesures, pour les
//!   séries de disponibilité (`up`, `probe_success`). Une disponibilité
//!   « somme des succès / nombre de mesures » se recompose donc exactement.
//!
//! Chaque point est horodaté à la **fin** de sa fenêtre `]T-5m, T]` : la même
//! fenêtre que `[5m]` évaluée en `T`, ce qui garde exactes les tranches de
//! présence des pages de statut.
//!
//! Une lecture dont la plage dépasse la rétention brute est réécrite
//! ([`rewrite`]) puis envoyée à l'instance long terme ; une plage de graphe est
//! recousue : agrégats pour le passé, mesures brutes pour le récent.

use std::collections::BTreeMap;

/// Pas des agrégats, en secondes.
pub const STEP_SECS: i64 = 300;

/// Rôle d'une série dans l'historique long terme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Disponibilité (0/1) : moyenne, min, max, somme et nombre de mesures.
    Availability,
    /// Jauge : moyenne, min et max.
    Gauge,
    /// Compteur : dernière valeur de la fenêtre.
    Counter,
}

/// Séries conservées au-delà de la rétention brute (noms sans préfixe).
///
/// Volontairement limité à ce que montrent les graphes, les pages de statut et
/// les rapports : disponibilité, latence, CPU, mémoire, disque, trafic. Tout le
/// reste n'existe qu'en brut.
pub const CATALOG: &[(&str, Kind)] = &[
    ("up", Kind::Availability),
    ("probe_success", Kind::Availability),
    ("probe_duration_seconds", Kind::Gauge),
    ("cpu_usage_percent", Kind::Gauge),
    ("memory_used_percent", Kind::Gauge),
    ("swap_used_percent", Kind::Gauge),
    ("filesystem_used_percent", Kind::Gauge),
    ("load_average_1", Kind::Gauge),
    ("load_average_5", Kind::Gauge),
    ("load_average_15", Kind::Gauge),
    ("proxmox_guest_cpu_percent", Kind::Gauge),
    ("proxmox_guest_memory_percent", Kind::Gauge),
    ("if_octets_in", Kind::Counter),
    ("if_octets_out", Kind::Counter),
    ("disk_read_bytes", Kind::Counter),
    ("disk_written_bytes", Kind::Counter),
    ("proxmox_guest_network_in_bytes", Kind::Counter),
    ("proxmox_guest_network_out_bytes", Kind::Counter),
    ("mikrotik_interface_rx_bytes_total", Kind::Counter),
    ("mikrotik_interface_tx_bytes_total", Kind::Counter),
];

/// Rôle d'une métrique (nom complet, préfixe compris).
pub fn kind_of(full_name: &str) -> Option<Kind> {
    let name = full_name.strip_prefix(super::METRIC_PREFIX)?;
    CATALOG.iter().find(|(known, _)| *known == name).map(|(_, kind)| *kind)
}

/// Une requête de l'agrégateur : l'expression MetricsQL sur l'instance brute,
/// et le suffixe ajouté au nom des séries qu'elle rend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateQuery {
    pub query: String,
    pub suffix: &'static str,
}

fn names_regex(kinds: &[Kind]) -> String {
    let names: Vec<&str> =
        CATALOG.iter().filter(|(_, kind)| kinds.contains(kind)).map(|(name, _)| *name).collect();
    format!("{}({})", super::METRIC_PREFIX, names.join("|"))
}

/// Les six requêtes d'un passage, à évaluer en `query_range` au pas de
/// [`STEP_SECS`] : chaque point en `T` résume la fenêtre `]T-5m, T]`.
pub fn aggregate_queries() -> Vec<AggregateQuery> {
    let gauges = names_regex(&[Kind::Gauge, Kind::Availability]);
    let availability = names_regex(&[Kind::Availability]);
    let counters = names_regex(&[Kind::Counter]);
    let build = |function: &str, regex: &str, suffix: &'static str| AggregateQuery {
        query: format!("{function}({{__name__=~\"{regex}\"}}[5m]) keep_metric_names"),
        suffix,
    };
    vec![
        build("avg_over_time", &gauges, ""),
        build("min_over_time", &gauges, ":5m_min"),
        build("max_over_time", &gauges, ":5m_max"),
        build("sum_over_time", &availability, ":5m_sum"),
        build("count_over_time", &availability, ":5m_count"),
        build("last_over_time", &counters, ""),
    ]
}

/// Fin de la dernière fenêtre complète à `now_secs`, avec `settle_secs` de
/// marge pour les écritures en retard (lots d'agents, latence de VictoriaMetrics).
pub fn last_complete_window(now_secs: i64, settle_secs: i64) -> i64 {
    (now_secs - settle_secs).div_euclid(STEP_SECS) * STEP_SECS
}

/// Aligne un horodatage (secondes) sur la grille des fenêtres, vers le bas.
pub fn align_down(secs: i64) -> i64 {
    secs.div_euclid(STEP_SECS) * STEP_SECS
}

// --------------------------------------------------------------------------
// Durées et profondeur d'une requête
// --------------------------------------------------------------------------

/// Durée MetricsQL (`5m`, `1h30m`, `1.5d`, `90d`) en secondes. Un nombre nu
/// est compté en secondes ; `5i` (multiples du pas) n'est pas compris.
pub fn parse_duration_secs(raw: &str) -> Option<f64> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(secs) = raw.parse::<f64>() {
        return Some(secs);
    }
    let bytes = raw.as_bytes();
    let mut total = 0.0;
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
            i += 1;
        }
        let number: f64 = raw[start..i].parse().ok()?;
        let unit_start = i;
        while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
            i += 1;
        }
        let factor = match &raw[unit_start..i] {
            "ms" => 0.001,
            "s" => 1.0,
            "m" => 60.0,
            "h" => 3_600.0,
            "d" => 86_400.0,
            "w" => 604_800.0,
            "y" => 31_536_000.0,
            _ => return None,
        };
        total += number * factor;
    }
    Some(total)
}

/// Morceaux d'une requête : ce qui est entre guillemets n'est jamais lu comme
/// du code (une regex `"a[0-9]"` n'est pas une fenêtre).
#[derive(Debug, Clone, PartialEq)]
enum Token<'a> {
    Ident { text: &'a str, at: usize },
    Str,
    Open,
    Close,
    Bracket(&'a str),
    Brace,
    Other,
}

fn tokenize(query: &str) -> Vec<Token<'_>> {
    let bytes = query.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'"' | b'\'' | b'`' => {
                let quote = c;
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    if bytes[i] == b'\\' && quote != b'`' {
                        i += 1;
                    }
                    i += 1;
                }
                i += 1;
                tokens.push(Token::Str);
            }
            b'(' => {
                tokens.push(Token::Open);
                i += 1;
            }
            b')' => {
                tokens.push(Token::Close);
                i += 1;
            }
            b'[' => {
                let start = i + 1;
                while i < bytes.len() && bytes[i] != b']' {
                    i += 1;
                }
                tokens.push(Token::Bracket(&query[start..i.min(bytes.len())]));
                i += 1;
            }
            b'{' => {
                // Le contenu d'un sélecteur : noms d'étiquettes et valeurs entre
                // guillemets, sans fonction ni métrique à réécrire.
                let mut j = i + 1;
                while j < bytes.len() && bytes[j] != b'}' {
                    if bytes[j] == b'"' || bytes[j] == b'\'' || bytes[j] == b'`' {
                        let quote = bytes[j];
                        j += 1;
                        while j < bytes.len() && bytes[j] != quote {
                            if bytes[j] == b'\\' && quote != b'`' {
                                j += 1;
                            }
                            j += 1;
                        }
                    }
                    j += 1;
                }
                tokens.push(Token::Brace);
                i = j + 1;
            }
            c if c.is_ascii_alphabetic() || c == b'_' || c == b':' => {
                let start = i;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b':')
                {
                    i += 1;
                }
                tokens.push(Token::Ident { text: &query[start..i], at: start });
            }
            _ => {
                tokens.push(Token::Other);
                i += 1;
            }
        }
    }
    tokens
}

/// Profondeur d'une requête, en secondes : la plus grande fenêtre (`[90d]`,
/// `[90d:5m]`) plus le plus grand décalage (`offset 30d`). C'est ce qui, avant
/// l'instant d'évaluation, doit être en base pour que la réponse soit complète.
pub fn lookback_secs(query: &str) -> i64 {
    let tokens = tokenize(query);
    let mut window: f64 = 0.0;
    let mut offset: f64 = 0.0;
    for token in &tokens {
        if let Token::Bracket(inside) = token {
            let range = inside.split(':').next().unwrap_or_default();
            if let Some(secs) = parse_duration_secs(range) {
                window = window.max(secs);
            }
        }
    }
    // `offset 30d` : le nombre et l'unité sont deux morceaux pour le découpage,
    // ils se relisent dans le texte.
    let lower = query.to_ascii_lowercase();
    let mut search = 0;
    while let Some(found) = lower[search..].find("offset") {
        let after = search + found + "offset".len();
        let tail = query[after..].trim_start();
        let duration: String =
            tail.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '.').collect();
        if let Some(secs) = parse_duration_secs(&duration) {
            offset = offset.max(secs);
        }
        search = after;
    }
    (window + offset).ceil() as i64
}

// --------------------------------------------------------------------------
// Réécriture
// --------------------------------------------------------------------------

/// Une requête réécrite pour l'instance long terme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rewritten {
    pub query: String,
    /// Faux si la requête désigne des séries par une regex sur `__name__` :
    /// l'historique long terme n'en a peut-être qu'une partie.
    pub all_known: bool,
}

/// Réécrit une requête pour qu'elle porte sur les agrégats.
///
/// `None` si une métrique n'est pas conservée, ou si une fonction ne peut pas
/// se recomposer à partir des agrégats (`count_over_time` d'une jauge, par
/// exemple) : la lecture reste alors sur l'instance brute.
pub fn rewrite(query: &str) -> Option<Rewritten> {
    let tokens = tokenize(query);
    // Pile des parenthèses ouvertes : la fonction qui les précède, s'il y en a.
    let mut stack: Vec<Option<(&str, usize)>> = Vec::new();
    // Remplacements (position, longueur, texte), appliqués de la fin au début.
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut all_known = true;
    let mut previous: Option<&Token<'_>> = None;

    for (index, token) in tokens.iter().enumerate() {
        match token {
            Token::Open => {
                let function = match previous {
                    Some(Token::Ident { text, at }) => Some((*text, *at)),
                    _ => None,
                };
                stack.push(function);
            }
            Token::Close => {
                stack.pop();
            }
            Token::Brace => {
                // Un sélecteur sans nom (`{__name__=~"…"}`) : on ne sait pas
                // ce qu'il désigne.
                if !matches!(previous, Some(Token::Ident { .. })) {
                    all_known = false;
                }
            }
            Token::Ident { text, at } if text.starts_with(super::METRIC_PREFIX) => {
                // Un appel de fonction n'est pas une métrique.
                if matches!(tokens.get(index + 1), Some(Token::Open)) {
                    previous = Some(token);
                    continue;
                }
                let kind = kind_of(text)?;
                let function = stack.last().copied().flatten();
                let (function_name, function_at) = match function {
                    Some((name, at)) => (name.to_ascii_lowercase(), Some(at)),
                    None => (String::new(), None),
                };
                let suffix = match (function_name.as_str(), kind) {
                    ("count_over_time", Kind::Availability) => {
                        if let Some(at) = function_at {
                            edits.push((at, "count_over_time".len(), "sum_over_time".into()));
                        }
                        ":5m_count"
                    }
                    ("sum_over_time", Kind::Availability) => ":5m_sum",
                    ("count_over_time" | "sum_over_time", _) => return None,
                    ("min_over_time", Kind::Availability | Kind::Gauge) => ":5m_min",
                    ("max_over_time", Kind::Availability | Kind::Gauge) => ":5m_max",
                    // Les variantes de compte ne se recomposent pas.
                    ("count_values_over_time" | "distinct_over_time" | "changes" | "resets", _) => {
                        return None;
                    }
                    _ => "",
                };
                if !suffix.is_empty() {
                    edits.push((at + text.len(), 0, suffix.to_string()));
                }
            }
            _ => {}
        }
        previous = Some(token);
    }

    let mut out = query.to_string();
    edits.sort_by_key(|edit| std::cmp::Reverse(edit.0));
    for (at, len, text) in edits {
        out.replace_range(at..at + len, &text);
    }
    Some(Rewritten { query: out, all_known })
}

// --------------------------------------------------------------------------
// Bascule
// --------------------------------------------------------------------------

/// Marge sous l'horizon brut : la donnée qui va être effacée n'est pas sûre.
pub const HORIZON_MARGIN_SECS: i64 = 3_600;
/// Tolérance sur la couverture des agrégats : ils doivent remonter au moins
/// jusqu'à l'horizon brut (à un jour près) pour être préférés.
pub const COVERAGE_SLACK_SECS: i64 = 86_400;

/// Ce que l'aiguillage sait de l'instant présent.
#[derive(Debug, Clone, Copy)]
pub struct Horizon {
    pub now_secs: i64,
    /// Rétention brute, en secondes.
    pub raw_retention_secs: i64,
    /// Plus ancienne fin de fenêtre agrégée ; `None` tant qu'il n'y en a pas.
    pub coverage_start_secs: Option<i64>,
}

impl Horizon {
    /// Plus ancien instant encore sûrement présent en brut.
    pub fn raw_start(&self) -> i64 {
        self.now_secs - self.raw_retention_secs + HORIZON_MARGIN_SECS
    }

    /// Les agrégats couvrent-ils au moins ce que couvre le brut ?
    fn long_covers(&self) -> bool {
        self.coverage_start_secs
            .is_some_and(|start| start <= self.raw_start() + COVERAGE_SLACK_SECS)
    }

    /// Une lecture qui remonte jusqu'à `oldest_secs` doit-elle aller aux agrégats ?
    pub fn wants_long(&self, oldest_secs: i64) -> bool {
        oldest_secs < self.raw_start() && self.long_covers()
    }
}

/// Plan d'une lecture sur plage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangePlan {
    /// Tout en brut.
    Raw,
    /// Tout sur les agrégats.
    Long { query: String },
    /// Agrégats de `start` à `split - step`, brut de `split` à `end`.
    Split { query: String, split_secs: i64 },
}

/// Où lire une requête instantanée évaluée maintenant.
pub fn plan_instant(query: &str, horizon: &Horizon) -> Option<String> {
    let oldest = horizon.now_secs - lookback_secs(query);
    if !horizon.wants_long(oldest) {
        return None;
    }
    let rewritten = rewrite(query)?;
    // Sans couture possible, une réponse partielle des agrégats serait pire que
    // la réponse partielle du brut.
    rewritten.all_known.then_some(rewritten.query)
}

/// Où lire une requête sur plage (`start`, `end` en secondes).
pub fn plan_range(query: &str, start: i64, end: i64, step: i64, horizon: &Horizon) -> RangePlan {
    let lookback = lookback_secs(query);
    if !horizon.wants_long(start - lookback) {
        return RangePlan::Raw;
    }
    let Some(rewritten) = rewrite(query) else { return RangePlan::Raw };
    let step = step.max(1);
    // Premier point de la grille dont toute la fenêtre est sûrement en brut.
    let safe = horizon.raw_start() + lookback;
    let offset = (safe - start).max(0);
    let split = start + offset.div_euclid(step) * step + if offset % step == 0 { 0 } else { step };
    if split > end {
        return RangePlan::Long { query: rewritten.query };
    }
    RangePlan::Split { query: rewritten.query, split_secs: split }
}

/// Recoud deux réponses : les points anciens des agrégats, puis les points
/// récents du brut, série par série (mêmes étiquettes).
pub fn stitch(
    old: Vec<super::RangeSeries>,
    recent: Vec<super::RangeSeries>,
) -> Vec<super::RangeSeries> {
    let mut out: Vec<super::RangeSeries> = Vec::with_capacity(old.len().max(recent.len()));
    let mut index: BTreeMap<BTreeMap<String, String>, usize> = BTreeMap::new();
    for series in old {
        index.insert(series.metric.clone(), out.len());
        out.push(series);
    }
    for series in recent {
        match index.get(&series.metric) {
            Some(&position) => {
                let target = &mut out[position];
                let last = target.values.last().map(|(ts, _)| *ts);
                target.values.extend(
                    series.values.into_iter().filter(|(ts, _)| last.is_none_or(|l| *ts > l)),
                );
            }
            None => {
                index.insert(series.metric.clone(), out.len());
                out.push(series);
            }
        }
    }
    out
}

/// Échantillons à écrire dans l'instance long terme, à partir de la réponse
/// d'une requête d'agrégation.
pub fn to_samples(series: &[super::RangeSeries], suffix: &str) -> Vec<dumbmonit_proto::Sample> {
    let mut out = Vec::new();
    for item in series {
        let Some(name) =
            item.metric.get("__name__").and_then(|name| name.strip_prefix(super::METRIC_PREFIX))
        else {
            continue;
        };
        let metric = format!("{name}{suffix}");
        for (ts, value) in &item.values {
            let Ok(value) = value.parse::<f64>() else { continue };
            if !value.is_finite() {
                continue;
            }
            let mut sample = dumbmonit_proto::Sample::new(
                metric.clone(),
                value,
                dumbmonit_proto::MetricKind::Gauge,
                (*ts * 1000.0).round() as i64,
            );
            for (label, label_value) in &item.metric {
                if label != "__name__" {
                    sample.labels.insert(label.clone(), label_value.clone());
                }
            }
            out.push(sample);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsdb::RangeSeries;

    fn series(labels: &[(&str, &str)], values: &[(f64, &str)]) -> RangeSeries {
        RangeSeries {
            metric: labels.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            values: values.iter().map(|(t, v)| (*t, v.to_string())).collect(),
        }
    }

    #[test]
    fn durations_follow_metricsql() {
        assert_eq!(parse_duration_secs("5m"), Some(300.0));
        assert_eq!(parse_duration_secs("1h30m"), Some(5_400.0));
        assert_eq!(parse_duration_secs("90d"), Some(7_776_000.0));
        assert_eq!(parse_duration_secs("1.5h"), Some(5_400.0));
        assert_eq!(parse_duration_secs("300"), Some(300.0));
        assert_eq!(parse_duration_secs("500ms"), Some(0.5));
        assert_eq!(parse_duration_secs("5i"), None);
        assert_eq!(parse_duration_secs(""), None);
    }

    #[test]
    fn lookback_is_the_widest_window_plus_the_offset() {
        assert_eq!(lookback_secs("dumbmonit_up"), 0);
        assert_eq!(lookback_secs("avg_over_time(dumbmonit_up[30d])"), 30 * 86_400);
        // Sous-requête : la plage avant le deux-points.
        assert_eq!(
            lookback_secs(
                "avg_over_time(((sum by (target) (count_over_time(dumbmonit_up{target=~\"1|2\"}[5m])) > bool 0) default 0)[90d:5m])"
            ),
            90 * 86_400
        );
        assert_eq!(
            lookback_secs("sum_over_time(dumbmonit_probe_success[7d] offset 7d)"),
            14 * 86_400
        );
        // Une regex entre guillemets n'est pas une fenêtre.
        assert_eq!(lookback_secs("dumbmonit_up{ifname=~\"eth[0-9]\"}"), 0);
    }

    #[test]
    fn availability_is_recomposed_from_sums_and_counts() {
        let q = "sum by (target) (sum_over_time(dumbmonit_probe_success{target=~\"1|2\"}[90d])) / sum by (target) (count_over_time(dumbmonit_probe_success{target=~\"1|2\"}[90d])) * 100";
        let rewritten = rewrite(q).unwrap();
        assert_eq!(
            rewritten.query,
            "sum by (target) (sum_over_time(dumbmonit_probe_success:5m_sum{target=~\"1|2\"}[90d])) / sum by (target) (sum_over_time(dumbmonit_probe_success:5m_count{target=~\"1|2\"}[90d])) * 100"
        );
        assert!(rewritten.all_known);
    }

    #[test]
    fn presence_slots_keep_their_five_minute_windows() {
        let q = "avg_over_time(((sum by (target) (count_over_time(dumbmonit_up{target=~\"3\"}[5m])) > bool 0) default 0)[90d:5m]) * 100";
        assert_eq!(
            rewrite(q).unwrap().query,
            "avg_over_time(((sum by (target) (sum_over_time(dumbmonit_up:5m_count{target=~\"3\"}[5m])) > bool 0) default 0)[90d:5m]) * 100"
        );
    }

    #[test]
    fn extremes_read_their_own_series_and_the_rest_reads_the_average() {
        assert_eq!(
            rewrite("min_over_time(dumbmonit_probe_success{target=\"4\"}[3600s])").unwrap().query,
            "min_over_time(dumbmonit_probe_success:5m_min{target=\"4\"}[3600s])"
        );
        assert_eq!(
            rewrite("max_over_time(dumbmonit_cpu_usage_percent[1d])").unwrap().query,
            "max_over_time(dumbmonit_cpu_usage_percent:5m_max[1d])"
        );
        assert_eq!(
            rewrite("avg_over_time(dumbmonit_probe_duration_seconds{target=\"4\"}[30d]) * 1000")
                .unwrap()
                .query,
            "avg_over_time(dumbmonit_probe_duration_seconds{target=\"4\"}[30d]) * 1000"
        );
        // Les compteurs gardent leur dernière valeur : `rate` reste tel quel.
        assert_eq!(
            rewrite("rate(dumbmonit_if_octets_in{target=\"9\"}[1h])").unwrap().query,
            "rate(dumbmonit_if_octets_in{target=\"9\"}[1h])"
        );
    }

    #[test]
    fn what_cannot_be_recomposed_stays_raw() {
        // Métrique non conservée.
        assert_eq!(rewrite("avg_over_time(dumbmonit_nut_battery_charge[30d])"), None);
        // Compte de mesures d'une jauge : n'a pas d'équivalent en agrégats.
        assert_eq!(rewrite("count_over_time(dumbmonit_cpu_usage_percent[30d])"), None);
        assert_eq!(rewrite("sum_over_time(dumbmonit_if_octets_in[30d])"), None);
        // Une regex sur le nom : réécrite, mais signalée.
        let regex = rewrite("rate({__name__=~\"dumbmonit_if_octets_(in|out)\"}[1h])").unwrap();
        assert!(!regex.all_known);
        // Un nom d'étiquette ou une valeur ne sont jamais réécrits.
        assert_eq!(
            rewrite("dumbmonit_up{job=\"dumbmonit_up\"}").unwrap().query,
            "dumbmonit_up{job=\"dumbmonit_up\"}"
        );
    }

    fn horizon(retention_days: i64, coverage: Option<i64>) -> Horizon {
        Horizon {
            now_secs: 1_000 * 86_400,
            raw_retention_secs: retention_days * 86_400,
            coverage_start_secs: coverage,
        }
    }

    #[test]
    fn instant_reads_switch_only_past_the_raw_retention() {
        let q = "sum by (target) (sum_over_time(dumbmonit_up[90d])) / sum by (target) (count_over_time(dumbmonit_up[90d]))";
        // Rétention brute d'un an : tout reste en brut.
        assert_eq!(plan_instant(q, &horizon(365, Some(0))), None);
        // Trente jours : les agrégats répondent.
        assert!(plan_instant(q, &horizon(30, Some(0))).unwrap().contains(":5m_count"));
        // Agrégats encore trop courts (rattrapage en cours) : le brut est meilleur.
        assert_eq!(plan_instant(q, &horizon(30, Some(990 * 86_400))), None);
        // Fenêtre courte : brut.
        assert_eq!(plan_instant("avg_over_time(dumbmonit_up[1h])", &horizon(30, Some(0))), None);
    }

    #[test]
    fn range_reads_are_split_at_the_raw_horizon() {
        let h = horizon(30, Some(0));
        let now = h.now_secs;
        let q = "dumbmonit_cpu_usage_percent{target=\"1\"}";
        // Sept jours : brut.
        assert_eq!(plan_range(q, now - 7 * 86_400, now, 3_600, &h), RangePlan::Raw);
        // Quatre-vingt-dix jours : agrégats puis brut, coupés sur la grille.
        let start = now - 90 * 86_400;
        match plan_range(q, start, now, 3_600, &h) {
            RangePlan::Split { query, split_secs } => {
                assert_eq!(query, q);
                assert_eq!((split_secs - start) % 3_600, 0);
                assert!(split_secs >= h.raw_start());
                assert!(split_secs < h.raw_start() + 3_600);
            }
            other => panic!("{other:?}"),
        }
        // Plage entièrement plus ancienne que le brut : agrégats seuls.
        assert!(matches!(plan_range(q, start, start + 86_400, 3_600, &h), RangePlan::Long { .. }));
        // Métrique non conservée : brut, même au-delà.
        assert_eq!(
            plan_range("dumbmonit_nut_load{target=\"1\"}", start, now, 3_600, &h),
            RangePlan::Raw
        );
    }

    #[test]
    fn stitching_appends_recent_points_without_duplicates() {
        let old = vec![series(&[("target", "1")], &[(100.0, "1"), (200.0, "2")])];
        let recent = vec![
            series(&[("target", "1")], &[(200.0, "9"), (300.0, "3")]),
            series(&[("target", "2")], &[(300.0, "5")]),
        ];
        let merged = stitch(old, recent);
        assert_eq!(merged.len(), 2);
        assert_eq!(
            merged[0].values,
            vec![(100.0, "1".into()), (200.0, "2".into()), (300.0, "3".into())]
        );
        assert_eq!(merged[1].values, vec![(300.0, "5".into())]);
    }

    #[test]
    fn windows_are_aligned_on_five_minutes() {
        assert_eq!(last_complete_window(1_000_000, 60), 999_900);
        assert_eq!(align_down(599), 300);
        assert_eq!(align_down(600), 600);
    }

    #[test]
    fn aggregate_queries_cover_every_kind() {
        let queries = aggregate_queries();
        assert_eq!(queries.len(), 6);
        let avg = &queries[0];
        assert_eq!(avg.suffix, "");
        assert!(avg.query.starts_with("avg_over_time({__name__=~\"dumbmonit_(up|probe_success|"));
        assert!(avg.query.ends_with("[5m]) keep_metric_names"));
        assert!(!avg.query.contains("if_octets_in"), "les compteurs n'ont pas de moyenne");
        let count = queries.iter().find(|q| q.suffix == ":5m_count").unwrap();
        assert_eq!(
            count.query,
            "count_over_time({__name__=~\"dumbmonit_(up|probe_success)\"}[5m]) keep_metric_names"
        );
        let last = queries.iter().find(|q| q.query.starts_with("last_over_time")).unwrap();
        assert!(last.query.contains("if_octets_in") && !last.query.contains("|up|"));
    }

    /// Disponibilité exacte : la somme des sommes sur la somme des comptes
    /// redonne la part des sondes réussies, même quand les fenêtres n'ont pas
    /// le même nombre de mesures (ce qu'une moyenne de moyennes fausserait).
    #[test]
    fn availability_from_aggregates_is_exact() {
        // Trois fenêtres : 10 mesures dont 10 réussies, 2 dont 0, 10 dont 5.
        let windows = [(10.0, 10.0), (0.0, 2.0), (5.0, 10.0)];
        let sum: f64 = windows.iter().map(|(s, _)| s).sum();
        let count: f64 = windows.iter().map(|(_, c)| c).sum();
        assert_eq!(sum / count, 15.0 / 22.0);
        let mean_of_means: f64 = windows.iter().map(|(s, c)| s / c).sum::<f64>() / 3.0;
        assert!((mean_of_means - sum / count).abs() > 0.01);
    }

    #[test]
    fn aggregated_series_keep_their_labels_under_the_new_name() {
        let answer = vec![series(
            &[("__name__", "dumbmonit_probe_success"), ("target", "7")],
            &[(1_800.0, "0.75"), (2_100.0, "NaN")],
        )];
        let samples = to_samples(&answer, ":5m_min");
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].metric, "probe_success:5m_min");
        assert_eq!(samples[0].ts_ms, 1_800_000);
        assert_eq!(samples[0].value, 0.75);
        assert_eq!(samples[0].labels.get("target").map(String::as_str), Some("7"));
        assert!(!samples[0].labels.contains_key("__name__"));
    }
}
