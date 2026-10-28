mod activedirectory;
mod ad_deploy;
pub(crate) mod agent_commands;
mod agent_events;
pub(crate) mod agent_files;
mod agent_logs;
mod agent_tls;
mod agent_updates;
mod alert_actions;
pub(crate) mod alerts;
mod auth;
mod backends;
mod backup;
mod blacklist;
mod channels;
pub(crate) mod clients;
mod collectors;
mod cves;
mod dashboard_templates;
pub mod dashboards;
mod deadlines;
pub(crate) mod discovery;
mod error;
mod forecasts;
mod geocode;
mod gitops;
mod health;
mod ical;
mod ingest;
mod instance;
mod integrations;
mod kuma_db;
mod kuma_import;
mod label_monitors;
mod ldap;
pub mod locations;
mod logs;
mod luns;
mod mcp;
mod metrics;
mod monitor_imports;
mod netconfig;
mod netinventory;
mod notify_policy;
mod observability;
mod oidc;
mod onboarding;
mod oncall;
mod openapi;
mod opnsense;
mod opnsense_live;
mod otlp;
mod packs;
mod passkeys;
mod pbs;
mod pdm;
mod pmg;
mod prometheus;
mod proxies;
mod proxmox;
mod push;
mod redfish;
mod relay;
mod remediation;
mod reports;
mod saml;
mod scim;
mod security;
mod self_monitoring;
mod seo_files;
mod skipped_features;
pub(crate) mod spa;
pub(crate) mod status_pages;
pub use status_pages::delivery::spawn as spawn_status_deliveries;
mod storage;
mod summary;
mod synology;
mod target_rules;
pub(crate) mod targets;
mod targets_import;
mod teams;
mod threats;
mod tokens;
mod topology;
pub(crate) mod topology_edits;
mod topology_l2;
mod topology_traffic;
mod totp;
mod truenas;
mod update;
mod users;
mod webchange;
mod webpush;
mod website;

pub use collectors::option_keys as collector_option_keys;
pub use error::{ApiError, ApiResult};
pub use openapi::API_VERSION;

use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, Request};
use axum::http::{Extensions, HeaderMap, HeaderValue, StatusCode, Version, header};
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{delete, get, post, put};
use axum::{Extension, Router, middleware};
use tower_http::compression::predicate::{NotForContentType, Predicate, SizeAbove};
use tower_http::compression::{CompressionLayer, CompressionLevel};
use tower_http::trace::TraceLayer;

use crate::auth::AuthState;
use crate::auth::cors::CorsPolicy;
use crate::state::AppState;

/// Protocole des agents : lots, commandes, sondes relayées, sous le garde du
/// jeton d'enrôlement. Monté sur le port de l'interface (ancien canal) et sur
/// le canal chiffré des agents ([`crate::agent_tls`]).
pub fn agent_routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/ingest", post(ingest::receive))
        // Canal de commandes des agents : même jeton, même raison d'être ouvert.
        .merge(agent_commands::agent_routes())
        // Lignes de journal collectées par les agents : même jeton, même canal.
        .merge(agent_logs::agent_routes())
        // Sondes déléguées aux agents relais (`relay.rs`) : même canal, et un
        // compte rendu peut peser autant qu'un lot de mesures.
        .merge(relay::agent_routes())
        // Binaire proposé par la mise à jour automatique de l'agent.
        .merge(agent_updates::agent_routes())
        .layer(DefaultBodyLimit::max(16 * 1024 * 1024))
        .route_layer(middleware::from_fn_with_state(state.clone(), ingest::require_agent_token))
}

