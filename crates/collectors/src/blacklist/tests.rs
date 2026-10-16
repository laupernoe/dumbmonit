//! Les deux moniteurs de bout en bout, contre un faux résolveur.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use dumbmonit_proto::{Collector, ProbeError, Sample};

use super::dns::fake::FakeDns;
use super::dns::{Dns, Qtype};
use super::options::Settings;
use super::{DnsFactory, IpBlacklistCollector, MailBlacklistCollector};
use crate::uptime::tags::test_support::cible;

fn factory(dns: &Arc<FakeDns>) -> DnsFactory {
    let dns = dns.clone();
    Arc::new(move |_: &Settings| Ok(dns.clone() as Arc<dyn Dns>))
}

fn find<'a>(samples: &'a [Sample], name: &str, labels: &[(&str, &str)]) -> Option<&'a Sample> {
    samples.iter().find(|s| {
        s.metric == name
            && labels.iter().all(|(k, v)| s.labels.get(*k).map(String::as_str) == Some(*v))
    })
}

fn value(samples: &[Sample], name: &str, labels: &[(&str, &str)]) -> f64 {
    find(samples, name, labels).unwrap_or_else(|| panic!("{name} {labels:?} absent")).value
}

const FAST: (&str, &str) = ("queries_per_second", "50");

/// Un domaine à deux adresses de MX (dont une IPv6) et une adresse propre.
fn mail_zone() -> Arc<FakeDns> {
    let dns = Arc::new(FakeDns::default());
    dns.answer("example.com.", Qtype::Mx, &["10 mx1.example.net."])
        .answer("mx1.example.net.", Qtype::A, &["8.8.8.1"])
        .answer("mx1.example.net.", Qtype::Aaaa, &["2a00:1450::5"])
        .answer("example.com.", Qtype::A, &["8.8.8.2"])
        .answer("example.com.", Qtype::Txt, &["v=spf1 mx -all", "google-site-verification=x"])
        .answer("_dmarc.example.com.", Qtype::Txt, &["v=DMARC1; p=reject"])
        .answer("1.8.8.8.in-addr.arpa.", Qtype::Ptr, &["mx1.example.net"])
        // SpamCop liste le MX, avec son motif.
        .answer("1.8.8.8.bl.spamcop.net.", Qtype::A, &["127.0.0.2"])
        .answer(
            "1.8.8.8.bl.spamcop.net.",
            Qtype::Txt,
            &["Blocked - see https://www.spamcop.net/bl.shtml?8.8.8.1"],
        );
    dns
}

#[tokio::test]
async fn un_domaine_avec_mx_est_verifie_adresse_par_adresse() {
    let dns = mail_zone();
    let collector = MailBlacklistCollector::with_dns(factory(&dns));
    let target = cible("mail_blacklist", "example.com", &[FAST]);
    let samples = collector.probe(&target).await.unwrap();

    assert_eq!(value(&samples, "blacklist_addresses_checked", &[]), 3.0);
    assert_eq!(value(&samples, "blacklist_addresses_listed", &[]), 1.0);
    assert_eq!(value(&samples, "blacklist_listed_major_lists", &[]), 1.0);
    assert_eq!(value(&samples, "blacklist_domain_listed", &[]), 0.0);
    let listed =
        find(&samples, "blacklist_check", &[("list", "spamcop"), ("subject", "8.8.8.1")]).unwrap();
    assert_eq!(listed.value, 1.0);
    assert_eq!(listed.labels["state"], "listed");
    assert_eq!(listed.labels["code"], "127.0.0.2");
    assert!(listed.labels["reason"].starts_with("Blocked"));
    let clean =
        find(&samples, "blacklist_check", &[("list", "spamhaus_zen"), ("subject", "8.8.8.2")]);
    assert_eq!(clean.unwrap().value, 0.0, "NXDOMAIN = absent");
    // L'IPv6 du MX : Spamhaus seul la couvre.
    let v6 = [("subject", "2a00:1450::5")];
    assert_eq!(value(&samples, "blacklist_check", &[v6[0], ("list", "barracuda")]), -3.0);
    assert_eq!(value(&samples, "blacklist_check", &[v6[0], ("list", "spamhaus_zen")]), 0.0);
    assert_eq!(value(&samples, "blacklist_list_listed", &[("list", "spamcop")]), 1.0);
    assert_eq!(value(&samples, "blacklist_list_checked", &[("list", "spamcop")]), 2.0);

    // Ce que regardent Gmail et Outlook.
    assert_eq!(
        value(&samples, "blacklist_ptr_ok", &[("address", "8.8.8.1"), ("ptr", "mx1.example.net")]),
        1.0
    );
    assert_eq!(value(&samples, "blacklist_ptr_ok", &[("address", "8.8.8.2")]), 0.0);
    assert_eq!(value(&samples, "blacklist_spf_records", &[]), 1.0);
    assert_eq!(value(&samples, "blacklist_dmarc_records", &[]), 1.0);
    assert!(find(&samples, "blacklist_dmarc_policy_info", &[("policy", "reject")]).is_some());
    assert_eq!(value(&samples, "blacklist_mail_auth_missing", &[]), 0.0);
    assert!(
        find(
            &samples,
            "blacklist_address_info",
            &[("address", "8.8.8.1"), ("mx", "mx1.example.net")]
        )
        .is_some()
    );

    // Le passage est gardé : la sonde suivante ne repose aucune question.
    let asked = dns.queries.load(Ordering::SeqCst);
    let again = collector.probe(&target).await.unwrap();
    assert_eq!(dns.queries.load(Ordering::SeqCst), asked);
    assert_eq!(value(&again, "blacklist_scan_running", &[]), 0.0);
}

