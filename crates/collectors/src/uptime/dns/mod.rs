//! Sonde de résolution DNS (`kind = "dns"`).
//!
//! Vérifie qu'un nom se résout, en combien de temps, et — c'est là tout l'intérêt —
//! **vers quoi**. Une zone détournée, un enregistrement effacé par une mauvaise
//! manipulation ou un basculement de bascule oublié se voient ici, alors qu'une
//! sonde HTTP les manquerait : elle suivrait la mauvaise adresse sans broncher.
//!
//! # Adresse et étiquettes
//!
//! Adresse : le nom à résoudre, `www.exemple.fr`.
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `record_type` | `A` | `A`, `AAAA`, `CNAME`, `MX`, `TXT`, `NS`, `SOA`, `SRV`, `PTR`, `CAA`. |
//! | `resolver` | système | Adresse IP du résolveur, port facultatif (`1.1.1.1`, `10.0.0.1:5353`). |
//! | `expect` | — | Valeurs devant toutes figurer dans la réponse, séparées par des virgules. |
//! | `alert_on_change` | `false` | Publie `probe_dns_answer_fingerprint`, l'empreinte de la réponse. |
//! | `timeout_seconds` | `5` | Délai propre à la sonde (1 à 60). |

mod answer;
pub(crate) mod options;

use std::time::Instant;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};
use hickory_resolver::config::{ConnectionConfig, NameServerConfig, ResolveHosts, ResolverConfig};
use hickory_resolver::net::NetError;
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::{Resolver, TokioResolver};
use tracing::debug;

use super::outcome::{Failure, Report};
use options::Options;

/// Collecteur de disponibilité par résolution de nom.
#[derive(Default)]
pub struct DnsCollector;

impl DnsCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Collector for DnsCollector {
    fn kind(&self) -> &'static str {
        "dns"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = Options::from_target(target)?;
        let resolver = build_resolver(&options)?;

        let mut report = Report::new(self.kind())
            .label("record_type", options.record_type.to_string())
            .label("resolver", options.resolver_label());

        resolve(&mut report, &resolver, &options).await;

        if let Some(detail) = report.detail() {
            debug!(target_id = target.id, name = %options.name, detail, "sonde DNS en échec");
        }
        Ok(report.finish())
    }
}

/// Interroge le résolveur et confronte la réponse aux attentes.
async fn resolve(report: &mut Report, resolver: &TokioResolver, options: &Options) {
    let started = Instant::now();
    let lookup = match resolver.lookup(options.name.as_str(), options.record_type).await {
        Ok(lookup) => lookup,
        Err(error) => {
            let (reason, detail) = classify(&error);
            report.fail(reason, detail);
            return;
        }
    };
    report.gauge("dns_lookup_seconds", started.elapsed().as_secs_f64());

    let records = lookup.answers();
    let matching = answer::count_of_type(records, options.record_type);
    report.gauge("dns_answer_records", matching as f64);

    // Une réponse sans le moindre enregistrement du type demandé est un `NOERROR`
    // vide : le nom existe, mais pas pour ce type. Le résolveur ne le signale pas
    // comme une erreur, et pourtant le service qui en dépend ne marchera pas.
    if answer::is_empty_answer(records, options.record_type) {
        report.fail(
            Failure::Record,
            format!("no {} record for \"{}\"", options.record_type, options.name),
        );
        return;
    }

    let answers = answer::render(records);

    // Posée avant les attentes : c'est justement quand la réponse change que
    // celles-ci échouent, et l'empreinte doit alors être écrite.
    if options.alert_on_change {
        report.gauge("dns_answer_fingerprint", f64::from(answer::fingerprint(&answers)));
    }

    if options.expect.is_empty() && options.forbid.is_empty() {
        return;
    }

    let forbidden = answer::forbidden_present(&answers, &options.forbid);
    if !forbidden.is_empty() {
        report.fail(
            Failure::Record,
            format!(
                "forbidden values present in the answer: {} (got: {})",
                forbidden.join(", "),
                answers.join(", ")
            ),
        );
        return;
    }

    if options.expect.is_empty() {
        return;
    }

    let missing = answer::missing(&answers, &options.expect);
    if !missing.is_empty() {
        report.fail(
            Failure::Record,
            format!(
                "expected values missing from the answer: {} (got: {})",
                missing.join(", "),
                answers.join(", ")
            ),
        );
        return;
    }

    // En mode exact, un enregistrement de plus est aussi grave qu'un de moins :
    // c'est ainsi qu'une zone détournée se signale sans rien effacer.
    if options.expect_mode == answer::Mode::Exact {
        let unexpected = answer::unexpected(&answers, &options.expect);
        if !unexpected.is_empty() {
            report.fail(
                Failure::Record,
                format!(
                    "the answer holds values that were not expected: {} (expected exactly: {})",
                    unexpected.join(", "),
                    options.expect.join(", ")
                ),
            );
        }
    }
}