pub fn router(state: AppState) -> Router {
    let auth_state = AuthState::from_env(&state.config);

    // Tout ce qui touche à l'instance — lecture comprise : la liste des
    // équipements d'un homelab est déjà une information à ne pas laisser traîner.
    //
    // Le garde applique aussi les rôles : POST/PUT/DELETE exigent un
    // administrateur, sauf `logout` et `password` que chacun fait pour soi.
    let protected = Router::new()
        .route("/auth/logout", post(auth::logout))
        .route("/auth/password", post(auth::change_password))
        .route("/auth/me", get(auth::me))
        // Second facteur et journal d'audit (`totp.rs`).
        .merge(totp::routes())
        // Clés d'accès : liste, ajout, renommage, suppression (`passkeys.rs`).
        .merge(passkeys::routes())
        .route(
            "/auth/oidc/config",
            get(oidc::get_config).put(oidc::put_config).delete(oidc::delete_config),
        )
        .route("/auth/oidc/test", post(oidc::test))
        .route(
            "/auth/ldap/config",
            get(ldap::get_config).put(ldap::put_config).delete(ldap::delete_config),
        )
        .route("/auth/ldap/test", post(ldap::test))
        // Connexion SAML : réglages et lecture de métadonnées d'IdP (`saml.rs`).
        .route("/auth/saml/config", get(saml::get_config).put(saml::put_config))
        .route("/auth/saml/metadata/import", post(saml::import_metadata))
        // Provisionnement SCIM : réglages et jeton du fournisseur (`scim.rs`).
        .route("/auth/scim", get(scim::get_settings).put(scim::put_settings))
        .route("/auth/scim/token", post(scim::create_token).delete(scim::delete_token))
        .route("/users", get(users::list).post(users::create))
        .route("/users/{id}", put(users::update).delete(users::delete))
        .route("/collectors", get(collectors::list))
        // Compteurs plats pour Homepage / tableaux de bord tiers (`summary.rs`).
        .route("/summary", get(summary::get))
        .route("/discovery", post(discovery::scan))
        .route("/discovery/subnets", get(discovery::local_subnets))
        .route("/discovery/jobs", get(discovery::list_jobs).post(discovery::create_job))
        .route("/discovery/jobs/{id}", get(discovery::get_job).delete(discovery::cancel_job))
        // Scans planifiés et inventaire du réseau (`netinventory.rs`).
        .route("/discovery/schedules", get(netinventory::list).post(netinventory::create))
        .route("/discovery/schedules/{id}", put(netinventory::update).delete(netinventory::remove))
        .route("/discovery/schedules/{id}/run", post(netinventory::run_now))
        .route("/discovery/inventory", get(netinventory::inventory))
        .route("/discovery/inventory/known", post(netinventory::all_known))
        .route("/discovery/inventory/devices/{id}/known", post(netinventory::device_known))
        .route("/discovery/inventory/devices/{id}/ignore", post(netinventory::device_ignore))
        .route("/discovery/inventory/ports/{id}/known", post(netinventory::port_known))
        .route("/onboarding", get(onboarding::get).put(onboarding::put))
        .route("/targets", get(targets::list).post(targets::create))
        .route("/targets/test", post(targets::test_connection))
        .route("/targets/reorder", post(targets::reorder))
        .route("/targets/folders/rename", post(targets::rename_folder))
        .route("/targets/import", post(targets_import::import))
        .route(
            "/targets/import/kuma",
            post(kuma_import::import).layer(DefaultBodyLimit::max(kuma_import::MAX_BODY_BYTES)),
        )
        .route("/targets/import/kuma-db", post(kuma_import::import_db))
        .route(
            "/targets/import/uptimerobot",
            post(monitor_imports::uptimerobot)
                .layer(DefaultBodyLimit::max(monitor_imports::MAX_BODY_BYTES)),
        )
        .route(
            "/targets/import/gatus",
            post(monitor_imports::gatus)
                .layer(DefaultBodyLimit::max(monitor_imports::MAX_BODY_BYTES)),
        )
        .route("/targets/import/healthchecks", post(monitor_imports::healthchecks))
        .route("/targets/{id}", get(targets::get_one).put(targets::update).delete(targets::delete))
        .route("/targets/{id}/release", post(label_monitors::release))
        .route("/targets/{id}/probe", post(targets::probe_now))
        .route("/targets/{id}/probe-locations", get(targets::probe_locations))
        .route("/targets/{id}/discover", post(targets::discover))
        .route("/targets/{id}/rules", get(target_rules::list))
        .route("/metrics/query", get(metrics::query))
        .route("/metrics/query_range", get(metrics::query_range))
        .route("/metrics/catalog", get(metrics::catalog))
        .route("/alerts", get(alerts::list_active))
        .route("/alerts/history", get(alerts::history))
        .route("/alerts/history/dismiss-resolved", post(alerts::dismiss_resolved_history))
        .route("/alerts/history/{id}/dismiss", post(alerts::dismiss_history_entry))
        .route("/alerts/{fingerprint}/ack", post(alerts::ack_alert).delete(alerts::unack_alert))
        .route(
            "/alerts/{fingerprint}/dismiss",
            post(alerts::dismiss_alert).delete(alerts::undismiss_alert),
        )
        .route("/alerts/rules", get(alerts::list_rules).post(alerts::create_rule))
        .route("/alerts/rules/{id}", put(alerts::update_rule).delete(alerts::delete_rule))
        .route("/alerts/rules/{id}/enable", post(alerts::set_rule_enabled))
        .route("/alerts/silences", get(alerts::list_silences).post(alerts::create_silence))
        .route("/alerts/silences/{id}", delete(alerts::delete_silence))
        .route("/notify/channels", get(channels::list).post(channels::create))
        .route("/notify/channels/{id}", put(channels::update).delete(channels::delete))
        .route("/notify/channels/{id}/test", post(channels::test))
        .route("/notify/kinds", get(channels::kinds))
        .route("/agent/tokens", get(ingest::list_tokens).post(ingest::create_token))
        .route("/agent/tokens/{id}", delete(ingest::revoke_token))
        // Machines enrôlées par un jeton : le flux d'ajout attend la première.
        .route("/agent/tokens/{id}/machines", get(ingest::token_machines))
        // Déploiement par stratégie de groupe Active Directory (`ad_deploy.rs`).
        .route("/agent/tokens/{id}/close", post(ad_deploy::close_token))
        .route("/agent/ad-kit", post(ad_deploy::create_kit))
        // Paramètres de la commande d'installation : URL, canal chiffré, empreintes.
        .route("/agent/install", get(ingest::install_params))
        // Canal chiffré des agents : état, exigence de mTLS, rotation de l'autorité.
        .merge(agent_tls::routes())
        // Mise à jour automatique des agents : réglages, état, « Update now ».
        .merge(agent_updates::routes())
        .route("/tokens", get(tokens::list).post(tokens::create))
        .route("/tokens/{id}", delete(tokens::revoke))
        // Actions sur les conteneurs d'une machine (`agent_commands.rs`).
        .merge(agent_commands::ui_routes())
        // Journal d'événements Windows d'une machine : règles et événements (`agent_events.rs`).
        .merge(agent_events::routes())
        // Collecte de journaux d'une machine et état des entrées (`agent_logs.rs`).
        .merge(agent_logs::ui_routes())
        // Panneau Synology : vue d'ensemble et appareils Active Backup (`synology.rs`).
        .merge(synology::routes())
        // Prévisions de remplissage d'un équipement (`forecasts.rs`).
        .merge(forecasts::routes())
        // LUN iSCSI et zvols d'un NAS, et leur consommateur (`luns.rs`).
        .merge(luns::routes())
        // Agents relais proposés par le formulaire d'équipement (`relay.rs`).
        .merge(relay::ui_routes())
        // Carte de topologie, construite à partir des liens connus (`topology.rs`).
        .merge(topology::routes())
        // Corrections de la carte et règles apprises (`topology_edits.rs`).
        .merge(topology_edits::routes())
        // Emplacements et carte du monde (`locations.rs`).
        .merge(locations::routes())
        // Tableaux de bord personnalisés (`dashboards.rs`).
        .merge(dashboards::routes())
        // Tableau des invités d'un hyperviseur Proxmox VE (`proxmox.rs`).
        .merge(proxmox::routes())
        // Fonctionnalités facultatives sautées faute de droit (`skipped_features.rs`).
        .merge(skipped_features::routes())
        // Politique de notification et surcharges par équipement (`notify_policy.rs`).
        .merge(notify_policy::routes())
        // Astreintes : plannings, escalades, contacts et heures de chacun (`oncall.rs`).
        .merge(oncall::routes())
        // Rapports périodiques par courriel (`reports.rs`).
        .merge(reports::routes())
        // Pages de statut et incidents (`status_pages.rs`).
        .merge(status_pages::routes())
        // Jeton du calendrier iCal privé des maintenances (`ical.rs`).
        .merge(ical::routes())
        // Calendrier des sauvegardes et travaux d'un Proxmox Backup Server (`pbs.rs`).
        .merge(pbs::routes())
        // Instances fédérées et santé d'un Proxmox Datacenter Manager (`pdm.rs`).
        .merge(pdm::routes())
        // Files d'attente, filtrage et santé d'une Proxmox Mail Gateway (`pmg.rs`).
        .merge(pmg::routes())
        // Passerelles, trafic et santé d'un pare-feu OPNsense (`opnsense.rs`).
        .merge(opnsense::routes())
        // Carte en direct du pare-feu et base IP → pays (`opnsense_live.rs`).
        .merge(opnsense_live::routes())
        // Pools, protection des données et santé d'un NAS TrueNAS (`truenas.rs`).
        .merge(truenas::routes())
        // Domaine Active Directory : contrôleurs, groupes privilégiés, constats (`activedirectory.rs`).
        .merge(activedirectory::routes())
        // Matériel d'un serveur lu par son contrôleur de gestion (`redfish.rs`).
        .merge(redfish::routes())
        // Santé d'un serveur de journaux ou de métriques (`observability.rs`).
        .merge(observability::routes())
        // Santé de Redis, MongoDB, RabbitMQ et CrowdSec (`backends.rs`).
        .merge(backends::routes())
        // Proxys inverses et enregistrement d'un domaine (`proxies.rs`).
        .merge(proxies::routes())
        // Listes noires DNS : serveur mail et plage d'adresses (`blacklist.rs`).
        .merge(blacklist::routes())
        // Frise des échéances : domaines, certificats, secrets, licences (`deadlines.rs`).
        .merge(deadlines::routes())
        // Moniteurs en poussée : jeton d'une cible et sa régénération (`push.rs`).
        .merge(push::ui_routes())
        // Sauvegarde et restauration de l'instance (`backup.rs`).
        .merge(backup::routes())
        // Configuration en fichiers (GitOps) : plan, application, export (`gitops.rs`).
        .merge(gitops::routes())
        // Journaux du serveur : stockage, export et destinations (`logs.rs`).
        .merge(logs::routes())
        // Stockage : rétentions, historique long terme, VictoriaLogs embarqué
        // et recherche dans les journaux (`storage.rs`).
        .merge(storage::routes())
        // Auto-surveillance et battement de cœur sortant (`self_monitoring.rs`).
        .merge(self_monitoring::routes())
        // Paquets d'intégration : types d'équipement décrits en YAML (`packs.rs`).
        .merge(packs::routes())
        // Changements d'un site web : pages, diff et captures (`webchange.rs`).
        .merge(webchange::routes())
        // Sauvegarde des configurations réseau : versions, diff, clé d'hôte (`netconfig.rs`).
        .merge(netconfig::routes())
        // Santé et référencement d'un site : dernière lecture et changements (`website.rs`).
        .merge(website::routes())
        // Note de sécurité par équipement et vue du parc (`security.rs`).
        .merge(security::routes())
        // Carte des menaces par pays, toutes sources réunies (`threats.rs`).
        .merge(threats::routes())
        // Failles connues (CVE) de l'inventaire logiciel (`cves.rs`).
        .merge(cves::routes())
        // Notifications Web Push du compte connecté : appareils abonnés, test (`webpush.rs`).
        .merge(webpush::routes())
        // Fichiers du bandeau pour sites tiers : plugin WordPress, aide PHP (`integrations.rs`).
        .merge(integrations::routes())
        // Mise à jour guidée : dernière version publiée (`update.rs`).
        .merge(update::routes())
        // Équipes et droits par dossier (`teams.rs`).
        .merge(teams::routes())
        // Procédures, actions de remédiation, politiques et journal (`remediation.rs`).
        .merge(remediation::routes())
        // Réglages d'instance : mode produit, assistant de démarrage (`instance.rs`).
        .merge(instance::routes())
        // Mode multi-clients (MSP) : clients, appartenance, logo (`clients.rs`).
        .merge(clients::routes())
        // `route_layer` plutôt que `layer` : le garde ne s'applique qu'aux routes
        // effectivement déclarées ici, jamais au repli qui sert l'interface.
        // Objets « gérés par fichier » : refus de les modifier à la main. Posé avant
        // le garde de session pour s'exécuter après lui (`route_layer` : le dernier
        // ajouté passe en premier).
        .route_layer(middleware::from_fn_with_state(state.clone(), crate::gitops::guard::guard))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth::middleware::require_session,
        ));

    // Les seules routes d'API ouvertes. `health` sert au diagnostic et aux sondes
    // d'orchestrateur ; les autres sont ce dont l'interface a besoin pour
    // afficher son écran de connexion et se connecter — les protéger la rendrait
    // inutilisable.
    let public = Router::new()
        .route("/health", get(health::health))
        // La spécification OpenAPI de tout ce qui suit : publique, sans secret.
        .route("/openapi.json", get(openapi::spec))
        // Mode produit et fin de l'assistant : l'interface choisit son habillage
        // avant la connexion (`instance.rs`).
        .merge(instance::public_routes())
        .route("/auth/status", get(auth::status))
        .route("/auth/setup", post(auth::setup))
        .route("/auth/login", post(auth::login))
        .route("/auth/login/totp", post(totp::login_step))
        // Connexion par clé d'accès (WebAuthn), publique comme `login`.
        .merge(passkeys::public_routes())
        .route("/auth/oidc/start", get(oidc::start))
        .route("/auth/oidc/callback", get(oidc::callback))
        // Connexion SAML : métadonnées du fournisseur de service, départ vers
        // l'IdP, réception de sa réponse (un formulaire posté depuis le site de
        // l'IdP, donc sans en-tête anti-CSRF : la signature de l'assertion fait
        // foi) et ouverture de la session (`saml.rs`).
        .route("/auth/saml/metadata", get(saml::metadata))
        .route("/auth/saml/start", get(saml::start))
        .route("/auth/saml/acs", post(saml::acs).layer(DefaultBodyLimit::max(saml::MAX_ACS_BYTES)))
        .route("/auth/saml/complete", get(saml::complete))
        // Réception des mesures poussées par les agents. Volontairement hors du
        // routeur protégé : un agent est un programme installé sur une machine
        // distante, il n'ouvre pas de session et présente un jeton
        // d'enregistrement dans son en-tête `Authorization`. Le passer sous le
        // garde de session couperait tout le parc.
        //
        // La limite de corps est relevée : un lot de rattrapage après une panne de
        // réseau pèse plusieurs mégaoctets, là où le défaut d'axum en refuse deux —
        // et le refuserait à chaque nouvelle tentative, sans issue.
        //
        // Le jeton est vérifié par `require_agent_token` avant que le corps ne
        // soit lu : un inconnu n'obtient pas 16 Mio de désérialisation.
        //
        // Ce sont aussi les routes du canal chiffré des agents (`agent_tls`), qui
        // les sert derrière un certificat client : ici, c'est l'ancien canal,
        // refusé quand l'administrateur exige mTLS.
        .merge(agent_routes(&state))
        // Pages de statut publiques : lecture seule, sans session, par conception.
        .merge(status_pages::public_routes())
        // Calendriers iCal des maintenances : par page, ou privé avec jeton (`ical.rs`).
        .merge(ical::public_routes())
        // Liens d'action des notifications : le jeton signé est l'autorisation
        // (`alert_actions.rs`).
        .merge(alert_actions::public_routes())
        // Calendrier iCal d'astreinte d'une personne : l'URL secrète est
        // l'autorisation, une application de calendrier n'a pas de session.
        .merge(oncall::public_routes())
        // Heartbeats : l'URL secrète qu'un cron ou un script appelle (`push.rs`).
        // Ouverte par nécessité — une crontab n'a ni session ni en-tête anti-CSRF.
        .merge(push::public_routes())
        // Provisionnement SCIM 2.0 (Entra ID, Okta) : hors session, sous le jeton
        // dédié du fournisseur, vérifié par sa propre garde (`scim.rs`).
        .merge(scim::routes(&state));

    // Serveur MCP : authentifié par jeton d'API, pas par session — un assistant
    // n'a pas de navigateur. Le garde ne couvre que `POST` ; `GET` et `DELETE`
    // restent libres, ils ne font qu'expliquer ce qu'est ce point d'entrée (405 :
    // ni flux serveur, ni session à fermer).
    let assistant = Router::new()
        .route("/mcp", post(mcp::post))
        .route_layer(crate::auth::token::require_token(
            state.clone(),
            crate::auth::token::Scope::Read,
        ))
        .route("/mcp", get(mcp::get).delete(mcp::get));

    Router::new()
        .nest("/api", public.merge(protected).merge(assistant).fallback(spa::api_not_found))
        // Lecture par un Prometheus ou un Grafana déjà en place, hors `/api` :
        // `/metrics` (santé de l'instance) et `/federate` (les mesures), sous
        // jeton `read` — voir `prometheus.rs`.
        .merge(prometheus::routes(state.clone()))
        // Réception OpenTelemetry (OTLP/HTTP), sous jeton `write` — voir `otlp.rs`.
        .merge(otlp::routes(state.clone()))
        // Distribution de l'agent, hors `/api` : ce sont les URL que les scripts
        // d'installation lisent, publiques par nécessité (voir `agent_files`).
        .route("/install.sh", get(agent_files::install_sh))
        .route("/install.ps1", get(agent_files::install_ps1))
        .route("/download/{name}", get(agent_files::download))
        // Référencement : fermé, sauf les pages de statut cochées « indexer ».
        .route("/robots.txt", get(seo_files::robots))
        .route("/sitemap.xml", get(seo_files::sitemap))
        // Tout ce qui n'est pas une route d'API est servi par l'interface web, sans
        // authentification : c'est l'application elle-même qui demande le mot de
        // passe, elle doit donc pouvoir se charger d'abord.
        .fallback(spa::serve)
        // `Secure` sur les cookies dès que le navigateur est en HTTPS. Sous la
        // couche qui dépose `AuthState`, dont il lit les mandataires de confiance.
        .layer(middleware::from_fn(crate::auth::cookie::secure_when_https))
        .layer(Extension(auth_state))
        // Mode démonstration : toute écriture refusée ici, avant l'authentification
        // et avant tout gestionnaire — une seule porte, pas une vérification par route.
        .layer(middleware::from_fn_with_state(state.clone(), crate::demo::guard))
        // CORS de l'API ouverte : requêtes à jeton seulement, jamais la session
        // (voir `auth/cors.rs`). Avant le mode démonstration, pour qu'un
        // préflight ne soit pas pris pour une écriture.
        .layer(middleware::from_fn_with_state(
            Arc::new(CorsPolicy::from_env()),
            crate::auth::cors::layer,
        ))
        // `X-DumbMonit-Api-Version` sur toute réponse de `/api`.
        .layer(middleware::from_fn(openapi::version_header))
        // Domaine public d'une page de statut : sur ce nom d'hôte, la page et
        // rien d'autre (`status_host.rs`). Sous les en-têtes de sécurité, pour
        // que ses refus les portent aussi.
        .layer(middleware::from_fn_with_state(state.clone(), crate::status_host::guard))
        .layer(middleware::from_fn(security_headers))
        // Compression gzip/brotli selon `Accept-Encoding` (et `Vary` en retour).
        .layer(compression_layer())
        // Le span ne porte que le chemin : la chaîne de requête d'une route peut
        // contenir un code d'autorisation OIDC ou un jeton — rien de tout cela
        // n'a sa place dans un journal, même en `debug`.
        .layer(TraceLayer::new_for_http().make_span_with(|request: &Request| {
            tracing::debug_span!(
                "request",
                method = %request.method(),
                path = %loggable_path(request.uri().path()),
            )
        }))
        .with_state(state)
}