#[tokio::test]
async fn un_refus_de_spamhaus_est_une_erreur_de_configuration() {
    let dns = mail_zone();
    dns.answer("1.8.8.8.zen.spamhaus.org.", Qtype::A, &["127.255.255.254"]);
    let collector = MailBlacklistCollector::with_dns(factory(&dns));
    let error =
        collector.probe(&cible("mail_blacklist", "example.com", &[FAST])).await.unwrap_err();
    assert!(
        matches!(error, ProbeError::Config(ref m) if m.contains("public") && m.contains("DQS")),
        "{error}"
    );
    assert!(!error.means_down());
}

#[tokio::test]
async fn un_refus_d_uribl_laisse_la_liste_inconnue() {
    let dns = mail_zone();
    dns.answer("example.com.multi.uribl.com.", Qtype::A, &["127.0.0.1"]);
    let collector = MailBlacklistCollector::with_dns(factory(&dns));
    let samples = collector.probe(&cible("mail_blacklist", "example.com", &[FAST])).await.unwrap();
    let check = find(&samples, "blacklist_check", &[("list", "uribl")]).unwrap();
    assert_eq!(check.labels["state"], "refused");
    assert_eq!(
        value(&samples, "blacklist_domain_listed", &[]),
        0.0,
        "un refus n'est pas une inscription"
    );
    assert_eq!(value(&samples, "blacklist_list_unknown", &[("list", "uribl")]), 1.0);
}

#[tokio::test]
async fn un_delai_n_est_pas_un_non_liste() {
    let dns = mail_zone();
    *dns.timeout_suffix.lock().unwrap() = Some("bl.spamcop.net.".into());
    let collector = MailBlacklistCollector::with_dns(factory(&dns));
    let samples = collector.probe(&cible("mail_blacklist", "example.com", &[FAST])).await.unwrap();
    let check =
        find(&samples, "blacklist_check", &[("list", "spamcop"), ("subject", "8.8.8.1")]).unwrap();
    assert_eq!(check.labels["state"], "unknown");
    assert_eq!(check.value, -1.0);
    assert_eq!(value(&samples, "blacklist_listed_major_lists", &[]), 0.0);
    assert_eq!(value(&samples, "blacklist_list_unknown", &[("list", "spamcop")]), 2.0);

    // Plus rien ne répond : le résolveur est en cause, pas les listes.
    let dns = Arc::new(FakeDns::default());
    *dns.timeout_suffix.lock().unwrap() = Some(".".into());
    let collector = MailBlacklistCollector::with_dns(factory(&dns));
    let error = collector.probe(&cible("mail_blacklist", "8.8.4.4", &[FAST])).await.unwrap_err();
    assert!(error.means_down(), "{error}");
}

#[tokio::test]
async fn une_plage_ne_detaille_que_les_inscriptions() {
    let dns = Arc::new(FakeDns::default());
    dns.answer("2.8.8.8.zen.spamhaus.org.", Qtype::A, &["127.0.0.3"]);
    let collector = IpBlacklistCollector::with_dns(factory(&dns));
    let samples = collector.probe(&cible("ip_blacklist", "8.8.8.0/30", &[FAST])).await.unwrap();
    assert_eq!(value(&samples, "blacklist_addresses_checked", &[]), 4.0);
    assert_eq!(value(&samples, "blacklist_addresses_listed", &[]), 1.0);
    assert_eq!(value(&samples, "blacklist_list_listed", &[("list", "spamhaus_zen")]), 1.0);
    let checks: Vec<&Sample> = samples.iter().filter(|s| s.metric == "blacklist_check").collect();
    assert_eq!(checks.len(), 1, "les adresses propres ne font pas de série");
    assert_eq!(checks[0].labels["meaning"], "SBL CSS");
    assert!(find(&samples, "blacklist_list_checked", &[("list", "spamhaus_dbl")]).is_none());
    // 4 adresses × 7 listes d'adresses, plus le motif de l'inscription.
    assert_eq!(dns.queries.load(Ordering::SeqCst), 29);
}

#[tokio::test]
async fn une_plage_trop_grande_ou_ipv6_est_refusee_sans_question() {
    let dns = Arc::new(FakeDns::default());
    let collector = IpBlacklistCollector::with_dns(factory(&dns));
    for address in ["8.8.0.0/22", "2a00:1450::/64"] {
        let error = collector.probe(&cible("ip_blacklist", address, &[])).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{address}: {error}");
    }
    assert_eq!(dns.queries.load(Ordering::SeqCst), 0);
}
