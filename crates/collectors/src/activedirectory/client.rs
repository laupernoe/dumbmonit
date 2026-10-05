//! Le dialogue LDAP avec le contrôleur, et rien d'autre.
//!
//! Chaque fonction rend les structures simples de `model.rs` ; aucune décision
//! n'est prise ici. Les recherches volumineuses sont paginées (contrôle « Paged
//! Results », 500 entrées par page) : AD plafonne une réponse à 1 000 entrées
//! par défaut.
//!
//! Aucun attribut secret n'est jamais demandé : ni mot de passe LAPS
//! (`ms-Mcs-AdmPwd`, `msLAPS-Password`), seulement leurs dates d'expiration, ni
//! rien de ce que seule la réplication complète (DCSync) donnerait.

use std::time::{Duration, Instant};

use dumbmonit_proto::ProbeError;
use ldap3::adapters::{Adapter, EntriesOnly, PagedResults};
use ldap3::{Ldap, LdapConnAsync, LdapConnSettings, LdapError, Scope, SearchEntry, ldap_escape};
use tokio::task::JoinHandle;

use super::analysis::Reach;
use super::decode;
use super::model::{
    Account, DcComputer, DomainObject, DsaRecord, Entry, FsmoRecord, GroupRecord, Inventory,
    Member, NTDSDSA_OPT_IS_GC, PRIVILEGED_GROUPS, RootDse,
};
use super::options::{Options, Security};
use super::tls;

/// Taille des pages des recherches paginées.
const PAGE_SIZE: i32 = 500;

/// Règle d'appartenance récursive d'AD (LDAP_MATCHING_RULE_IN_CHAIN) : suit
/// l'imbrication des groupes côté serveur.
const IN_CHAIN: &str = "1.2.840.113556.1.4.1941";

/// Test de bit d'AD (LDAP_MATCHING_RULE_BIT_AND).
const BIT_AND: &str = "1.2.840.113556.1.4.803";

/// Une connexion ouverte au contrôleur.
pub struct Session {
    ldap: Ldap,
    driver: JoinHandle<()>,
    timeout: Duration,
}

/// Résultat de la liaison du compte de service.
pub enum Bind {
    Accepted,
    /// Refusée par le contrôleur, avec la raison en clair.
    Refused(String),
}

/// Ouvre la connexion (TCP, puis TLS ou StartTLS). Rend la durée écoulée.
pub async fn connect(options: &Options) -> Result<(Session, f64), ProbeError> {
    let mut settings = LdapConnSettings::new().set_conn_timeout(options.request_timeout);
    if options.security != Security::Plain {
        settings = settings
            .set_config(tls::client_config(options)?)
            .set_starttls(options.security == Security::StartTls)
            .set_no_tls_verify(options.insecure_tls);
    }
    let url = options.url();
    let started = Instant::now();
    let (conn, ldap) =
        tokio::time::timeout(options.request_timeout, LdapConnAsync::with_settings(settings, &url))
            .await
            .map_err(|_| ProbeError::Timeout(options.request_timeout))?
            .map_err(|error| connect_error(error, options))?;
    let elapsed = started.elapsed().as_secs_f64();
    let driver = tokio::spawn(async move {
        // La connexion s'arrête d'elle-même à la fermeture ; une erreur ici ne
        // concerne qu'elle et remonte déjà par l'opération en cours.
        let _ = conn.drive().await;
    });
    Ok((Session { ldap, driver, timeout: options.request_timeout }, elapsed))
}

/// Classe une erreur de connexion : injoignable (alerte) ou problème de
/// certificat (à corriger dans la configuration).
fn connect_error(error: LdapError, options: &Options) -> ProbeError {
    let text = error.to_string();
    let lower = text.to_ascii_lowercase();
    if lower.contains("certificate")
        || lower.contains("unknownissuer")
        || matches!(error, LdapError::Rustls { .. })
    {
        return ProbeError::Config(format!(
            "TLS certificate of {} rejected ({text}). Paste the certificate of the authority \
             that issued it as CA certificate, or tick Accept an unverifiable certificate.",
            options.host
        ));
    }
    match error {
        LdapError::Timeout { .. } => ProbeError::Timeout(options.request_timeout),
        LdapError::Io { source } => match source.kind() {
            std::io::ErrorKind::TimedOut => ProbeError::Timeout(options.request_timeout),
            std::io::ErrorKind::InvalidData => {
                ProbeError::Protocol(format!("TLS failed with {}: {source}", options.host))
            }
            _ => ProbeError::Unreachable(format!("{}:{}: {source}", options.host, options.port)),
        },
        LdapError::LdapResult { result } => ProbeError::Protocol(format!(
            "the domain controller refused StartTLS (LDAP result {}: {})",
            result.rc,
            result.text.trim()
        )),
        other => ProbeError::Unreachable(format!("{}:{}: {other}", options.host, options.port)),
    }
}