/// Compression des réponses (brotli ou gzip, au choix du client).
///
/// Niveau 4 : brotli au niveau par défaut (11) est trop lent pour compresser à
/// chaque requête les ~800 Ko du front ; à 4 il reste nettement plus petit que
/// gzip pour un coût comparable. Ne sont pas compressés :
/// - les réponses de moins de 1 Ko (l'en-tête coûte plus que le gain) ;
/// - les images (le logo : déjà compressées) — le
///   SVG, lui, est du texte et l'est ;
/// - les flux d'évènements (SSE) et gRPC, qu'il ne faut pas tamponner ;
/// - les téléchargements (binaires de l'agent, sauvegardes) : fichiers déjà
///   compacts ou chiffrés, dont l'empreinte et la taille annoncée comptent.
///
/// BREACH : la compression ne devient exploitable que si une réponse mêle un
/// secret et une entrée de l'attaquant. Le seul jeton anti-CSRF est la valeur
/// fixe de l'en-tête `X-Requested-With`, jamais dans un corps ; la session est
/// un cookie, et les secrets (jetons d'API, codes) ne sont renvoyés qu'une fois,
/// à la création, sans rien refléter de la requête. Rien à exclure de plus.
pub fn compression_layer() -> CompressionLayer<impl Predicate> {
    let predicate = SizeAbove::new(1024)
        .and(NotForContentType::GRPC)
        .and(NotForContentType::IMAGES)
        .and(NotForContentType::SSE)
        .and(NotForContentType::const_new("application/octet-stream"))
        .and(|_: StatusCode, _: Version, headers: &HeaderMap, _: &Extensions| {
            !headers
                .get(header::CONTENT_DISPOSITION)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.trim_start().starts_with("attachment"))
        });
    CompressionLayer::new().quality(CompressionLevel::Precise(4)).compress_when(predicate)
}

