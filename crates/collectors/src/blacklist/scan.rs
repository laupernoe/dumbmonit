//! Le résultat d'un passage, et les séries qu'il produit.
//!
//! Un passage est gardé en mémoire et réécrit à chaque sonde jusqu'au suivant
//! (`refresh_hours`) : les séries restent continues pour les règles d'alerte,
//! sans réinterroger les listes toutes les minutes.

use std::collections::BTreeSet;
use std::net::IpAddr;
use std::time::Duration;

use dumbmonit_proto::{MetricKind, Sample};

use super::check::{Outcome, Verdict};
use super::lists::{List, Subject, Target};

/// Une adresse vérifiée par le moniteur de serveur mail, et d'où elle vient.
#[derive(Debug, Clone)]
pub struct Address {
    pub ip: IpAddr,
    /// `input`, `mx`, `domain` ou `extra`.
    pub source: &'static str,
    /// L'hôte MX qui l'a donnée, le cas échéant.
    pub host: Option<String>,
    /// Le nom de son DNS inverse, `None` si elle n'en a pas ; `Some("")` jamais.
    pub ptr: Option<String>,
    /// Le DNS inverse renvoie à un nom qui se résout vers cette adresse
    /// (FCrDNS) ; `None` quand la question est restée sans réponse.
    pub fcrdns: Option<bool>,
}

/// Les enregistrements d'authentification d'un domaine d'envoi.
#[derive(Debug, Clone, Default)]
pub struct MailAuth {
    pub mx_hosts: usize,
    /// Enregistrements `v=spf1` : un seul est valide (RFC 7208, section 4.5).
    pub spf_records: Option<usize>,
    pub dmarc_records: Option<usize>,
    /// `p=` de l'enregistrement DMARC.
    pub dmarc_policy: Option<String>,
}

impl MailAuth {
    /// SPF absent ou en double, DMARC absent : ce que Gmail et Yahoo exigent.
    pub fn missing(&self) -> Option<usize> {
        let spf = self.spf_records? != 1;
        let dmarc = self.dmarc_records? == 0;
        Some(usize::from(spf) + usize::from(dmarc))
    }
}

#[derive(Debug, Clone)]
pub struct Scan {
    pub addresses_checked: usize,
    pub outcomes: Vec<Outcome>,
    /// Moniteur de serveur mail : les adresses et leur DNS inverse.
    pub addresses: Vec<Address>,
    pub domain: Option<String>,
    pub auth: Option<MailAuth>,
    pub lists: Vec<&'static List>,
    pub duration: Duration,
    /// Écrire aussi les vérifications sans histoire (une adresse de serveur) ;
    /// pour une plage, seules les inscriptions et les inconnues sont écrites.
    pub every_check: bool,
}