/// Erreur d'une opération après la connexion.
fn op_error(context: &str, error: LdapError, timeout: Duration) -> ProbeError {
    match error {
        LdapError::Timeout { .. } => ProbeError::Timeout(timeout),
        LdapError::Io { source } => ProbeError::Unreachable(format!("{context}: {source}")),
        LdapError::LdapResult { result } => ProbeError::Protocol(format!(
            "{context}: LDAP result {} ({})",
            result.rc,
            result.text.trim().trim_end_matches('\0')
        )),
        other => ProbeError::Protocol(format!("{context}: {other}")),
    }
}

impl Session {
    /// Liaison simple du compte de service. Un refus n'est pas une erreur de
    /// sonde : le contrôleur répond, c'est le compte qui pose problème.
    pub async fn bind(&mut self, user: &str, password: &str) -> Result<Bind, ProbeError> {
        let result = self
            .ldap
            .with_timeout(self.timeout)
            .simple_bind(user, password)
            .await
            .map_err(|error| op_error("bind", error, self.timeout))?;
        if result.rc == 0 {
            Ok(Bind::Accepted)
        } else {
            Ok(Bind::Refused(decode::explain_bind_failure(result.rc, &result.text)))
        }
    }

    /// Lit un seul objet. `None` s'il n'existe pas.
    pub async fn base(&mut self, dn: &str, attrs: &[&str]) -> Result<Option<Entry>, ProbeError> {
        let result = self
            .ldap
            .with_timeout(self.timeout)
            .search(dn, Scope::Base, "(objectClass=*)", attrs.to_vec())
            .await
            .map_err(|error| op_error(dn_context(dn), error, self.timeout))?;
        // 32 : noSuchObject.
        if result.1.rc == 32 {
            return Ok(None);
        }
        let (entries, _) =
            result.success().map_err(|error| op_error(dn_context(dn), error, self.timeout))?;
        Ok(entries.into_iter().next().map(to_entry))
    }

    /// Recherche paginée sous `base`.
    pub async fn search(
        &mut self,
        base: &str,
        scope: Scope,
        filter: &str,
        attrs: &[&str],
    ) -> Result<Vec<Entry>, ProbeError> {
        let attrs: Vec<String> = attrs.iter().map(|a| (*a).to_string()).collect();
        let timeout = self.timeout;
        let work = async {
            let adapters: Vec<Box<dyn Adapter<'_, String, Vec<String>>>> =
                vec![Box::new(EntriesOnly::new()), Box::new(PagedResults::new(PAGE_SIZE))];
            let mut stream =
                self.ldap.streaming_search_with(adapters, base, scope, filter, attrs).await?;
            let mut entries = Vec::new();
            while let Some(entry) = stream.next().await? {
                entries.push(to_entry(entry));
            }
            stream.finish().await.success()?;
            Ok::<_, LdapError>(entries)
        };
        // Toute la recherche, toutes pages comprises, tient dans un délai : une
        // page qui ne vient pas ne doit pas bloquer la sonde.
        let budget = timeout.saturating_mul(4);
        tokio::time::timeout(budget, work)
            .await
            .map_err(|_| ProbeError::Timeout(budget))?
            .map_err(|error| op_error(filter, error, timeout))
    }

    pub async fn close(mut self) {
        let _ = tokio::time::timeout(Duration::from_secs(2), self.ldap.unbind()).await;
        self.driver.abort();
    }
}

fn dn_context(dn: &str) -> &str {
    if dn.is_empty() { "RootDSE" } else { dn }
}

fn to_entry(raw: ldap3::ResultEntry) -> Entry {
    let entry = SearchEntry::construct(raw);
    Entry::new(entry.dn, entry.attrs, entry.bin_attrs)
}