/// Chemin tel qu'il peut figurer dans un journal : le jeton d'un heartbeat
/// (`/api/push/<jeton>`) est le seul secret porté par un chemin, il est masqué.
fn loggable_path(path: &str) -> std::borrow::Cow<'_, str> {
    match path.strip_prefix("/api/push/") {
        Some(rest) if !rest.is_empty() => "/api/push/<redacted>".into(),
        _ => path.into(),
    }
}

/// Politique de contenu commune, sans la partie qui dépend de la requête.
///
/// L'interface ne charge rien qui ne vienne d'elle-même : les polices sont
/// embarquées dans le build, uPlot aussi, et le seul appel réseau est celui de
/// son propre `/api`. Tout part donc de `default-src 'none'`, et chaque
/// directive ouverte ci-dessous l'est pour une raison vérifiée sur le build :
///
/// - `script-src` : `'self'` pour les modules de `_app/`, plus le nonce de la
///   réponse pour les deux scripts en ligne de la page (choix du thème avant le
///   premier rendu, amorce de SvelteKit). Leur contenu — donc leur empreinte —
///   change à chaque construction ; un nonce par réponse est ce qui permet de
///   s'en passer sans rouvrir `'unsafe-inline'`.
/// - `style-src` : `'unsafe-inline'` est **nécessaire et ne peut pas être
///   resserré**. Svelte pose des attributs `style=` sur les éléments (les
///   directives `style:` des composants, le `display: contents` du gabarit), et
///   un attribut n'est couvert ni par un nonce ni par une empreinte — seul
///   `'unsafe-hashes'` le serait, ce qui revient au même en moins lisible. Le
///   risque résiduel est l'exfiltration par feuille de style injectée, qui
///   suppose déjà une injection HTML.
/// - `img-src` : les icônes de l'interface sont des `data:` SVG produits par le
///   build (les flèches de `<select>`, par exemple). L'interface y ajoute, si
///   un fond en ligne est choisi, l'origine des tuiles de la carte du monde.
/// - `manifest-src` : le manifeste web de l'interface (installation sur un
///   téléphone), sans quoi `default-src 'none'` le refuse à chaque page.
/// - `font-src`, `connect-src` : `'self'` et rien d'autre. Aucune police
///   Google, aucun CDN, aucune télémétrie — et la politique le rend vérifiable.
/// - `base-uri 'none'`, `form-action 'self'`, `object-src` hérité de
///   `default-src 'none'` : de quoi rendre inopérantes les variantes
///   d'injection qui ne passent pas par un script.
const CSP_BASE: &str = "default-src 'none'; \
     style-src 'self' 'unsafe-inline'; \
     font-src 'self'; \
     connect-src 'self'; \
     manifest-src 'self'; \
     base-uri 'none'; \
     form-action 'self'";