/// Construit un résolveur dédié à cette interrogation.
///
/// Il est reconstruit à chaque passage, et son cache est désactivé : un moniteur
/// doit poser la question au réseau à chaque fois. Un résolveur mutualisé
/// répondrait depuis son cache et mesurerait la latence de sa propre mémoire,
/// masquant aussi bien une panne du serveur DNS qu'un changement d'enregistrement.
fn build_resolver(options: &Options) -> Result<TokioResolver, ProbeError> {
    let mut builder = match options.resolver {
        Some(address) => {
            let mut udp = ConnectionConfig::udp();
            udp.port = address.port();
            let mut tcp = ConnectionConfig::tcp();
            tcp.port = address.port();
            let server = NameServerConfig::new(address.ip(), true, vec![udp, tcp]);
            Resolver::builder_with_config(
                ResolverConfig::from_parts(None, Vec::new(), vec![server]),
                TokioRuntimeProvider::default(),
            )
        }
        None => Resolver::builder_tokio().map_err(|error| {
            ProbeError::Config(format!(
                "no usable system resolver ({error}): set the one to query with the \
                 \"resolver\" tag, for example \"1.1.1.1\""
            ))
        })?,
    };

    {
        let opts = builder.options_mut();
        opts.cache_size = 0;
        // Une seule tentative : les reprises internes masqueraient une perte de
        // paquets qui est justement ce que l'on cherche à mesurer, et feraient
        // dépasser le délai de la sonde.
        opts.attempts = 1;
        opts.timeout = options.timeout;
        // `/etc/hosts` court-circuiterait la requête et la sonde ne testerait plus
        // le serveur DNS mais un fichier local.
        opts.use_hosts_file = ResolveHosts::Never;
        // Les `CNAME` intermédiaires sont conservés : ils font partie de ce que
        // l'utilisateur veut pouvoir attendre avec `expect`.
        opts.preserve_intermediates = true;
        opts.num_concurrent_reqs = 1;
    }

    builder.build().map_err(|error| ProbeError::Config(format!("unusable resolver: {error}")))
}