// --------------------------------------------------------------------------
// Lectures
// --------------------------------------------------------------------------

pub async fn root_dse(session: &mut Session) -> Result<RootDse, ProbeError> {
    let entry = session
        .base(
            "",
            &[
                "defaultNamingContext",
                "configurationNamingContext",
                "schemaNamingContext",
                "rootDomainNamingContext",
                "dnsHostName",
                "dsServiceName",
                "domainFunctionality",
                "forestFunctionality",
                "domainControllerFunctionality",
                "isSynchronized",
                "isGlobalCatalogReady",
                "currentTime",
                // Attribut construit : lisible avec le droit « Monitor Active
                // Directory Replication », absent sinon (sans erreur).
                "msDS-ReplAllInboundNeighbors",
            ],
        )
        .await?
        .ok_or_else(|| ProbeError::Protocol("the RootDSE is empty".to_string()))?;
    let root = RootDse::from_entry(&entry);
    if root.default_nc.is_empty() {
        return Err(ProbeError::Protocol(
            "this server publishes no defaultNamingContext: it is not an Active Directory \
             domain controller"
                .to_string(),
        ));
    }
    Ok(root)
}

pub async fn domain_object(
    session: &mut Session,
    root: &RootDse,
) -> Result<DomainObject, ProbeError> {
    let entry = session
        .base(
            &root.default_nc,
            &[
                "objectSid",
                "minPwdLength",
                "pwdHistoryLength",
                "maxPwdAge",
                "minPwdAge",
                "lockoutThreshold",
                "lockoutDuration",
                "lockOutObservationWindow",
                "pwdProperties",
                "ms-DS-MachineAccountQuota",
                "fSMORoleOwner",
            ],
        )
        .await?
        .ok_or_else(|| ProbeError::Protocol("the domain object cannot be read".to_string()))?;
    Ok(DomainObject::from_entry(&entry))
}

/// Les cinq rôles FSMO : deux de forêt, trois de domaine.
pub async fn fsmo(
    session: &mut Session,
    root: &RootDse,
    domain: &DomainObject,
) -> Result<Vec<FsmoRecord>, ProbeError> {
    let mut records = Vec::new();
    for (role, dn) in [
        ("schema", root.schema_nc.clone()),
        ("domain_naming", format!("CN=Partitions,{}", root.config_nc)),
        ("rid", format!("CN=RID Manager$,CN=System,{}", root.default_nc)),
        ("infrastructure", format!("CN=Infrastructure,{}", root.default_nc)),
    ] {
        let holder = session
            .base(&dn, &["fSMORoleOwner"])
            .await?
            .and_then(|e| e.first("fSMORoleOwner").map(str::to_string));
        records.push(FsmoRecord { role, holder_dsa: holder });
    }
    records.insert(2, FsmoRecord { role: "pdc", holder_dsa: domain.pdc_owner.clone() });
    Ok(records)
}

/// Les contrôleurs de ce domaine, lus dans la partition de configuration.
pub async fn domain_controllers(
    session: &mut Session,
    root: &RootDse,
) -> Result<Vec<DsaRecord>, ProbeError> {
    let entries = session
        .search(
            &format!("CN=Sites,{}", root.config_nc),
            Scope::Subtree,
            "(|(objectClass=server)(objectClass=nTDSDSA)(objectClass=nTDSConnection))",
            &[
                "objectClass",
                "dNSHostName",
                "options",
                "msDS-isRODC",
                "msDS-hasDomainNCs",
                "hasMasterNCs",
                "msDS-hasFullReplicaNCs",
            ],
        )
        .await?;
    Ok(assemble_dsas(&entries, &root.default_nc))
}