impl Scan {
    fn listed_lists(&self) -> BTreeSet<&'static str> {
        self.outcomes.iter().filter(|o| o.verdict.is_listed()).map(|o| o.list.id).collect()
    }

    /// Les séries du passage, datées `ts_ms`.
    pub fn samples(&self, age: Duration, running: bool, ts_ms: i64) -> Vec<Sample> {
        let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);
        let count = |n: usize| n as f64;
        let mut out = Vec::new();

        for list in &self.lists {
            let mine: Vec<&Outcome> =
                self.outcomes.iter().filter(|o| o.list.id == list.id).collect();
            let applicable = mine.iter().filter(|o| o.verdict != Verdict::NotApplicable).count();
            let labelled = |sample: Sample| sample.with_label("list", list.id);
            // Ce que la page montre d'une liste, y compris une liste personnalisée
            // que le catalogue ne connaît pas.
            out.push(labelled(
                gauge("blacklist_list_info", 1.0)
                    .with_label("name", list.name)
                    .with_label("zone", list.zone)
                    .with_label(
                        "subject",
                        if list.subject == Subject::Ip { "ip" } else { "domain" },
                    )
                    .with_label("major", if list.major { "true" } else { "false" }),
            ));
            out.push(labelled(gauge("blacklist_list_checked", count(applicable))));
            out.push(labelled(gauge(
                "blacklist_list_listed",
                count(mine.iter().filter(|o| o.verdict.is_listed()).count()),
            )));
            out.push(labelled(gauge(
                "blacklist_list_unknown",
                count(mine.iter().filter(|o| o.verdict.is_unknown()).count()),
            )));
        }

        for outcome in &self.outcomes {
            let quiet = matches!(outcome.verdict, Verdict::Clean | Verdict::NotApplicable);
            if quiet && !self.every_check {
                continue;
            }
            let mut sample = gauge("blacklist_check", outcome.verdict.value())
                .with_label("list", outcome.list.id)
                .with_label("subject", outcome.subject.to_string())
                .with_label("state", outcome.verdict.state());
            match &outcome.verdict {
                Verdict::Listed { codes, meaning, reason } => {
                    let codes: Vec<String> = codes.iter().map(|c| c.to_string()).collect();
                    sample =
                        sample.with_label("code", codes.join(",")).with_label("meaning", meaning);
                    if let Some(reason) = reason {
                        sample = sample.with_label("reason", reason);
                    }
                }
                Verdict::Refused(why) | Verdict::Unknown(why) => {
                    sample = sample.with_label("reason", why);
                }
                Verdict::Clean | Verdict::NotApplicable => {}
            }
            out.push(sample);
        }

        let listed_lists = self.listed_lists();
        let major = listed_lists
            .iter()
            .filter(|id| self.lists.iter().any(|l| l.id == **id && l.major))
            .count();
        let listed_ips: BTreeSet<String> = self
            .outcomes
            .iter()
            .filter(|o| o.list.subject == Subject::Ip && o.verdict.is_listed())
            .map(|o| o.subject.to_string())
            .collect();
        out.push(gauge("blacklist_addresses_checked", count(self.addresses_checked)));
        out.push(gauge("blacklist_addresses_listed", count(listed_ips.len())));
        out.push(gauge("blacklist_listed_lists", count(listed_lists.len())));
        out.push(gauge("blacklist_listed_major_lists", count(major)));
        out.push(gauge(
            "blacklist_unknown_checks",
            count(self.outcomes.iter().filter(|o| o.verdict.is_unknown()).count()),
        ));
        if let Some(domain) = &self.domain {
            let listed = self.outcomes.iter().any(|o| {
                o.verdict.is_listed() && matches!(&o.subject, Target::Domain(d) if d == domain)
            });
            out.push(gauge("blacklist_domain_listed", if listed { 1.0 } else { 0.0 }));
        }

        for address in &self.addresses {
            let mut info = gauge("blacklist_address_info", 1.0)
                .with_label("address", address.ip.to_string())
                .with_label("source", address.source);
            if let Some(host) = &address.host {
                info = info.with_label("mx", host);
            }
            out.push(info);
            if let Some(fcrdns) = address.fcrdns {
                out.push(
                    gauge("blacklist_ptr_ok", if fcrdns { 1.0 } else { 0.0 })
                        .with_label("address", address.ip.to_string())
                        .with_label("ptr", address.ptr.clone().unwrap_or_default()),
                );
            }
        }

        if let Some(auth) = &self.auth {
            out.push(gauge("blacklist_mx_hosts", count(auth.mx_hosts)));
            if let Some(n) = auth.spf_records {
                out.push(gauge("blacklist_spf_records", count(n)));
            }
            if let Some(n) = auth.dmarc_records {
                out.push(gauge("blacklist_dmarc_records", count(n)));
            }
            if let Some(policy) = &auth.dmarc_policy {
                out.push(gauge("blacklist_dmarc_policy_info", 1.0).with_label("policy", policy));
            }
            if let Some(missing) = auth.missing() {
                out.push(gauge("blacklist_mail_auth_missing", count(missing)));
            }
        }

        out.push(gauge("blacklist_scan_age_seconds", age.as_secs_f64()));
        out.push(gauge("blacklist_scan_duration_seconds", self.duration.as_secs_f64()));
        out.push(gauge("blacklist_scan_running", if running { 1.0 } else { 0.0 }));
        out
    }
}