/// Traduit l'erreur du résolveur en raison exposée en métrique.
///
/// La distinction utile n'est pas technique mais opérationnelle : « le serveur DNS
/// ne répond pas » se corrige sur le serveur DNS, « le nom n'existe pas » se
/// corrige dans la zone.
fn classify(error: &NetError) -> (Failure, String) {
    let detail = error.to_string();
    match error {
        NetError::Timeout => (Failure::Timeout, "the resolver did not answer in time".to_string()),
        NetError::Io(_) | NetError::NoConnections | NetError::Busy => {
            (Failure::Connect, format!("resolver unreachable: {detail}"))
        }
        NetError::Dns(hickory_resolver::net::DnsError::NoRecordsFound(_)) => {
            (Failure::Record, detail)
        }
        _ => (Failure::Dns, detail),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::tags::test_support::cible;

    #[test]
    fn le_collecteur_annonce_son_type() {
        assert_eq!(DnsCollector::new().kind(), "dns");
    }

    #[test]
    fn un_resolveur_explicite_donne_un_resolveur_construit_sans_reseau() {
        let options =
            Options::from_target(&cible("dns", "exemple.fr", &[("resolver", "9.9.9.9:5353")]))
                .unwrap();
        assert!(build_resolver(&options).is_ok(), "aucune socket n'est ouverte à la construction");
    }

    #[test]
    fn un_type_denregistrement_inconnu_est_refuse_avant_toute_requete() {
        let collector = DnsCollector::new();
        let error = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(collector.probe(&cible("dns", "exemple.fr", &[("record_type", "PIZZA")])))
            .unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
        assert!(!error.means_down(), "une faute de frappe n'est pas une panne de service");
    }

    /// Les trois assertions de zone, vérifiées sur des réponses fabriquées :
    /// aucune requête DNS n'est émise, seul le verdict est en jeu.
    #[test]
    fn les_assertions_de_zone_distinguent_labsence_de_lajout_et_de_linterdit() {
        use answer::fixtures::a;

        let attendu = "203.0.113.10";
        let mut options =
            Options::from_target(&cible("dns", "exemple.fr", &[("expect", attendu)])).unwrap();

        // Un enregistrement ajouté à côté du bon : invisible par défaut.
        let detourne = [a("exemple.fr.", [203, 0, 113, 10]), a("exemple.fr.", [198, 51, 100, 7])];
        let answers = answer::render(&detourne);
        assert!(answer::missing(&answers, &options.expect).is_empty());

        options.expect_mode = answer::Mode::Exact;
        assert_eq!(answer::unexpected(&answers, &options.expect), vec!["198.51.100.7"]);

        options.forbid = vec!["198.51.100.7".to_string()];
        assert_eq!(answer::forbidden_present(&answers, &options.forbid), vec!["198.51.100.7"]);
    }

    /// Un faux serveur DNS (UDP) dont la réponse change en cours de route :
    /// l'empreinte suit l'ensemble des valeurs, pas leur ordre.
    #[tokio::test]
    async fn l_empreinte_suit_un_changement_de_reponse_d_un_vrai_serveur() {
        use std::net::Ipv4Addr;
        use std::sync::{Arc, Mutex};

        use hickory_resolver::proto::op::{Message, OpCode};
        use hickory_resolver::proto::rr::rdata::A;
        use hickory_resolver::proto::rr::{RData, Record};

        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = socket.local_addr().unwrap().to_string();
        let answers =
            Arc::new(Mutex::new(vec![Ipv4Addr::new(192, 0, 2, 1), Ipv4Addr::new(192, 0, 2, 2)]));
        let served = answers.clone();
        tokio::spawn(async move {
            let mut buffer = [0u8; 512];
            while let Ok((length, peer)) = socket.recv_from(&mut buffer).await {
                let Ok(query) = Message::from_vec(&buffer[..length]) else { continue };
                let mut reply = Message::response(query.metadata.id, OpCode::Query);
                reply.metadata.recursion_desired = query.metadata.recursion_desired;
                reply.metadata.recursion_available = true;
                for question in &query.queries {
                    reply.add_query(question.clone());
                    let ips = served.lock().unwrap().clone();
                    for ip in ips {
                        let name = question.name().clone();
                        reply.add_answer(Record::from_rdata(name, 60, RData::A(A(ip))));
                    }
                }
                let _ = socket.send_to(&reply.to_vec().unwrap(), peer).await;
            }
        });

        let tags = [("resolver", address.as_str()), ("alert_on_change", "true")];
        let target = cible("dns", "www.exemple.test", &tags);
        let probe = || async {
            let samples = DnsCollector::new().probe(&target).await.unwrap();
            let success = samples.iter().find(|s| s.metric == "probe_success").unwrap().value;
            assert_eq!(success, 1.0);
            samples.iter().find(|s| s.metric == "probe_dns_answer_fingerprint").map(|s| s.value)
        };
        let first = probe().await.expect("empreinte publiée");
        answers.lock().unwrap().reverse();
        assert_eq!(probe().await, Some(first), "le tourniquet ne change pas l'empreinte");
        answers.lock().unwrap()[1] = Ipv4Addr::new(198, 51, 100, 7);
        assert_ne!(probe().await, Some(first), "une adresse remplacée la change");

        let target = cible("dns", "www.exemple.test", &[("resolver", address.as_str())]);
        let samples = DnsCollector::new().probe(&target).await.unwrap();
        assert!(!samples.iter().any(|s| s.metric == "probe_dns_answer_fingerprint"));
    }

    #[test]
    fn un_resolveur_muet_et_un_nom_absent_ne_se_confondent_pas() {
        assert_eq!(classify(&NetError::Timeout).0, Failure::Timeout);
        assert_eq!(classify(&NetError::NoConnections).0, Failure::Connect);
        assert_eq!(classify(&NetError::Msg("réponse tronquée".into())).0, Failure::Dns);
    }
}