/// Range serveurs, DSA et connexions en une ligne par contrôleur du domaine.
pub fn assemble_dsas(entries: &[Entry], default_nc: &str) -> Vec<DsaRecord> {
    let class_of = |entry: &Entry, class: &str| {
        entry.all("objectClass").iter().any(|c| c.eq_ignore_ascii_case(class))
    };
    let parent_of =
        |dn: &str| dn.split_once(',').map(|(_, rest)| rest.to_string()).unwrap_or_default();
    let mut dsas = Vec::new();
    for dsa in entries.iter().filter(|e| class_of(e, "nTDSDSA")) {
        let domains = dsa.all("msDS-hasDomainNCs");
        // Un RODC porte sa partition de domaine dans `msDS-hasFullReplicaNCs`.
        let masters: Vec<&String> =
            dsa.all("hasMasterNCs").iter().chain(dsa.all("msDS-hasFullReplicaNCs")).collect();
        let ours = if domains.is_empty() {
            masters.is_empty() || masters.iter().any(|nc| decode::same_dn(nc, default_nc))
        } else {
            domains.iter().any(|nc| decode::same_dn(nc, default_nc))
        };
        if !ours {
            continue;
        }
        let server_dn = parent_of(&dsa.dn);
        let server = entries.iter().find(|e| decode::same_dn(&e.dn, &server_dn));
        let (name, site) = decode::server_and_site_of_dsa(&dsa.dn).unwrap_or_default();
        dsas.push(DsaRecord {
            dsa_dn: dsa.dn.clone(),
            server: name,
            site,
            dns_host_name: server.and_then(|s| s.first("dNSHostName").map(str::to_string)),
            global_catalog: dsa.int("options").is_some_and(|o| o & NTDSDSA_OPT_IS_GC != 0),
            read_only: dsa.bool("msDS-isRODC").unwrap_or(false),
            inbound_connections: entries
                .iter()
                .filter(|e| {
                    class_of(e, "nTDSConnection") && decode::same_dn(&parent_of(&e.dn), &dsa.dn)
                })
                .count(),
        });
    }
    dsas
}

/// Les objets ordinateurs des contrôleurs, pour leur système.
pub async fn dc_computers(
    session: &mut Session,
    root: &RootDse,
) -> Result<Vec<DcComputer>, ProbeError> {
    let filter = format!(
        "(&(objectCategory=computer)(|(userAccountControl:{BIT_AND}:=8192)(userAccountControl:{BIT_AND}:=67108864)))"
    );
    let entries = session
        .search(
            &root.default_nc,
            Scope::Subtree,
            &filter,
            &["sAMAccountName", "dNSHostName", "operatingSystem", "operatingSystemVersion"],
        )
        .await?;
    Ok(entries
        .iter()
        .map(|e| DcComputer {
            dns_host_name: e.first("dNSHostName").map(str::to_string),
            name: e.first("sAMAccountName").unwrap_or_default().to_string(),
            operating_system: e.first("operatingSystem").map(str::to_string),
            os_version: e.first("operatingSystemVersion").map(str::to_string),
        })
        .collect())
}

/// La corbeille AD est activée quand sa fonctionnalité optionnelle est liée à
/// une partition.
pub async fn recycle_bin(
    session: &mut Session,
    root: &RootDse,
) -> Result<Option<bool>, ProbeError> {
    let dn = format!(
        "CN=Recycle Bin Feature,CN=Optional Features,CN=Directory Service,CN=Windows NT,CN=Services,{}",
        root.config_nc
    );
    Ok(session.base(&dn, &["msDS-EnabledFeatureBL"]).await?.map(|e| e.has("msDS-EnabledFeatureBL")))
}

/// Les groupes privilégiés et leurs membres effectifs.
pub async fn privileged_groups(
    session: &mut Session,
    root: &RootDse,
    domain: &DomainObject,
) -> Result<Vec<GroupRecord>, ProbeError> {
    let Some(domain_sid) = domain.sid.as_deref() else {
        return Err(ProbeError::Protocol("the domain SID cannot be read".to_string()));
    };
    let forest_root = decode::same_dn(&root.default_nc, &root.root_nc);
    let wanted: Vec<_> =
        PRIVILEGED_GROUPS.iter().filter(|def| forest_root || !def.forest_root_only).collect();
    let filter = format!(
        "(&(objectCategory=group)(|{}))",
        wanted.iter().map(|def| format!("(objectSid={})", def.sid(domain_sid))).collect::<String>()
    );
    let found = session
        .search(&root.default_nc, Scope::Subtree, &filter, &["objectSid", "sAMAccountName"])
        .await?;
    let mut groups = Vec::new();
    for entry in found {
        let Some(sid) = entry.sid() else { continue };
        let Some(def) = wanted.iter().find(|def| def.sid(domain_sid) == sid) else { continue };
        let members = session
            .search(
                &root.default_nc,
                Scope::Subtree,
                &format!("(memberOf:{IN_CHAIN}:={})", ldap_escape(&entry.dn)),
                &["sAMAccountName", "objectClass", "userAccountControl"],
            )
            .await?;
        groups.push(GroupRecord {
            def: **def,
            dn: entry.dn.clone(),
            members: members.iter().map(Member::from_entry).collect(),
        });
    }
    Ok(groups)
}

