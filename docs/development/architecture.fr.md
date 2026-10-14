# Architecture

DumbMonit est livré dans un seul conteneur. À l'intérieur, un processus Rust
sert l'interface web et l'API, et démarre puis supervise un processus enfant
(VictoriaMetrics). La configuration et l'état vivent dans SQLite.

## Composants

| Composant | Où | Rôle |
|---|---|---|
| Serveur | `crates/server` | Le binaire Rust unique : API HTTP (axum), authentification, planificateur, moteur d'alertes, canaux de notification, rapports. |
| Interface web | `web/` | Application SvelteKit monopage (Svelte 5, Tailwind 4, uPlot), compilée en fichiers statiques et embarquée dans le binaire avec `rust_embed`. |
| Séries temporelles | `crates/server/src/tsdb/` | Écrivain par lots et proxy de requêtes pour VictoriaMetrics. `embedded.rs` démarre et supervise le VictoriaMetrics fourni (données dans `/data/vm`) ; `DUMBMONIT_VM_URL` désigne une instance externe à la place. |
| Configuration et état | `crates/server/src/db/` | SQLite avec migrations numérotées : équipements, identifiants, utilisateurs, règles, état des alertes, silences, canaux. |
| Collecteurs | `crates/collectors`, `crates/server/src/collectors/` | Une implémentation du trait `Collector` par type d'équipement ou de service, enregistrée dans un `Registry`. Le planificateur ne connaît que le trait. |
| Types partagés | `crates/proto` | `Sample`, `Target`, `Credential`, le trait `Collector` et `ProbeError`, et le protocole d'agent. |
| Agent | `crates/agent` | Agent Linux, macOS, FreeBSD et Windows qui envoie les métriques de la machine ; il peut aussi servir de relais et exécuter les collecteurs d'un site distant. |
| Alertes | `crates/server/src/alerting/` | Règles, machine à états des alertes, suppression par parent, silences, références apprises. |
| Notifications | `crates/server/src/notify/` | Canaux (messagerie, push, astreinte, e-mail, webhooks), décrits à l'interface par `catalog.rs`. |
| Secrets | `crates/server/src/crypto.rs` | AES-256-GCM pour les identifiants et jetons stockés dans SQLite ; la clé vient de `/data/secret.key` ou de `DUMBMONIT_SECRET`. |

## Flux de données

```text
 équipements / services ──(SNMP, API HTTP, sondes)──► collecteurs ┐
                                                                  │ échantillons
 agents ──(envoi, jeton par machine)──► /api/ingest ──────────────┤
 agent relais (collecteurs d'un site distant) ────────────────────┤
                                                                  ▼
                                         écrivain tsdb (lots) ──► VictoriaMetrics
                                                                       │
 navigateur / scripts / assistants ◄─ /api/* ◄─ proxy de requêtes ◄────┘
                                      │
                                      └─► moteur d'alertes ──► canaux de notification

 SQLite (configuration, utilisateurs, état des alertes, silences) est lu et
 écrit par l'API, le planificateur et le moteur d'alertes.
```

1. Le planificateur exécute chaque équipement activé à son intervalle, par le
   registre de collecteurs. Les agents et relais envoient leurs mesures à
   `/api/ingest` avec un jeton propre à chaque machine.
2. Les collecteurs renvoient des échantillons nommés `dumbmonit_<type>_*` avec
   une étiquette `target` (l'identifiant de l'équipement). L'écrivain les
   envoie par lots à VictoriaMetrics.
3. Le moteur d'alertes évalue les règles sur les séries stockées. `phase` est
   l'état du moteur ; `effective_phase` est ce que voient les utilisateurs
   (`suppressed` quand un équipement parent est en panne). Un équipement
   injoignable déclenche une alerte ; une erreur de configuration est affichée
   mais pas notifiée.
4. Les notifications partent par les canaux de la politique de notification.
5. L'interface et les scripts lisent tout par `/api/*` : il n'y a pas de moitié
   privée de l'API.

## Points d'extension

- Un nouveau type d'équipement est une implémentation de `Collector` et un
  appel à `Registry::register`. Les formulaires de l'interface sont générés à
  partir de `GET /api/collectors`.
- Un nouveau canal de notification est un émetteur et une entrée dans
  `notify/catalog.rs` ; l'interface construit le formulaire à partir de
  `GET /api/notify/kinds`.
- Un équipement SNMP qui n'a besoin que d'un profil n'exige pas de Rust : un
  fichier YAML dans `profiles/`.

Voir [Développement](../development.md) pour la compilation et
[Modèle de sécurité](security-model.md) pour les frontières de confiance.
