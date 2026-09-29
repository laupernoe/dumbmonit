//! Collecteurs MDaemon Email Server et SecurityGateway for Email Servers.
//!
//! Deux produits Windows de MDaemon Technologies, deux types d'équipement
//! (`mdaemon` et `securitygateway`), une même démarche :
//!
//! 1. **Les services de messagerie répondent-ils ?** Chaque port surveillé
//!    (SMTP, IMAP, POP3, messagerie web, administration…) est ouvert, et pour les
//!    protocoles texte la bannière d'accueil est lue et jugée : un SMTP qui
//!    accepte la connexion pour répondre `421` n'est pas en service. Aucun
//!    identifiant n'est nécessaire : c'est le socle, et il suffit à alerter sur
//!    l'essentiel.
//! 2. **L'API d'administration, quand un identifiant est fourni.** L'API XML de
//!    MDaemon (`/MdMgmtWS/`, servie par l'administration à distance) ; l'API
//!    REST de SecurityGateway 12.5 et suivants (`/api/v1`, clé d'API).
//!
//! # Ce que la documentation publique permet, et rien de plus
//!
//! Ni l'une ni l'autre API ne publie en ligne le schéma de ses réponses : la
//! référence complète est livrée avec le serveur (`Docs\API`). Ce module ne lit
//! donc que ce qui est documenté publiquement :
//!
//! * pour MDaemon, l'enveloppe commune à toutes les réponses XML —
//!   `<API productversion=… serviceversion=…>` et `<Status value=… message=…>` —
//!   qui donne la version et le verdict de l'appel, quel que soit l'appel ;
//! * pour SecurityGateway, l'enveloppe `{"success": …, "data": …}`, la
//!   spécification OpenAPI servie à `/api/v1/openapi`, et l'existence de
//!   compteurs de performance en lecture seule. Le chemin de ces compteurs est
//!   trouvé dans la spécification, et leurs valeurs numériques sont reprises
//!   telles quelles, sans supposer leurs noms.
//!
//! Les files d'attente de MDaemon ne sont publiées que sous forme de compteurs
//! de performance Windows ; aucune opération documentée de l'API XML ne les
//! donne. Elles ne sont pas inventées ici (voir `docs/devices/mdaemon.md`).
//!
//! # Principes
//!
//! * **Une panne partielle reste une collecte réussie.** Un service arrêté
//!   produit `…_service_up = 0` ; seule l'absence totale de réponse — aucun
//!   port, aucune API — fait échouer l'interrogation (« injoignable »).
//! * **Les erreurs sont classées.** Un mot de passe ou une clé refusés donnent
//!   `ProbeError::Auth`, jamais « équipement hors ligne ».
//! * **Aucun secret ne sort d'ici**, ni dans un journal, ni dans un message
//!   d'erreur : les types qui en portent ne dérivent pas `Debug`.

mod address;
mod email_server;
mod ports;
mod security_gateway;
mod xmlapi;

pub use email_server::MdaemonCollector;
pub use security_gateway::SecurityGatewayCollector;