/// L'inventaire complet : comptes, ordinateurs, nombre de groupes, schéma LAPS.
pub async fn inventory(
    session: &mut Session,
    root: &RootDse,
    now: i64,
) -> Result<Inventory, ProbeError> {
    let users = session
        .search(
            &root.default_nc,
            Scope::Subtree,
            "(&(objectCategory=person)(objectClass=user))",
            &[
                "sAMAccountName",
                "objectSid",
                "userAccountControl",
                "msDS-User-Account-Control-Computed",
                "lastLogonTimestamp",
                "pwdLastSet",
                "whenCreated",
                "lockoutTime",
                "adminCount",
                "servicePrincipalName",
            ],
        )
        .await?;
    let computers = session
        .search(
            &root.default_nc,
            Scope::Subtree,
            "(objectCategory=computer)",
            &[
                "sAMAccountName",
                "userAccountControl",
                "lastLogonTimestamp",
                "pwdLastSet",
                "whenCreated",
                "operatingSystem",
                "dNSHostName",
                // Dates d'expiration seulement : les mots de passe eux-mêmes ne
                // sont jamais demandés.
                "ms-Mcs-AdmPwdExpirationTime",
                "msLAPS-PasswordExpirationTime",
            ],
        )
        .await?;
    // « 1.1 » : aucun attribut, seulement les DN.
    let groups = session
        .search(&root.default_nc, Scope::Subtree, "(objectCategory=group)", &["1.1"])
        .await?;
    let schema = session
        .search(
            &root.schema_nc,
            Scope::OneLevel,
            "(|(lDAPDisplayName=ms-Mcs-AdmPwdExpirationTime)(lDAPDisplayName=msLAPS-PasswordExpirationTime))",
            &["lDAPDisplayName"],
        )
        .await?;
    let in_schema = |name: &str| {
        schema
            .iter()
            .any(|e| e.first("lDAPDisplayName").is_some_and(|n| n.eq_ignore_ascii_case(name)))
    };
    Ok(Inventory {
        refreshed_at: now,
        users: users.iter().map(Account::from_entry).collect(),
        computers: computers.iter().map(Account::from_entry).collect(),
        groups_total: groups.len() as u64,
        laps_legacy_schema: in_schema("ms-Mcs-AdmPwdExpirationTime"),
        laps_windows_schema: in_schema("msLAPS-PasswordExpirationTime"),
    })
}

// --------------------------------------------------------------------------
// Joignabilité des autres contrôleurs
// --------------------------------------------------------------------------

/// Ouvre une connexion TCP vers le port LDAP de chaque contrôleur, en
/// parallèle. Un nom qui ne se résout pas depuis DumbMonit (serveur DNS hors
/// du domaine) n'est pas compté comme une panne.
pub async fn check_reachability(
    dsas: &[DsaRecord],
    skip: Option<&str>,
    port: u16,
    timeout: Duration,
) -> Vec<(String, Reach)> {
    let checks = dsas
        .iter()
        .filter(|dsa| skip.is_none_or(|queried| !decode::same_dn(queried, &dsa.dsa_dn)))
        .map(|dsa| async move {
            let reach = match &dsa.dns_host_name {
                None => Reach::NotChecked,
                Some(host) => reach(host, port, timeout).await,
            };
            (dsa.dsa_dn.clone(), reach)
        });
    futures::future::join_all(checks).await
}