/// Images : celles du build (`data:` compris), partout.
const CSP_IMG_SRC: &str = "img-src 'self' data:";

/// Chemins qu'une page tierce peut encadrer : une page de statut
/// (`/s/<slug>`) et sa vue compacte (`/s/<slug>/embed`), rien d'autre — ni les
/// pages de confirmation ou de désabonnement, ni un chemin qui ne ferait que
/// commencer par `/s/` pour qu'une application monopage y serve autre chose.
fn is_embeddable(path: &str) -> bool {
    let Some(rest) = path.strip_prefix("/s/") else { return false };
    let rest = rest.strip_suffix('/').unwrap_or(rest);
    let slug = rest.strip_suffix("/embed").unwrap_or(rest);
    (2..=40).contains(&slug.len())
        && slug.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// En-têtes de protection posés sur toute réponse, interface comme API.
///
/// L'interface est une application monopage : encadrée dans une page tierce,
/// elle se prête au détournement de clic. Seules les pages de statut publiques
/// (`/s/…`) sont faites pour être intégrées ailleurs ; elles restent encadrables.
///
/// Sur le domaine public d'une page (`crate::status_host`), la racine est la
/// page elle-même : encadrable comme `/s/<slug>`, et nulle part la politique de
/// l'interface (tuiles de la carte) n'est servie.
async fn security_headers(mut request: Request, next: Next) -> Response {
    let path_embeddable = is_embeddable(request.uri().path());
    let root = request.uri().path() == "/";
    // Un nonce par réponse : 128 bits d'aléa, inutilisables une seconde fois.
    let nonce = hex::encode(rand::random::<[u8; 16]>());
    request.extensions_mut().insert(spa::Nonce(nonce.clone()));

    let mut response = next.run(request).await;
    let status_host = response.extensions().get::<crate::status_host::StatusHost>().is_some();
    // Sur un domaine de page, seul ce qui a été servi est encadrable : un refus
    // (`/s/<autre slug>`, par exemple) garde `DENY`.
    let embeddable = match status_host {
        true => response.status().is_success() && (root || path_embeddable),
        false => path_embeddable,
    };
    let headers = response.headers_mut();
    headers.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    headers.insert("referrer-policy", HeaderValue::from_static("same-origin"));
    let policy = match embeddable {
        // Une page de statut est faite pour être intégrée dans l'intranet de
        // quelqu'un : lui interdire d'être encadrée la rendrait inutile.
        true => format!("{CSP_BASE}; {CSP_IMG_SRC}; script-src 'self' 'nonce-{nonce}'"),
        false if status_host => {
            headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
            format!(
                "{CSP_BASE}; {CSP_IMG_SRC}; script-src 'self' 'nonce-{nonce}'; \
                 frame-ancestors 'none'"
            )
        }
        false => {
            headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
            // Les tuiles de la carte du monde, quand un fond en ligne est
            // choisi : une seule origine, validée à l'enregistrement.
            let tiles = locations::tile_origin().map(|o| format!(" {o}")).unwrap_or_default();
            // Rien n'est encadré et aucun script tiers n'est chargé : sans
            // `frame-src`, `default-src 'none'` refuse toute iframe.
            format!(
                "{CSP_BASE}; {CSP_IMG_SRC}{tiles}; script-src 'self' 'nonce-{nonce}'; \
                 frame-ancestors 'none'"
            )
        }
    };
    if let Ok(value) = HeaderValue::from_str(&policy) {
        headers.insert("content-security-policy", value);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_tokens_never_reach_the_log() {
        assert_eq!(loggable_path("/api/push/abcdef0123456789"), "/api/push/<redacted>");
        assert_eq!(loggable_path("/api/targets/3/push"), "/api/targets/3/push");
        assert_eq!(loggable_path("/api/ingest"), "/api/ingest");
    }

    #[test]
    fn the_policy_forbids_everything_it_does_not_name() {
        assert!(CSP_BASE.starts_with("default-src 'none'"));
        // Rien d'extérieur : pas de joker, pas de `http:`, aucun domaine tiers.
        assert!(!CSP_BASE.contains('*'), "un joker ouvrirait la porte : {CSP_BASE}");
        assert!(!CSP_BASE.contains("http"), "aucune origine externe : {CSP_BASE}");
        // Les scripts ne sont jamais autorisés en ligne sans nonce.
        assert!(!CSP_BASE.contains("script-src"), "script-src dépend de la réponse");
    }

    #[test]
    fn nothing_is_framed_and_no_third_party_script_is_loaded() {
        assert!(!CSP_BASE.contains("frame-src"), "aucune iframe, nulle part");
        assert!(!CSP_BASE.contains("img-src"), "chaque réponse choisit ses images");
        assert!(!CSP_BASE.contains("scdn") && !CSP_IMG_SRC.contains("scdn"));
    }

    #[test]
    fn only_status_pages_and_their_embed_can_be_framed() {
        assert!(is_embeddable("/s/home-lab"));
        assert!(is_embeddable("/s/home-lab/"));
        assert!(is_embeddable("/s/home-lab/embed"));
        assert!(!is_embeddable("/s/home-lab/unsubscribe"));
        assert!(!is_embeddable("/s/home-lab/confirm"));
        assert!(!is_embeddable("/s/x%2F..%2Fsettings"));
        assert!(!is_embeddable("/s/../settings"));
        assert!(!is_embeddable("/settings"));
        assert!(!is_embeddable("/status/3"));
        assert!(!is_embeddable("/api/public/status/home-lab"));
        assert!(!is_embeddable("/s/"));
    }

    #[test]
    fn each_response_gets_its_own_nonce() {
        let first = hex::encode(rand::random::<[u8; 16]>());
        let second = hex::encode(rand::random::<[u8; 16]>());
        assert_ne!(first, second);
        // Le nonce doit tenir dans un en-tête HTTP tel quel.
        assert!(HeaderValue::from_str(&format!("{CSP_BASE}; script-src 'nonce-{first}'")).is_ok());
    }

    /// Réponse d'un mini-routeur sous la couche de compression, telle que la
    /// verrait un client qui annonce `accept`.
    async fn compressed(
        content_type: &'static str,
        disposition: Option<&'static str>,
        size: usize,
        accept: &str,
    ) -> Response {
        use tower::ServiceExt;
        // `fallback` et non une route : le test d'`openapi` lit ce fichier pour
        // y relever les routes déclarées.
        let app = Router::new()
            .fallback(get(move || async move {
                let mut response = Response::new(axum::body::Body::from("abcd".repeat(size / 4)));
                let headers = response.headers_mut();
                headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
                if let Some(value) = disposition {
                    headers.insert(header::CONTENT_DISPOSITION, HeaderValue::from_static(value));
                }
                response
            }))
            .layer(compression_layer());
        let request = Request::builder()
            .uri("/")
            .header(header::ACCEPT_ENCODING, accept)
            .body(axum::body::Body::empty())
            .unwrap();
        app.oneshot(request).await.unwrap()
    }

    fn encoding(response: &Response) -> Option<&str> {
        response.headers().get(header::CONTENT_ENCODING).and_then(|v| v.to_str().ok())
    }

    #[tokio::test]
    async fn text_is_compressed_with_the_clients_preferred_codec() {
        let gzip = compressed("application/json", None, 8192, "gzip").await;
        assert_eq!(encoding(&gzip), Some("gzip"));
        assert_eq!(gzip.headers().get(header::VARY).unwrap(), "accept-encoding");
        let brotli = compressed("application/json", None, 8192, "br, gzip").await;
        assert_eq!(encoding(&brotli), Some("br"));
        assert_eq!(encoding(&compressed("application/json", None, 8192, "identity").await), None);
    }

    #[tokio::test]
    async fn small_images_streams_and_downloads_are_left_alone() {
        assert_eq!(encoding(&compressed("application/json", None, 512, "gzip").await), None);
        for kind in ["image/webp", "image/jpeg", "text/event-stream", "application/octet-stream"] {
            assert_eq!(encoding(&compressed(kind, None, 8192, "gzip, br").await), None, "{kind}");
        }
        let attachment =
            compressed("application/json", Some("attachment; filename=\"b.json\""), 8192, "gzip")
                .await;
        assert_eq!(encoding(&attachment), None);
    }
}