async fn reach(host: &str, port: u16, timeout: Duration) -> Reach {
    let addresses = match tokio::time::timeout(timeout, tokio::net::lookup_host((host, port))).await
    {
        Err(_) => return Reach::Unresolved(format!("{host} did not resolve in time")),
        Ok(Err(error)) => return Reach::Unresolved(format!("{host} does not resolve: {error}")),
        Ok(Ok(addresses)) => addresses.collect::<Vec<_>>(),
    };
    let Some(address) = addresses.first() else {
        return Reach::Unresolved(format!("{host} resolves to no address"));
    };
    match tokio::time::timeout(timeout, tokio::net::TcpStream::connect(address)).await {
        Err(_) => Reach::Unreachable(format!(
            "{host}:{port} did not answer within {}s",
            timeout.as_secs()
        )),
        Ok(Err(error)) => Reach::Unreachable(format!("{host}:{port}: {error}")),
        Ok(Ok(_)) => Reach::Reachable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn entry(dn: &str, attrs: &[(&str, &[&str])]) -> Entry {
        Entry::new(
            dn.to_string(),
            attrs
                .iter()
                .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
                .collect(),
            HashMap::new(),
        )
    }

    #[test]
    fn la_configuration_donne_un_controleur_par_dsa_du_domaine() {
        let cfg = "CN=Sites,CN=Configuration,DC=corp,DC=lan";
        let entries = vec![
            entry(
                &format!("CN=DC1,CN=Servers,CN=Paris,{cfg}"),
                &[("objectClass", &["top", "server"]), ("dNSHostName", &["dc1.corp.lan"])],
            ),
            entry(
                &format!("CN=NTDS Settings,CN=DC1,CN=Servers,CN=Paris,{cfg}"),
                &[
                    ("objectClass", &["top", "applicationSettings", "nTDSDSA"]),
                    ("options", &["1"]),
                    ("msDS-hasDomainNCs", &["DC=corp,DC=lan"]),
                ],
            ),
            entry(
                &format!("CN=abc,CN=NTDS Settings,CN=DC1,CN=Servers,CN=Paris,{cfg}"),
                &[("objectClass", &["top", "nTDSConnection"])],
            ),
            entry(
                &format!("CN=RODC,CN=Servers,CN=Lyon,{cfg}"),
                &[("objectClass", &["server"]), ("dNSHostName", &["rodc.corp.lan"])],
            ),
            entry(
                &format!("CN=NTDS Settings,CN=RODC,CN=Servers,CN=Lyon,{cfg}"),
                &[
                    ("objectClass", &["nTDSDSA"]),
                    ("msDS-isRODC", &["TRUE"]),
                    ("msDS-hasDomainNCs", &["DC=corp,DC=lan"]),
                ],
            ),
            // Un contrôleur d'un autre domaine de la forêt : écarté.
            entry(
                &format!("CN=NTDS Settings,CN=CHILD1,CN=Servers,CN=Paris,{cfg}"),
                &[
                    ("objectClass", &["nTDSDSA"]),
                    ("msDS-hasDomainNCs", &["DC=child,DC=corp,DC=lan"]),
                ],
            ),
            // Un serveur sans DSA (métadonnées d'un contrôleur rétrogradé) : écarté.
            entry(&format!("CN=OLD,CN=Servers,CN=Paris,{cfg}"), &[("objectClass", &["server"])]),
        ];
        let dsas = assemble_dsas(&entries, "DC=corp,DC=lan");
        assert_eq!(dsas.len(), 2);
        assert_eq!(dsas[0].server, "DC1");
        assert_eq!(dsas[0].site, "Paris");
        assert_eq!(dsas[0].dns_host_name.as_deref(), Some("dc1.corp.lan"));
        assert!(dsas[0].global_catalog);
        assert_eq!(dsas[0].inbound_connections, 1);
        assert!(dsas[1].read_only);
        assert!(!dsas[1].global_catalog);
    }

    #[tokio::test]
    async fn un_nom_qui_ne_se_resout_pas_n_est_pas_une_panne() {
        let dsa = DsaRecord {
            dsa_dn:
                "CN=NTDS Settings,CN=X,CN=Servers,CN=S,CN=Sites,CN=Configuration,DC=corp,DC=lan"
                    .into(),
            dns_host_name: Some("nexiste-pas.invalid".into()),
            ..DsaRecord::default()
        };
        let result = check_reachability(&[dsa], None, 636, Duration::from_secs(2)).await;
        assert!(matches!(result[0].1, Reach::Unresolved(_)), "{:?}", result[0].1);
    }

    #[tokio::test]
    async fn un_port_ferme_est_injoignable() {
        // Un port local tout juste libéré : la connexion est refusée.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        assert!(matches!(
            reach("127.0.0.1", port, Duration::from_secs(2)).await,
            Reach::Unreachable(_)
        ));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        assert_eq!(reach("127.0.0.1", port, Duration::from_secs(2)).await, Reach::Reachable);
    }
}
