//! Règles livrées par défaut.
//!
//! Objectif produit : une instance fraîchement installée doit alerter utilement
//! sans que personne n'ait rien réglé. Ces règles sont insérées au premier
//! démarrage puis appartiennent à l'utilisateur — elles ne sont jamais réécrites
//! par-dessus ses modifications, seules les règles manquantes sont recréées.

use std::time::Duration;

use crate::alerting::model::{
    AnomalyParams, Operator, RULE_HOST_DOWN, Rule, RuleKind, Severity, TargetSelector,
};

/// Taux d'occupation processeur, toutes sources confondues.
///
/// Chaque collecteur nomme la sienne : `cpu_load_percent` pour SNMP
/// (HOST-RESOURCES-MIB), `proxmox_node_cpu_percent` pour Proxmox,
/// `cpu_usage_percent` pour l'agent et la démonstration. L'union `or` fait que la
/// règle livrée fonctionne quel que soit le type d'équipement ajouté, sans que
/// l'utilisateur ait à écrire quoi que ce soit.
const CPU_PERCENT: &str = "avg by (target, host) (\
     dumbmonit_cpu_load_percent \
     or dumbmonit_proxmox_node_cpu_percent \
     or dumbmonit_cpu_usage_percent)";

/// Taux de remplissage des systèmes de fichiers.
///
/// SNMP expose des octets utilisés et totaux (HOST-RESOURCES-MIB), pas un
/// pourcentage : il est calculé ici. Proxmox fournit déjà le taux.
const FS_USED_PERCENT: &str = "(\
     100 * dumbmonit_storage_bytes_used / dumbmonit_storage_bytes_total \
     or dumbmonit_proxmox_storage_used_percent \
     or dumbmonit_proxmox_node_rootfs_percent)";

/// Rappel par défaut : six heures. Assez pour ne pas oublier une panne en cours,
/// assez peu pour ne pas devenir du bruit pendant une semaine de vacances.
const DEFAULT_REPEAT: Duration = Duration::from_secs(6 * 3600);

fn base(uid: &str, name: &str, kind: RuleKind, query: &str) -> Rule {
    Rule {
        // `id = 0` : la règle n'est pas encore en base, l'insertion l'attribuera.
        id: 0,
        uid: uid.to_string(),
        name: name.to_string(),
        description: String::new(),
        kind,
        query: query.to_string(),
        operator: Operator::Gt,
        threshold: 0.0,
        // Pas d'hystérésis par défaut : chaque règle choisit son seuil de retour.
        clear_threshold: None,
        for_duration: Duration::ZERO,
        severity: Severity::Warning,
        selector: TargetSelector::All,
        // Vide signifie « tous les canaux actifs » : sans cela, les règles livrées
        // seraient muettes tant que l'utilisateur n'aurait pas pensé à les rattacher
        // au canal qu'il vient de créer.
        channels: Vec::new(),
        params: AnomalyParams::default(),
        unit: String::new(),
        repeat_interval: Some(DEFAULT_REPEAT),
        escalate_after: None,
        enabled: true,
        builtin: true,
    }
}

/// Les règles livrées avec le produit.
pub fn builtin_rules() -> Vec<Rule> {
    vec![
        // Équipement injoignable.
        //
        // On ne cherche pas un `up == 0` : le planificateur n'écrit rien quand
        // l'interrogation échoue, la série s'arrête simplement. On mesure donc l'âge
        // du dernier échantillon, ce qui détecte aussi bien une panne réseau qu'un
        // collecteur bloqué. La fenêtre de sept jours borne le coût de la requête ;
        // au-delà, la cible n'a plus de série du tout et l'interface prend le relais.
        Rule {
            description: "No measurement received for more than three minutes.".to_string(),
            operator: Operator::Gt,
            threshold: 180.0,
            for_duration: Duration::from_secs(60),
            severity: Severity::Critical,
            unit: "s".to_string(),
            ..base(
                RULE_HOST_DOWN,
                "Device unreachable",
                RuleKind::Threshold,
                "time() - tlast_over_time(dumbmonit_up[7d])",
            )
        },
        // Processeur élevé. L'agrégation ramène les cœurs à une seule série par
        // équipement : personne ne veut vingt-quatre alertes pour un seul serveur.
        Rule {
            description: "CPU load sustained above 90%.".to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            // Hystérésis : une charge qui oscille entre 88 et 92 % ne doit pas
            // déclencher et résoudre à chaque cycle.
            clear_threshold: Some(85.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            escalate_after: Some(Duration::from_secs(3600)),
            ..base("cpu_high", "High CPU", RuleKind::Threshold, CPU_PERCENT)
        },
        // Disque presque plein, par point de montage : ici on veut bien une série
        // par système de fichiers, la remédiation n'étant pas la même.
        Rule {
            description: "Filesystem 90% full or more.".to_string(),
            operator: Operator::Ge,
            threshold: 90.0,
            clear_threshold: Some(88.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            escalate_after: Some(Duration::from_secs(24 * 3600)),
            ..base("disk_almost_full", "Disk almost full", RuleKind::Threshold, FS_USED_PERCENT)
        },
        // Onduleur sur batterie : `for` très court, la coupure est déjà l'événement.
        Rule {
            description: "The UPS is powering the load from battery.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(900)),
            ..base(
                "ups_on_battery",
                "UPS on battery",
                RuleKind::Threshold,
                // upsOutputSource : 5 = batterie (RFC 1628). La comparaison rend une
                // série valant 1 seulement quand la condition est vraie, ce qui donne
                // bien « supérieur à 0 » comme test de déclenchement.
                "dumbmonit_ups_output_source == 5",
            )
        },
        // Prédictif : tout le calcul est fait par VictoriaMetrics. Le `and deriv(...)`
        // écarte les systèmes de fichiers stables ou en décroissance, pour lesquels
        // l'extrapolation linéaire n'a aucun sens.
        Rule {
            description: "At this rate, the filesystem will be full within four days.".to_string(),
            operator: Operator::Ge,
            threshold: 100.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            ..base(
                "fs_will_be_full",
                "Filesystem almost full",
                RuleKind::Predict,
                &predict_full_query(FS_USED_PERCENT, 6, 4),
            )
        },
        // Anomalie saisonnière sur le processeur. Le seuil et la durée sont ceux du
        // jalon 5 ; la règle reste muette pendant les quatorze jours d'apprentissage.
        Rule {
            description: "CPU load noticeably different from the usual at this time \
                          and day of the week."
                .to_string(),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Info,
            unit: "%".to_string(),
            ..base("cpu_anomaly", "Unusual CPU", RuleKind::Anomaly, CPU_PERCENT)
        },
        // Batterie d'onduleur en fin de vie ou déchargée. Distincte de « sur
        // batterie » : une coupure brève est normale, une batterie basse annonce un
        // arrêt brutal des équipements alimentés.
        Rule {
            description: "The UPS battery is low or depleted.".to_string(),
            operator: Operator::Ge,
            threshold: 3.0,
            for_duration: Duration::from_secs(60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(1800)),
            ..base(
                "ups_battery_low",
                "UPS battery low",
                RuleKind::Threshold,
                // upsBatteryStatus : 3 = basse, 4 = épuisée (RFC 1628).
                "dumbmonit_ups_battery_status",
            )
        },
        // Port réseau qui accumule les erreurs (IF-MIB, tout équipement SNMP).
        //
        // `increase` sur une heure plutôt qu'un seuil sur le compteur brut : un
        // commutateur allumé depuis trois ans a forcément quelques erreurs, ce
        // qui compte est qu'elles continuent d'arriver. Le relevé réel d'un Zyxel
        // montre le cas type : un port au lien tombé, des milliers d'erreurs en
        // entrée, les ports voisins à zéro. Cinquante par heure écarte la trame
        // isolée d'un débranchement.
        //
        // `increase_prometheus` et non `increase` : sur une série neuve,
        // VictoriaMetrics compte la première valeur comme un accroissement quand
        // elle lui paraît petite — vérifié sur le relevé Zyxel, où un port à sept
        // erreurs historiques ressortait avec sept erreurs « dans l'heure » dès
        // l'ajout de l'équipement. La variante Prometheus exige deux points dans
        // la fenêtre : rien ne sonne pendant la première heure, puis seules les
        // erreurs réellement nouvelles comptent.
        Rule {
            description: "A network port logged more than fifty errors in the last hour: \
                          usually a failing cable, a dirty fibre or a duplex mismatch."
                .to_string(),
            operator: Operator::Gt,
            threshold: 50.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "port_errors",
                "Port accumulating errors",
                RuleKind::Threshold,
                "increase_prometheus(dumbmonit_if_errors_in[1h]) \
                 + increase_prometheus(dumbmonit_if_errors_out[1h])",
            )
        },
        // Port qui bagote : chaque montée et chaque descente change ifOperStatus.
        // Plus de quatre changements en trente minutes, c'est au moins deux
        // coupures, ce qu'aucune maintenance ordinaire ne produit. Un port coupé
        // par l'administrateur disparaît des séries (filtre d'IF-MIB) et ne peut
        // donc pas déclencher. `changes_prometheus`, pour la même raison que
        // ci-dessus : `changes` compte la première valeur d'une série neuve.
        Rule {
            description: "A network port went down and up again more than twice in thirty \
                          minutes."
                .to_string(),
            operator: Operator::Gt,
            threshold: 4.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "port_flapping",
                "Port flapping",
                RuleKind::Threshold,
                "changes_prometheus(dumbmonit_if_oper_status[30m])",
            )
        },
        // Matériel serveur vu par le contrôleur de gestion, via Redfish
        // (`collectors/redfish`). Les séries de santé valent 0 OK, 1 Warning,
        // 2 Critical ; un emplacement « Absent » ou « Disabled » n'en a pas.
        Rule {
            description: "The server's management controller reports a fan as failed \
                          (health Critical)."
                .to_string(),
            operator: Operator::Ge,
            threshold: 2.0,
            for_duration: Duration::from_secs(2 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "redfish_fan_failed",
                "Server fan failed",
                RuleKind::Threshold,
                "dumbmonit_redfish_fan_health",
            )
        },
        // Même principe que la règle de l'agent : le seuil est celui que le
        // capteur publie (UpperThresholdCritical), jamais une valeur en dur.
        Rule {
            description: "A server temperature sensor is above the critical threshold its \
                          management controller declares."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            unit: "°C".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "redfish_temperature_critical",
                "Server temperature above critical",
                RuleKind::Threshold,
                "dumbmonit_redfish_temperature_celsius \
                 >= dumbmonit_redfish_temperature_upper_critical_celsius",
            )
        },
        // Redondance d'alimentation perdue : le serveur tourne encore, sur une
        // seule alimentation. C'est l'alerte qui laisse le temps d'agir.
        Rule {
            description: "The server's power supplies are no longer redundant: one more \
                          failure and it goes down."
                .to_string(),
            operator: Operator::Ge,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            escalate_after: Some(Duration::from_secs(3600)),
            ..base(
                "redfish_psu_redundancy_lost",
                "Power supply redundancy lost",
                RuleKind::Threshold,
                "dumbmonit_redfish_power_redundancy_health",
            )
        },
        Rule {
            description: "The server's management controller reports a power supply as \
                          failed (health Critical)."
                .to_string(),
            operator: Operator::Ge,
            threshold: 2.0,
            for_duration: Duration::from_secs(2 * 60),
            severity: Severity::Critical,
            ..base(
                "redfish_psu_failed",
                "Power supply failed",
                RuleKind::Threshold,
                "dumbmonit_redfish_psu_health",
            )
        },
        // FailurePredicted : le contrôleur de stockage (SMART, PFA) annonce la fin
        // prochaine du disque. Il fonctionne encore : c'est le moment de le changer.
        Rule {
            description: "A drive reports a predicted failure: replace it while it still \
                          works."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "redfish_drive_failure_predicted",
                "Drive failure predicted",
                RuleKind::Threshold,
                "dumbmonit_redfish_drive_failure_predicted",
            )
        },
        // La santé propre du système (Status.Health), pas son agrégat : le
        // HealthRollup redirait en double ce que les règles ci-dessus disent déjà
        // composant par composant.
        Rule {
            description: "The server's management controller reports the system health as \
                          Critical."
                .to_string(),
            operator: Operator::Ge,
            threshold: 2.0,
            for_duration: Duration::from_secs(2 * 60),
            severity: Severity::Critical,
            ..base(
                "redfish_system_critical",
                "Server health critical",
                RuleKind::Threshold,
                "dumbmonit_redfish_system_health",
            )
        },
        // Sauvegarde Proxmox trop ancienne. C'est le genre de panne silencieuse qui
        // ne se découvre qu'au moment de restaurer, quand il est trop tard.
        Rule {
            description: "No successful backup for more than seven days.".to_string(),
            operator: Operator::Gt,
            threshold: 7.0 * 24.0 * 3600.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "backup_too_old",
                "Backup too old",
                RuleKind::Threshold,
                "dumbmonit_proxmox_backup_last_age_seconds",
            )
        },
        // Proxmox Backup Server. Les seuils sont ceux d'un homelab qui sauvegarde
        // chaque nuit : deux jours sans sauvegarde, c'est une nuit ratée plus la
        // marge d'une nuit ; huit jours sans GC, c'est une GC hebdomadaire manquée.
        Rule {
            description: "PBS datastore more than 90% full.".to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            escalate_after: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pbs_datastore_almost_full",
                "PBS datastore almost full",
                RuleKind::Threshold,
                "dumbmonit_pbs_datastore_used_percent",
            )
        },
        // L'estimation est calculée par PBS lui-même sur un mois de mesures : elle
        // est absente tant qu'il manque de points, la règle reste alors muette.
        Rule {
            description: "At the current rate, PBS estimates the datastore full within seven days."
                .to_string(),
            operator: Operator::Lt,
            threshold: 7.0 * 24.0 * 3600.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pbs_datastore_will_be_full",
                "PBS datastore filling up",
                RuleKind::Threshold,
                "dumbmonit_pbs_datastore_estimated_full_seconds",
            )
        },
        // Même logique que `backup_too_old` côté PVE, par groupe de sauvegarde.
        Rule {
            description: "No new snapshot for this machine for more than two days.".to_string(),
            operator: Operator::Gt,
            threshold: 2.0 * 24.0 * 3600.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pbs_backup_too_old",
                "PBS backup too old",
                RuleKind::Threshold,
                "dumbmonit_pbs_backup_last_age_seconds",
            )
        },
        // Critique : une sauvegarde dont la vérification échoue est une sauvegarde
        // que l'on ne pourra pas restaurer. La série vaut 1 (vérifiée) ou 0 (en
        // échec) et n'existe pas pour un instantané jamais vérifié : « < 1 » ne vise
        // donc que les vrais échecs. On ne filtre pas par `== 0` côté MetricsQL :
        // la série renvoyée garderait sa valeur 0, et aucun seuil « > 0 » ne la
        // verrait.
        Rule {
            description: "Verification of the latest snapshot for this machine failed.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pbs_backup_verification_failed",
                "PBS backup verification failed",
                RuleKind::Threshold,
                "dumbmonit_pbs_backup_last_verified",
            )
        },
        Rule {
            description: "At least one PBS task failed in the review window.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pbs_task_failed",
                "PBS task failed",
                RuleKind::Threshold,
                "dumbmonit_pbs_tasks_failed",
            )
        },
        // Sans GC, les blocs des instantanés supprimés ne sont jamais libérés et
        // le datastore se remplit sans raison apparente.
        Rule {
            description:
                "No successful garbage collection (GC) on this datastore for more than eight days."
                    .to_string(),
            operator: Operator::Gt,
            threshold: 8.0 * 24.0 * 3600.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pbs_gc_too_old",
                "PBS garbage collection too old",
                RuleKind::Threshold,
                "dumbmonit_pbs_gc_last_success_age_seconds",
            )
        },
        // Moniteurs de disponibilité. Contrairement au matériel, un service en
        // panne écrit bien un point (`probe_success = 0`) : la règle est un simple
        // seuil, et le `for` absorbe un raté isolé. `== bool 0` rend 1 pour un
        // service en panne ; sans `bool`, la comparaison renverrait 0 (la valeur
        // de gauche) et le seuil « > 0 » ne se déclencherait jamais.
        //
        // Les heartbeats (`probe="push"`) écrivent la même série mais ont leur
        // propre règle, `push_missed`, en avertissement : un cron en retard n'est
        // pas une panne critique, et il ne doit pas alerter deux fois.
        Rule {
            description: "The service has not responded correctly for three minutes.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(1800)),
            ..base(
                "service_down",
                "Service down",
                RuleKind::Threshold,
                "dumbmonit_probe_success{probe!=\"push\"} == bool 0",
            )
        },
        // Heartbeat manqué. Le collecteur `push` écrit `probe_success = 0` dès que
        // le dernier appel dépasse la période attendue plus la tolérance, ou que
        // le script a lui-même signalé `status=down` ; la règle n'a donc aucun
        // délai à connaître — il est propre à chaque cible. Le `for` court
        // n'absorbe que le battement du planificateur : la tolérance est déjà
        // dans le verdict.
        Rule {
            description: "The job has not called in within its expected interval plus grace period, or reported a failure itself."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(2 * 60),
            severity: Severity::Warning,
            ..base(
                "push_missed",
                "Heartbeat missed",
                RuleKind::Threshold,
                "dumbmonit_probe_success{probe=\"push\"} == bool 0",
            )
        },
        // Instabilité : un service qui alterne sans cesse n'est jamais « en panne »
        // assez longtemps pour la règle précédente, et pourtant il est inutilisable.
        Rule {
            description: "The service changed state more than six times in thirty minutes."
                .to_string(),
            operator: Operator::Gt,
            threshold: 6.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "service_flapping",
                "Service flapping",
                RuleKind::Threshold,
                "changes(dumbmonit_probe_success{probe!=\"push\"}[30m])",
            )
        },
        // Lenteur : le défaut de délai des sondes est de 5 s, au-delà elles échouent
        // déjà ; 3 s soutenues pendant dix minutes est un service qui souffre.
        Rule {
            description: "The service takes more than three seconds to respond.".to_string(),
            operator: Operator::Gt,
            threshold: 3.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "s".to_string(),
            ..base(
                "service_slow",
                "Slow service",
                RuleKind::Threshold,
                "dumbmonit_probe_duration_seconds",
            )
        },
        // Certificats. Les deux règles sont disjointes (`>= 0` / `< 0`) pour qu'un
        // certificat périmé ne déclenche pas aussi « bientôt expiré ».
        Rule {
            description: "The certificate expires in less than fourteen days.".to_string(),
            operator: Operator::Lt,
            threshold: 14.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "d".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "tls_cert_expiring",
                "Certificate expiring soon",
                RuleKind::Threshold,
                "dumbmonit_probe_ssl_cert_expiry_days >= 0",
            )
        },
        Rule {
            description: "The certificate has expired.".to_string(),
            operator: Operator::Lt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            unit: "d".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "tls_cert_expired",
                "Certificate expired",
                RuleKind::Threshold,
                "dumbmonit_probe_ssl_cert_expiry_days < 0",
            )
        },
        // Conteneurs Docker, vus par l'agent. Chaque série porte le nom du
        // conteneur : l'alerte le montre, et une politique peut y répondre.
        Rule {
            description: "The container has been stopped for two minutes.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(2 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "container_stopped",
                "Container stopped",
                RuleKind::Threshold,
                // `== bool` rend 1 quand le conteneur est arrêté, 0 sinon ; sans
                // `bool`, MetricsQL renverrait la valeur de gauche — 0 — qu'un
                // seuil « > 0 » ne verrait jamais. Voir `service_down`.
                "dumbmonit_container_up == bool 0",
            )
        },
        Rule {
            description: "The container's health check has been failing for three minutes."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "container_unhealthy",
                "Container unhealthy",
                RuleKind::Threshold,
                // 2 = unhealthy (0 none, 1 healthy, 3 starting).
                "dumbmonit_container_health == 2",
            )
        },
        // Un conteneur qui redémarre en boucle est rarement « arrêté » assez
        // longtemps pour la règle précédente.
        Rule {
            description: "The container restarted three times or more in fifteen minutes."
                .to_string(),
            operator: Operator::Ge,
            threshold: 3.0,
            for_duration: Duration::from_secs(60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "container_restarting",
                "Container restarting",
                RuleKind::Threshold,
                // `increase_prometheus` et non `increase` : ce dernier compte la
                // première valeur d'une série neuve comme une hausse. Un conteneur
                // qui a redémarré cinquante fois l'an dernier et dont la série
                // réapparaît (agent réinstallé, étiquettes changées) sonnerait
                // aussitôt « cinquante redémarrages en un quart d'heure ».
                "increase_prometheus(dumbmonit_container_restart_count[15m])",
            )
        },
        // Information, pas panne : une image plus récente existe dans le dépôt.
        // Une heure de `for` absorbe une vérification passagèrement fausse.
        Rule {
            description: "A newer image is available in the registry for this container."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "container_update_available",
                "Container update available",
                RuleKind::Threshold,
                "dumbmonit_container_update_available == 1",
            )
        },
        // Sauvegardes Plakar, par kloset et par source. Mêmes seuils que PBS :
        // deux jours, c'est une nuit ratée plus la marge d'une nuit. Les deux
        // règles ne visent que des séries étiquetées `kloset`, que l'agent
        // n'émet qu'une fois Plakar détecté : sans Plakar, rien ne peut se
        // déclencher.
        Rule {
            description: "No new Plakar snapshot for this source for more than two days."
                .to_string(),
            operator: Operator::Gt,
            threshold: 2.0 * 24.0 * 3600.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "plakar_backup_too_old",
                "Plakar backup too old",
                RuleKind::Threshold,
                "dumbmonit_backup_last_success_seconds",
            )
        },
        // La série vaut 1 (dernier instantané sans erreur) ou 0 (erreurs, ou
        // kloset illisible) : « < 1 » ne vise que les vrais échecs.
        Rule {
            description: "The latest Plakar snapshot has errors, or the kloset cannot be read."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "plakar_backup_failed",
                "Plakar backup failed",
                RuleKind::Threshold,
                "dumbmonit_backup_last_status",
            )
        },
        // ------------------------------------------------------------------
        // Matériel remonté par l'agent : disques, pools ZFS, sondes.
        //
        // Ces quatre règles ne visent que des séries que l'agent n'émet **que**
        // s'il a trouvé de quoi les remplir — `smartctl` installé, un pool ZFS,
        // une sonde qui annonce son propre seuil. Sur une machine sans rien de
        // tout cela, elles ne peuvent pas se déclencher, et n'ont donc pas à
        // être désactivées.
        // ------------------------------------------------------------------

        // Le disque a fait son autodiagnostic et se déclare en fin de vie. C'est
        // le préavis le plus précieux qu'un homelab puisse recevoir, et il
        // arrive en général des semaines avant la panne.
        Rule {
            description: "This disk's own SMART self-assessment reports it as failing. \
                          Replace it and check the backups before it goes."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "disk_smart_failed",
                "Disk SMART failing",
                RuleKind::Threshold,
                // « == bool 0 » rend une série valant 1 par disque en panne
                // annoncée et 0 pour les autres : le seuil se lit sans détour.
                "dumbmonit_agent_disk_smart_ok == bool 0",
            )
        },
        // 0 = ONLINE, 1 = DEGRADED, 2 = tout le reste. « > 0 » attrape donc le
        // pool dégradé comme le pool en faute, sans énumérer des états dont la
        // liste s'allonge à chaque version d'OpenZFS.
        Rule {
            description: "A ZFS pool is no longer healthy: a device is missing, faulted \
                          or removed."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "zfs_pool_degraded",
                "ZFS pool degraded",
                RuleKind::Threshold,
                "dumbmonit_agent_zfs_pool_health",
            )
        },
        // Un nettoyage qui trouve des erreurs a lu des données abîmées. Le pool
        // peut être ONLINE et le rester : c'est justement le silence que cette
        // règle vient rompre.
        Rule {
            description: "The last ZFS scrub found errors on this pool.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "zfs_scrub_errors",
                "ZFS scrub found errors",
                RuleKind::Threshold,
                // Les fichiers définitivement perdus comptent autant que les
                // erreurs relevées par le nettoyage lui-même.
                "dumbmonit_agent_zfs_pool_scrub_errors \
                 or dumbmonit_agent_zfs_pool_data_errors",
            )
        },
        // Température au-dessus du seuil critique **que la sonde annonce
        // elle-même**. Pas de valeur en dur : 85 °C est une alerte sur un disque
        // et une journée ordinaire pour un processeur de portable.
        Rule {
            description: "A sensor is above the critical temperature its own hardware \
                          declares."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            unit: "°C".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "sensor_temperature_critical",
                "Temperature above critical",
                RuleKind::Threshold,
                // Les deux séries portent exactement les mêmes étiquettes : la
                // comparaison les apparie d'elle-même, et rend la température,
                // qui est ce qu'on veut lire dans l'alerte.
                "dumbmonit_agent_sensor_temperature_celsius \
                 >= dumbmonit_agent_sensor_temperature_critical_celsius",
            )
        },
        // Active Backup for Business (`collectors/synology/abb.rs`). `last_status`
        // vaut 1 réussite, 0 échec, 2 en cours, -1 inconnu : « == bool 0 » rend une
        // série valant 1 pour chaque tâche en échec et 0 pour les autres, ce qu'un
        // seuil « > 0 » lit sans ambiguïté — un « < 1 » attraperait aussi l'inconnu.
        Rule {
            description: "The last run of this Active Backup for Business task failed, \
                          or backed up only part of its devices."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "synology_abb_task_failed",
                "Active Backup task failed",
                RuleKind::Threshold,
                "dumbmonit_abb_task_last_status == bool 0",
            )
        },
        // Mêmes seuils que les autres sauvegardes : deux jours, c'est une nuit ratée
        // plus la marge d'une nuit. La série n'existe pas pour une tâche qui n'a
        // jamais réussi ; c'est alors `synology_abb_task_failed` qui parle.
        Rule {
            description: "No successful Active Backup for Business run for this task for more \
                          than two days."
                .to_string(),
            operator: Operator::Gt,
            threshold: 2.0 * 24.0 * 3600.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "synology_abb_backup_too_old",
                "Active Backup too old",
                RuleKind::Threshold,
                "dumbmonit_abb_task_last_success_seconds",
            )
        },
        // Information : une tâche sans planning ne sauvegardera plus rien tant que
        // personne ne la lance à la main. Une heure de `for` absorbe une
        // reprogrammation en cours.
        Rule {
            description: "This Active Backup for Business task has no schedule, or its \
                          continuous backup is paused."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "synology_abb_task_disabled",
                "Active Backup task disabled",
                RuleKind::Threshold,
                "dumbmonit_abb_task_enabled == bool 0",
            )
        },
        // Proxmox VE, au-delà des sauvegardes. Sévérités : « warning » de la
        // ligne de produit = `Critical` ici, « advisory » = `Warning`.
        //
        // Machine arrêtée alors qu'elle tournait. `1 - running` vaut 1 pour une
        // machine arrêtée ; le `and` ne retient que celles vues en marche dans les
        // deux dernières heures, pour ne pas alerter sur une machine arrêtée de
        // longue date. Une comparaison MetricsQL renvoie la valeur de gauche, pas
        // un booléen : `running == 0` donnerait 0, invisible d'un seuil « > 0 ».
        Rule {
            description: "The VM or container was running and has been stopped for five minutes."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pve_guest_stopped",
                "VM or container stopped",
                RuleKind::Threshold,
                "(1 - dumbmonit_proxmox_guest_running) \
                 and (max_over_time(dumbmonit_proxmox_guest_running[2h]) == 1)",
            )
        },
        // Haute disponibilité : une ressource en `error` ou `fence` ne redémarrera
        // pas toute seule, c'est une intervention.
        Rule {
            description: "A high-availability resource is in error or fenced.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(2 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "pve_ha_resource_error",
                "HA resource in error",
                RuleKind::Threshold,
                "dumbmonit_proxmox_ha_resource_error",
            )
        },
        // Quorum perdu : plus aucune machine ne peut démarrer sur le cluster, et la
        // HA arrête celles qui tournent.
        Rule {
            description: "The cluster has lost quorum: guests can no longer be started."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(1800)),
            ..base(
                "pve_cluster_no_quorum",
                "Cluster lost quorum",
                RuleKind::Threshold,
                "1 - dumbmonit_proxmox_cluster_quorate",
            )
        },
        // Nœud hors ligne vu du cluster. Distinct de « équipement injoignable » :
        // l'API répond, c'est un membre qui manque.
        Rule {
            description: "A cluster node has been offline for two minutes.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(2 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(1800)),
            ..base(
                "pve_node_offline",
                "Proxmox node offline",
                RuleKind::Threshold,
                "1 - dumbmonit_proxmox_node_up",
            )
        },
        // Stockage à 85 % : un cran avant « Disk almost full » (90 %), qui reste
        // l'alerte sérieuse. Un LVM-thin plein bloque toutes les machines qu'il
        // héberge, il vaut mieux prévenir tôt.
        Rule {
            description: "A Proxmox storage is more than 85% full.".to_string(),
            operator: Operator::Gt,
            threshold: 85.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pve_storage_almost_full",
                "Proxmox storage almost full",
                RuleKind::Threshold,
                "dumbmonit_proxmox_storage_used_percent",
            )
        },
        // Dernier travail de sauvegarde du nœud en échec (tâches `vzdump`).
        Rule {
            description: "The last backup job on this node failed.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pve_backup_job_failed",
                "Backup job failed",
                RuleKind::Threshold,
                "1 - dumbmonit_proxmox_backup_job_last_ok",
            )
        },
        // Instantané oublié : il grossit avec le temps et ralentit la machine, et
        // personne ne s'en souvient. Information, pas panne.
        Rule {
            description: "The oldest snapshot of this machine is more than thirty days old."
                .to_string(),
            operator: Operator::Gt,
            threshold: 30.0 * 24.0 * 3600.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pve_snapshot_old",
                "Old snapshot",
                RuleKind::Threshold,
                "dumbmonit_proxmox_guest_snapshot_oldest_age_seconds",
            )
        },
        // Réplication en échec : la copie de secours de la machine n'est plus à
        // jour, la bascule restaurerait un état ancien.
        Rule {
            description: "A replication job is failing: the standby copy is stale.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pve_replication_failed",
                "Replication failed",
                RuleKind::Threshold,
                "dumbmonit_proxmox_replication_job_error",
            )
        },
        // Ceph : 0 OK, 1 WARN, 2 ERR. Les deux règles sont disjointes (`>= 2` et
        // `== 1`) pour qu'un HEALTH_ERR ne déclenche pas aussi l'avertissement.
        Rule {
            description: "Ceph reports HEALTH_ERR: data may be unavailable.".to_string(),
            operator: Operator::Ge,
            threshold: 2.0,
            for_duration: Duration::from_secs(2 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(1800)),
            ..base(
                "pve_ceph_health_error",
                "Ceph health error",
                RuleKind::Threshold,
                "dumbmonit_proxmox_ceph_health",
            )
        },
        Rule {
            description: "Ceph reports HEALTH_WARN for more than fifteen minutes.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pve_ceph_health_warning",
                "Ceph health warning",
                RuleKind::Threshold,
                "dumbmonit_proxmox_ceph_health == 1",
            )
        },
        // Mises à jour en attente : information, une fois par semaine suffit.
        Rule {
            description: "More than twenty package updates are pending on this node.".to_string(),
            operator: Operator::Gt,
            threshold: 20.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pve_updates_pending",
                "Proxmox updates pending",
                RuleKind::Threshold,
                "dumbmonit_proxmox_node_updates_pending",
            )
        },
        // Certificat de l'interface d'un nœud : PVE renouvelle les siens lui-même,
        // pas ceux importés à la main.
        Rule {
            description: "A node certificate expires in less than fourteen days.".to_string(),
            operator: Operator::Lt,
            threshold: 14.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "d".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pve_certificate_expiring",
                "Node certificate expiring",
                RuleKind::Threshold,
                "dumbmonit_proxmox_node_certificate_expiry_days",
            )
        },
        // Pool à provisionnement fin saturé : toutes les machines qu'il héberge
        // basculent en lecture seule d'un coup, et un pool plein ne se vide pas
        // en supprimant des fichiers dans les machines. Critique, et plus tôt
        // que pour un stockage ordinaire.
        Rule {
            description: "An LVM thin pool is more than 90% full: its guests will go read-only."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(88.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Critical,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pve_thinpool_almost_full",
                "Thin pool almost full",
                RuleKind::Threshold,
                "dumbmonit_proxmox_node_thinpool_used_percent",
            )
        },
        // Les métadonnées d'un pool fin tiennent sur un volume minuscule et
        // saturent souvent avant les données — avec exactement les mêmes
        // conséquences, pour une cause que personne ne pense à regarder.
        Rule {
            description: "The metadata volume of an LVM thin pool is more than 80% full."
                .to_string(),
            operator: Operator::Gt,
            threshold: 80.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Critical,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pve_thinpool_metadata_full",
                "Thin pool metadata almost full",
                RuleKind::Threshold,
                "dumbmonit_proxmox_node_thinpool_metadata_used_percent",
            )
        },
        // Un démon essentiel arrêté est la panne la plus traître de Proxmox :
        // sans `pvestatd`, l'interface continue d'afficher les chiffres du
        // moment où il s'est arrêté, et tout a l'air normal.
        Rule {
            description: "A core Proxmox service is not running on this node.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "pve_service_down",
                "Proxmox service down",
                RuleKind::Threshold,
                "dumbmonit_proxmox_node_core_services_down",
            )
        },
        // Pont ou agrégat déclaré au démarrage qui n'est pas monté : toutes les
        // machines qui s'y rattachent sont coupées du réseau, et elles-mêmes
        // tournent parfaitement.
        Rule {
            description: "A network interface set to start at boot is down on this node."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pve_interface_offline",
                "Node network interface down",
                RuleKind::Threshold,
                "dumbmonit_proxmox_node_interface_offline",
            )
        },
        // OSD tombé : Ceph recopie ailleurs, le cluster reste utilisable, mais
        // la redondance fond et le prochain incident coûte des données.
        Rule {
            description: "A Ceph OSD has been down for five minutes.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "pve_ceph_osd_down",
                "Ceph OSD down",
                RuleKind::Threshold,
                "1 - dumbmonit_proxmox_ceph_osd_up",
            )
        },
        // Un seul OSD à 85 % suffit à bloquer les écritures de tous les pools
        // qui s'appuient dessus : la moyenne du cluster ne dit rien de ce
        // risque-là.
        Rule {
            description: "A Ceph OSD is more than 85% full: writes stop when it reaches its limit."
                .to_string(),
            operator: Operator::Gt,
            threshold: 85.0,
            clear_threshold: Some(82.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pve_ceph_osd_nearly_full",
                "Ceph OSD nearly full",
                RuleKind::Threshold,
                "dumbmonit_proxmox_ceph_osd_used_percent",
            )
        },
        Rule {
            description: "A Ceph pool is more than 85% full.".to_string(),
            operator: Operator::Gt,
            threshold: 85.0,
            clear_threshold: Some(82.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pve_ceph_pool_almost_full",
                "Ceph pool almost full",
                RuleKind::Threshold,
                "dumbmonit_proxmox_ceph_pool_used_percent",
            )
        },
        // `noout` posé le temps d'un redémarrage puis oublié : Ceph ne sortira
        // plus jamais un OSD mort du cluster, et la santé reste au vert pendant
        // que la redondance disparaît. Deux heures, c'est plus long que toute
        // maintenance normale.
        Rule {
            description: "The Ceph noout flag has been set for two hours: rebalancing is disabled."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(2 * 3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pve_ceph_noout_set",
                "Ceph noout flag left on",
                RuleKind::Threshold,
                "dumbmonit_proxmox_ceph_flag{flag=\"noout\"}",
            )
        },
        // Un LRM qui n'écrit plus n'exécute plus rien : les machines en haute
        // disponibilité de ce nœud ne seront ni relancées ni déplacées, et rien
        // d'autre ne le dit.
        Rule {
            description: "The HA local resource manager of a node has stopped reporting."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "pve_ha_lrm_stale",
                "HA manager not reporting",
                RuleKind::Threshold,
                "dumbmonit_proxmox_ha_lrm_stale",
            )
        },
        // Sauvegarde qui réussit tous les soirs sans rien contenir : un disque
        // porte `backup=0` et personne ne le sait avant d'avoir à restaurer.
        // Les exclusions normales (lecteur de CD, cloudinit) ne comptent pas.
        Rule {
            description: "A backup job skips a disk of one of the guests it covers.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pve_backup_excludes_disk",
                "Backup job excludes a disk",
                RuleKind::Threshold,
                "dumbmonit_proxmox_backup_job_guest_excluded_volumes",
            )
        },
        // Verrou oublié : une sauvegarde interrompue laisse la machine
        // verrouillée, et plus rien ne peut être fait dessus — pas même la
        // sauvegarde du lendemain. Six heures dépassent toute opération normale.
        Rule {
            description: "A guest has been locked for six hours: no operation can run on it."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(6 * 3600),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pve_guest_locked",
                "Guest locked",
                RuleKind::Threshold,
                "dumbmonit_proxmox_guest_locked",
            )
        },
        // Proxmox Backup Server : synchronisation vers un site distant en échec.
        // La série vaut 1 (dernier passage réussi) ou 0 ; « < 1 » ne vise que les
        // échecs, comme pour la vérification.
        Rule {
            description: "The last run of this PBS sync job failed.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pbs_sync_failed",
                "PBS sync job failed",
                RuleKind::Threshold,
                "dumbmonit_pbs_sync_job_last_ok",
            )
        },
        Rule {
            description: "More than twenty package updates are pending on the backup server."
                .to_string(),
            operator: Operator::Gt,
            threshold: 20.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pbs_updates_pending",
                "PBS updates pending",
                RuleKind::Threshold,
                "dumbmonit_pbs_node_updates_pending",
            )
        },
        // --- Proxmox Backup Server : travaux, GC, disques (`collectors/pbs`) ---
        //
        // Un travail de purge ou de vérification en échec n'empêche pas les
        // sauvegardes de tourner : c'est justement pourquoi personne ne le voit
        // avant que le datastore soit plein ou qu'une restauration échoue. Les
        // séries `job_last_ok` valent 1 ou 0 et n'existent pas pour un travail
        // qui n'a jamais tourné ; « < 1 » ne vise donc que les vrais échecs. Le
        // `for` de dix minutes absorbe une lecture prise pendant la relance.
        Rule {
            description: "The last run of this PBS prune job failed: nothing is pruned and the \
                          datastore keeps filling up."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pbs_prune_failed",
                "PBS prune job failed",
                RuleKind::Threshold,
                "dumbmonit_pbs_job_last_ok{kind=\"prune\"}",
            )
        },
        Rule {
            description: "The last run of this PBS verification job failed: at least one \
                          snapshot could not be verified."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pbs_verify_job_failed",
                "PBS verification job failed",
                RuleKind::Threshold,
                "dumbmonit_pbs_job_last_ok{kind=\"verify\"}",
            )
        },
        // `gc_last_run_ok` vient de `/gc` (PBS 3.3+) ou, avant, de la GC la plus
        // récente de la fenêtre de tâches.
        Rule {
            description: "The last garbage collection on this PBS datastore failed: freed \
                          space is not reclaimed."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pbs_gc_failed",
                "PBS garbage collection failed",
                RuleKind::Threshold,
                "dumbmonit_pbs_gc_last_run_ok",
            )
        },
        // Disques du serveur de sauvegarde, mêmes séries que pour un nœud PVE :
        // un SMART en échec est le dernier avertissement avant de perdre le
        // datastore avec les sauvegardes qu'il porte. La série n'existe que si
        // SMART a rendu un verdict.
        Rule {
            description: "A disk of the backup server reports SMART health FAILED.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pbs_disk_smart_failed",
                "PBS disk SMART failure",
                RuleKind::Threshold,
                "dumbmonit_pbs_node_disk_smart_failed",
            )
        },
        Rule {
            description: "This SSD of the backup server has used more than 90% of its rated \
                          endurance."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pbs_disk_wearout",
                "PBS SSD worn out",
                RuleKind::Threshold,
                "dumbmonit_pbs_node_disk_wearout_percent",
            )
        },
        // `zfs_pool_degraded` vaut 0 pour ONLINE ; DEGRADED, FAULTED, OFFLINE
        // ou UNAVAIL donnent 1. Un pool dégradé fonctionne encore : c'est le
        // moment de remplacer le disque, pas après le second.
        Rule {
            description: "A ZFS pool of the backup server is not ONLINE (degraded, faulted or \
                          unavailable)."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pbs_zpool_degraded",
                "PBS ZFS pool degraded",
                RuleKind::Threshold,
                "dumbmonit_pbs_node_zfs_pool_degraded",
            )
        },
        // Une unité arrêtée sur le serveur de sauvegarde : sans le mandataire,
        // plus aucune sauvegarde n'entre, et l'interface reste pourtant
        // joignable tant que l'API tourne. La série porte `expected = "1"` pour
        // les seules unités indispensables ; les autres — postfix, un agent de
        // journalisation — produisent leur série sans réveiller personne.
        Rule {
            description: "A service that Proxmox Backup Server needs is not running: no backup \
                          can be taken in while it is down."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pbs_service_down",
                "PBS service down",
                RuleKind::Threshold,
                "dumbmonit_pbs_node_service_active{expected=\"1\"}",
            )
        },
        // Le certificat de l'interface. La série n'existe que si l'option est
        // ouverte : PBS garde cette lecture derrière un privilège d'écriture.
        // Trois semaines laissent le temps de renouveler à la main un
        // certificat qu'ACME ne gère pas.
        Rule {
            description: "The certificate served by the backup server expires in less than \
                          three weeks."
                .to_string(),
            operator: Operator::Lt,
            threshold: 21.0 * 24.0 * 3600.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(48 * 3600)),
            ..base(
                "pbs_certificate_expiring",
                "PBS certificate expiring",
                RuleKind::Threshold,
                "dumbmonit_pbs_node_certificate_expires_seconds",
            )
        },
        // Le paquet a été mis à niveau, le démon tourne encore sur l'ancien
        // code. Rien ne casse tout de suite, et c'est le problème : le correctif
        // que l'on croit appliqué ne l'est pas.
        Rule {
            description: "The Proxmox Backup Server package has been upgraded but the running \
                          daemon is still the old one: restart its services to apply it."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(6 * 3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pbs_version_stale",
                "PBS restart pending after upgrade",
                RuleKind::Threshold,
                "dumbmonit_pbs_node_running_version_stale",
            )
        },
        // Une bande est la copie que rien en ligne ne peut atteindre, et
        // personne ne regarde une bandothèque : un travail en échec y reste des
        // semaines. `last_ok` vaut 1 ou 0, et n'existe pas avant le premier
        // passage.
        Rule {
            description: "The last tape backup run failed: the offline copy is not being made."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pbs_tape_job_failed",
                "PBS tape backup failed",
                RuleKind::Threshold,
                "dumbmonit_pbs_tape_backup_job_last_ok",
            )
        },
        // Un travail activé, planifié, et qui n'a jamais tourné : ni échec ni
        // succès, donc invisible partout ailleurs. Deux jours laissent passer un
        // travail créé hier soir pour une exécution hebdomadaire.
        Rule {
            description: "This PBS job is enabled and scheduled but has never run: check the \
                          schedule, or that the service that triggers it is up."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(2 * 24 * 3600),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pbs_job_never_run",
                "PBS job never ran",
                RuleKind::Threshold,
                "dumbmonit_pbs_job_never_run",
            )
        },
        // Des chunks que la GC a trouvés illisibles et laissés en place. Ce
        // n'est pas de la place à reprendre : ce sont des sauvegardes que l'on
        // ne pourra pas restaurer.
        Rule {
            description: "The garbage collection found unreadable chunks on this datastore: \
                          some backups can no longer be restored."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pbs_gc_bad_chunks",
                "PBS corrupt chunks found",
                RuleKind::Threshold,
                "dumbmonit_pbs_gc_bad_chunks",
            )
        },
        // Un datastore amovible débranché ne rend pas d'erreur : il est
        // simplement absent, et la sauvegarde de ce soir n'aura pas lieu. Six
        // heures laissent passer un disque de rotation que l'on emporte la
        // journée.
        Rule {
            description: "A removable datastore of the backup server is not mounted: nothing \
                          can be written to it."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(6 * 3600),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pbs_datastore_unmounted",
                "PBS datastore not mounted",
                RuleKind::Threshold,
                "dumbmonit_pbs_datastore_removable_unmounted",
            )
        },
        // --- fin du bloc Proxmox Backup Server ---
        // --- Proxmox Datacenter Manager : parc fédéré (`collectors/pdm`) ---
        //
        // Une console de datacenter ne se surveille pas pour elle-même : elle se
        // surveille pour les instances qu'elle fédère. La série vaut 1 (la console
        // a obtenu une réponse) ou 0, et porte `remote` et `type` : la notification
        // dit « site-b (pve) ».
        Rule {
            description: "The datacenter console cannot reach this Proxmox VE cluster or backup \
                          server."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pdm_remote_unreachable",
                "Federated instance unreachable",
                RuleKind::Threshold,
                "dumbmonit_pdm_remote_reachable",
            )
        },
        // La série n'existe que pour une instance dont la version a été lue, et ne
        // compare que des instances du même produit entre elles : c'est le cluster
        // que l'on a oublié de mettre à jour, pas un retard sur l'amont.
        Rule {
            description: "This instance runs an older version than another instance of the same \
                          product in the estate."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pdm_remote_version_behind",
                "Federated instance behind",
                RuleKind::Threshold,
                "dumbmonit_pdm_remote_version_behind",
            )
        },
        Rule {
            description: "At least one task failed on this federated instance in the review \
                          window."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pdm_task_failed",
                "Task failed on a federated instance",
                RuleKind::Threshold,
                "dumbmonit_pdm_remote_tasks_failed",
            )
        },
        // Le disque racine de la console porte son cache de métriques : plein, la
        // console cesse d'enregistrer ce qu'elle voit et n'a plus rien à montrer.
        Rule {
            description: "The root filesystem of the datacenter console is more than 90% full."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            escalate_after: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pdm_node_disk_almost_full",
                "Datacenter console disk almost full",
                RuleKind::Threshold,
                "dumbmonit_pdm_node_rootfs_percent",
            )
        },
        Rule {
            description: "A certificate of the datacenter console expires in less than fourteen \
                          days."
                .to_string(),
            operator: Operator::Lt,
            threshold: 14.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "d".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pdm_certificate_expiring",
                "Datacenter console certificate expiring",
                RuleKind::Threshold,
                "dumbmonit_pdm_node_certificate_expiry_days",
            )
        },
        Rule {
            description: "More than twenty package updates are pending on the datacenter console."
                .to_string(),
            operator: Operator::Gt,
            threshold: 20.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pdm_updates_pending",
                "Datacenter console updates pending",
                RuleKind::Threshold,
                "dumbmonit_pdm_node_updates_pending",
            )
        },
        // --- fin du bloc Proxmox Datacenter Manager ---
        // --- Proxmox Mail Gateway : files, filtrage, signatures (`collectors/pmg`) ---
        //
        // Une passerelle de messagerie tombe rarement d'un coup : elle filtre de
        // moins en moins bien, ou elle garde le courrier sans le dire. Les règles
        // qui suivent visent ces pannes silencieuses avant la panne franche.
        //
        // La file différée est le premier symptôme d'un relais aval injoignable :
        // elle grossit alors qu'aucune série ne bouge par ailleurs. On mesure la
        // croissance et non le niveau, parce qu'une passerelle chargée garde en
        // permanence quelques dizaines de messages différés sans que rien n'aille
        // mal.
        Rule {
            description: "The deferred mail queue has grown by more than a hundred messages in \
                          two hours: the next hop is probably refusing mail."
                .to_string(),
            operator: Operator::Gt,
            threshold: 100.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pmg_queue_growing",
                "Mail queue growing",
                RuleKind::Threshold,
                "delta(dumbmonit_pmg_queue_messages{queue=\"deferred\"}[2h])",
            )
        },
        // `queue_oldest_age_seconds` est un minorant : `qshape` ne donne que des
        // tranches, et on retient la borne basse de la plus haute tranche occupée.
        // Quatre heures dépassent largement les retentatives normales de Postfix.
        Rule {
            description: "A message has been waiting in this queue for more than four hours: mail \
                          is stuck, not slow."
                .to_string(),
            operator: Operator::Gt,
            threshold: 4.0 * 3600.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Critical,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pmg_queue_stuck",
                "Mail stuck in the queue",
                RuleKind::Threshold,
                "dumbmonit_pmg_queue_oldest_age_seconds",
            )
        },
        // Les unités qui font le travail : sans `pmg-smtp-filter`, Postfix accepte
        // le courrier et ne le filtre plus ; sans `postfix`, il ne l'accepte même
        // plus. La série vaut 1 ou 0 et n'existe pas pour une unité non installée :
        // « < 1 » ne vise que les vrais arrêts. Le `for` absorbe un redémarrage.
        Rule {
            description: "A service the gateway needs to accept and filter mail is stopped."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pmg_filter_service_down",
                "Mail gateway service stopped",
                RuleKind::Threshold,
                "dumbmonit_pmg_service_running{service=~\"postfix|pmg-smtp-filter|pmgpolicy|pmgproxy|pmgdaemon\"}",
            )
        },
        // Signatures antivirus : ClamAV publie plusieurs fois par jour. Deux jours
        // sans mise à jour veut dire que `freshclam` ne tourne plus — la passerelle
        // continue de filtrer, avec les virus de l'avant-veille. La règle ne vise
        // que la base quotidienne : `main` date de plusieurs mois par construction.
        Rule {
            description: "The ClamAV daily virus signatures are more than two days old: freshclam \
                          has stopped updating them."
                .to_string(),
            operator: Operator::Gt,
            threshold: 2.0 * 24.0 * 3600.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pmg_virus_signatures_stale",
                "Virus signatures out of date",
                RuleKind::Threshold,
                "dumbmonit_pmg_signature_age_seconds{family=\"virus\",database=\"daily\"}",
            )
        },
        // Règles antispam : `sa-update` publie environ une fois par semaine. Huit
        // jours laissent passer une publication décalée sans crier. La série
        // n'existe pas pour un canal jamais daté, donc un canal secondaire
        // inactif ne réveille personne.
        Rule {
            description: "The SpamAssassin rules are more than eight days old: sa-update has \
                          stopped bringing new ones."
                .to_string(),
            operator: Operator::Gt,
            threshold: 8.0 * 24.0 * 3600.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pmg_spam_rules_stale",
                "Spam rules out of date",
                RuleKind::Threshold,
                "dumbmonit_pmg_signature_age_seconds{family=\"spam\"}",
            )
        },
        // Quarantaine : c'est la croissance qui alerte, pas la taille. Une
        // quarantaine bien réglée garde des milliers de messages en régime
        // normal ; mille de plus en six heures signale une campagne, ou une règle
        // qui vient de basculer tout un domaine légitime en indésirable.
        Rule {
            description: "The quarantine has taken in more than a thousand messages in six \
                          hours: a campaign, or a rule that just went wrong."
                .to_string(),
            operator: Operator::Gt,
            threshold: 1000.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "pmg_quarantine_growing",
                "Quarantine filling up",
                RuleKind::Threshold,
                "delta(dumbmonit_pmg_quarantine_messages{kind=\"spam\"}[6h])",
            )
        },
        // Grappe : `cluster_node_insync` vaut 0 quand la base de règles d'un nœud
        // n'a pas reçu les dernières modifications. Un nœud désynchronisé filtre
        // avec des règles périmées, sans rien signaler de lui-même. La série
        // n'existe pas sur une installation autonome.
        Rule {
            description: "A gateway of the cluster is no longer in sync with the others: it \
                          filters with an out-of-date rule database."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pmg_cluster_degraded",
                "Mail gateway cluster degraded",
                RuleKind::Threshold,
                "dumbmonit_pmg_cluster_node_insync",
            )
        },
        // Le certificat que sert l'interface — et, avec la même paire, le TLS
        // entrant. Quatorze jours laissent le temps de renouveler à la main un
        // certificat qu'ACME n'a pas repris.
        Rule {
            description: "A certificate of the mail gateway expires in less than fourteen days."
                .to_string(),
            operator: Operator::Lt,
            threshold: 14.0 * 24.0 * 3600.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pmg_certificate_expiring",
                "Mail gateway certificate expiring",
                RuleKind::Threshold,
                "dumbmonit_pmg_certificate_expires_in_seconds",
            )
        },
        Rule {
            description: "More than twenty package updates are pending on the mail gateway."
                .to_string(),
            operator: Operator::Gt,
            threshold: 20.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pmg_updates_pending",
                "Mail gateway updates pending",
                RuleKind::Threshold,
                "dumbmonit_pmg_node_updates_pending",
            )
        },
        // --- fin du bloc Proxmox Mail Gateway ---
        // --- MDaemon Email Server et SecurityGateway (`collectors/mdaemon`) ---
        //
        // Ce qui compte d'abord pour un serveur de messagerie : que ses ports
        // acceptent le courrier. `service_up` vaut 0 quand le port ne répond pas
        // ou que la bannière refuse le service (`421`, `-ERR`, `* BYE`) ; seuls
        // les services listés sur l'équipement ont une série. Le `for` absorbe
        // un redémarrage du service Windows.
        Rule {
            description: "A mail service of the MDaemon server does not answer, or refuses \
                          service."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "mdaemon_service_down",
                "MDaemon mail service down",
                RuleKind::Threshold,
                "dumbmonit_mdaemon_service_up",
            )
        },
        // L'API XML ne répond plus alors que la collecte, elle, réussit : les
        // ports de messagerie répondent, c'est l'administration à distance qui
        // est arrêtée. La série n'existe que si un compte est configuré.
        Rule {
            description: "The MDaemon XML API has stopped answering while the mail services \
                          still do: Remote Administration is probably stopped."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "mdaemon_api_down",
                "MDaemon XML API not answering",
                RuleKind::Threshold,
                "dumbmonit_mdaemon_api_up",
            )
        },
        // Files d'attente de MDaemon : elles ne sont publiées que comme compteurs
        // de performance Windows, que l'agent lit sur le serveur lui-même
        // (`crates/agent/src/collect/perf_counters.rs`). Les séries portent donc
        // la cible de l'agent, pas celle de l'équipement `mdaemon`, et
        // n'existent que sur une machine où tourne MDaemon.
        //
        // Croissance et non niveau, comme pour PMG : un serveur chargé garde en
        // permanence quelques messages en transit. Les files entrante, locale et
        // distante sont celles que MDaemon vide d'elle-même ; une file qui
        // grossit de cent messages en deux heures ne se vide plus.
        Rule {
            description: "An MDaemon mail queue has grown by more than a hundred messages in two \
                          hours: MDaemon has stopped delivering from it. Read from the Windows \
                          performance counters by the agent installed on the mail server."
                .to_string(),
            operator: Operator::Gt,
            threshold: 100.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "mdaemon_queue_growing",
                "MDaemon mail queue growing",
                RuleKind::Threshold,
                "delta(dumbmonit_mdaemon_queue_messages{queue=~\"inbound|local|remote\"}[2h])",
            )
        },
        // La file « Bad » reçoit ce que MDaemon n'a pas su traiter ni renvoyer :
        // rien n'en sort sans un administrateur. Un seul message suffit à le
        // prévenir ; l'attente absorbe un message que quelqu'un traite déjà.
        Rule {
            description: "Messages are waiting in the MDaemon Bad queue: MDaemon could neither \
                          deliver nor bounce them, and nothing leaves that queue unless an \
                          administrator releases or deletes it."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "mdaemon_bad_queue_not_empty",
                "MDaemon Bad queue not empty",
                RuleKind::Threshold,
                "dumbmonit_mdaemon_queue_messages{queue=\"bad\"}",
            )
        },
        // La file de nouvelle tentative garde ce que les serveurs distants ont
        // refusé temporairement. Quelques messages y passent chaque jour ; plus
        // de cinquante pendant une heure, c'est un voisin qui refuse tout, ou
        // une réputation d'expéditeur à vérifier.
        Rule {
            description: "More than fifty messages have been waiting in the MDaemon Retry queue \
                          for an hour: remote servers keep refusing mail temporarily."
                .to_string(),
            operator: Operator::Gt,
            threshold: 50.0,
            for_duration: Duration::from_secs(60 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "mdaemon_retry_queue_high",
                "MDaemon Retry queue high",
                RuleKind::Threshold,
                "dumbmonit_mdaemon_queue_messages{queue=\"retry\"}",
            )
        },
        Rule {
            description: "A service of the SecurityGateway mail gateway does not answer, or \
                          refuses service."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "securitygateway_service_down",
                "SecurityGateway service down",
                RuleKind::Threshold,
                "dumbmonit_securitygateway_service_up",
            )
        },
        Rule {
            description: "The SecurityGateway REST API has stopped answering while the mail \
                          services still do."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "securitygateway_api_down",
                "SecurityGateway API not answering",
                RuleKind::Threshold,
                "dumbmonit_securitygateway_api_up",
            )
        },
        // La file de remise est le premier symptôme d'un serveur aval qui refuse
        // le courrier. Les noms des compteurs viennent tels quels de l'API : la
        // règle vise ceux qui parlent de la file de remise, sous les deux formes
        // que prend ce nom dans l'aide (« Delivery Queue », « Queued for
        // Delivery »). Croissance et non niveau, comme pour PMG.
        Rule {
            description: "The SecurityGateway delivery queue has grown by more than a hundred \
                          messages in two hours: the mail server behind it is probably \
                          refusing mail."
                .to_string(),
            operator: Operator::Gt,
            threshold: 100.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "securitygateway_queue_growing",
                "SecurityGateway delivery queue growing",
                RuleKind::Threshold,
                "delta(dumbmonit_securitygateway_counter{counter=~\".*(delivery_queue|queued_for_delivery).*\"}[2h])",
            )
        },
        // --- fin du bloc MDaemon ---
        // --- OPNsense : passerelles, pare-feu, tunnels (`collectors/opnsense`) ---
        //
        // Un pare-feu tombe rarement d'un bloc : il perd un lien, sature sa table
        // d'états ou cesse de résoudre, et tout le monde continue à le croire en
        // bonne santé parce qu'il répond toujours au ping. Les règles qui suivent
        // visent ces pannes-là.
        //
        // La bascule multi-WAN est la première : le lien de secours a pris le
        // relais il y a trois semaines, la facture mobile explose et personne ne
        // le sait. `gateway_up` vaut 0 sur « down » et « force_down » seulement :
        // « loss » et « delay » sont des avertissements de dpinger, pas des pannes.
        Rule {
            description: "A gateway is down: dpinger no longer gets an answer from it. On a \
                          multi-WAN firewall the traffic has already moved to another link, \
                          quietly."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "opnsense_gateway_down",
                "Internet gateway down",
                RuleKind::Threshold,
                "dumbmonit_opnsense_gateway_up",
            )
        },
        // Perte de paquets : une passerelle qui répond encore mais perd un
        // cinquième de ce qu'on lui envoie rend les appels inaudibles et les
        // sessions instables, sans jamais être déclarée tombée.
        Rule {
            description: "A gateway is losing more than a fifth of the packets sent to it: the \
                          link answers, badly."
                .to_string(),
            operator: Operator::Gt,
            threshold: 20.0,
            clear_threshold: Some(10.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "opnsense_gateway_loss",
                "Gateway losing packets",
                RuleKind::Threshold,
                "dumbmonit_opnsense_gateway_loss_percent",
            )
        },
        // Latence : trois cents millisecondes tiennent compte d'un lien mobile de
        // secours, qui n'a aucune raison de déclencher une alerte pour sa nature.
        Rule {
            description: "A gateway's round-trip time has stayed above 300 ms for a quarter of \
                          an hour."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.3,
            clear_threshold: Some(0.2),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Info,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "opnsense_gateway_latency",
                "Gateway slow",
                RuleKind::Threshold,
                "dumbmonit_opnsense_gateway_delay_seconds",
            )
        },
        // Table d'états : elle se remplit longtemps avant de déborder, et quand
        // elle déborde le pare-feu refuse des connexions neuves sans qu'aucune
        // autre série ne bouge.
        Rule {
            description: "The firewall's state table is more than 80% full. Past the limit it \
                          drops new connections without any other symptom."
                .to_string(),
            operator: Operator::Gt,
            threshold: 80.0,
            clear_threshold: Some(70.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "opnsense_state_table_filling",
                "Firewall state table filling up",
                RuleKind::Threshold,
                "dumbmonit_opnsense_pf_states_used_percent",
            )
        },
        // Tampons réseau de FreeBSD : les épuiser arrête le routage alors que le
        // processeur et la mémoire restent au repos. C'est la panne qui ne
        // ressemble à rien, et elle a un seuil.
        Rule {
            description: "The firewall has used more than 90% of its network buffers. FreeBSD \
                          stops forwarding when they run out, with the CPU still idle."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(80.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "opnsense_mbuf_exhausted",
                "Firewall network buffers exhausted",
                RuleKind::Threshold,
                "dumbmonit_opnsense_mbuf_used_percent",
            )
        },
        // Tunnel VPN : la série n'existe que pour un tunnel configuré et activé.
        // Un greffon absent ou une connexion désactivée n'en produisent aucune,
        // et ne peuvent donc pas déclencher.
        Rule {
            description: "A configured VPN tunnel has had no session for ten minutes.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "opnsense_vpn_tunnel_down",
                "VPN tunnel down",
                RuleKind::Threshold,
                "dumbmonit_opnsense_vpn_tunnel_up",
            )
        },
        // Le résolveur : un pare-feu qui route mais ne résout plus paraît en
        // bonne santé à tout le monde sauf aux machines derrière lui. La série
        // n'existe que si Unbound est installé ; un pare-feu qui résout avec
        // autre chose n'a qu'à désactiver cette règle.
        Rule {
            description: "Unbound has stopped answering. The firewall still routes; the machines \
                          behind it no longer resolve."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "opnsense_resolver_down",
                "Firewall resolver stopped",
                RuleKind::Threshold,
                "dumbmonit_opnsense_unbound_running",
            )
        },
        // Services du cœur : `dpinger` est ce qui surveille les passerelles et
        // `configd` ce qui exécute tout le reste. Les deux tournent sur toute
        // installation ; les autres services sont volontairement hors de la
        // règle, parce qu'un service arrêté exprès n'est pas une panne.
        Rule {
            description: "A core service of the firewall is stopped: dpinger, which watches the \
                          gateways, or configd, which runs everything else."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "opnsense_core_service_down",
                "Firewall core service stopped",
                RuleKind::Threshold,
                "dumbmonit_opnsense_service_running{service=~\"dpinger|configd\"}",
            )
        },
        // Température : un boîtier de pare-feu vit dans un placard, souvent sans
        // ventilateur. Quatre-vingt-cinq degrés est la limite au-delà de laquelle
        // les processeurs embarqués commencent à se brider.
        Rule {
            description: "A sensor of the firewall is above 85 °C: the fanless box in the \
                          cupboard is about to start throttling."
                .to_string(),
            operator: Operator::Gt,
            threshold: 85.0,
            clear_threshold: Some(78.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "°C".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "opnsense_temperature_high",
                "Firewall too hot",
                RuleKind::Threshold,
                "dumbmonit_opnsense_temperature_celsius",
            )
        },
        // CARP en mode maintenance : la bascule a été demandée à la main pour une
        // mise à jour, et jamais annulée. Le pare-feu reste secondaire pour
        // toujours, ce qui ne se voit nulle part ailleurs.
        Rule {
            description: "CARP has been left in persistent maintenance mode: this firewall has \
                          handed its virtual addresses to its partner and will not take them \
                          back on its own."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "opnsense_carp_maintenance",
                "Firewall left in CARP maintenance mode",
                RuleKind::Threshold,
                "dumbmonit_opnsense_carp_maintenance_mode",
            )
        },
        // Mise à jour posée mais pas démarrée : le noyau ou un micrologiciel
        // attend un redémarrage, et le pare-feu tourne encore sur l'ancien.
        Rule {
            description: "An update is installed on the firewall but needs a reboot to take \
                          effect."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "opnsense_reboot_pending",
                "Firewall reboot pending",
                RuleKind::Threshold,
                "dumbmonit_opnsense_firmware_reboot_required",
            )
        },
        // Mises à jour en attente : un pare-feu est la machine la plus exposée du
        // réseau, et c'est la seule dont les mises à jour comptent vraiment.
        Rule {
            description: "The firewall has packages waiting to be updated. It is the most \
                          exposed machine on the network."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "opnsense_updates_pending",
                "Firewall updates pending",
                RuleKind::Threshold,
                "dumbmonit_opnsense_firmware_updates_pending",
            )
        },
        // --- fin du bloc OPNsense ---
        // --- TrueNAS : pools, disques, protection des données (`collectors/truenas`) ---
        //
        // La panne pour laquelle ce bloc existe : un vdev redondant qui a perdu un
        // disque sert toujours les données. Le partage reste monté, les
        // sauvegardes passent, et personne ne le voit avant le deuxième disque.
        // Les règles qui suivent visent ces pannes-là, avant la panne franche.
        //
        // `pool_healthy` est l'avis de ZFS lui-même : il tombe à 0 sur un pool
        // DEGRADED comme sur un pool FAULTED. Cinq minutes absorbent un disque
        // qu'on remplace à chaud.
        Rule {
            description: "A pool is no longer healthy. A degraded pool still serves its data, which \
                          is exactly why nobody notices until the next disk fails."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "truenas_pool_degraded",
                "NAS pool degraded",
                RuleKind::Threshold,
                "dumbmonit_truenas_pool_healthy",
            )
        },
        // Erreurs de lecture, d'écriture ou de somme de contrôle : ZFS les a
        // corrigées tant qu'il avait de la redondance, mais un disque qui en
        // accumule est en train de partir. Elles restent jusqu'au `zpool clear`.
        Rule {
            description: "Disks in this pool have reported read, write or checksum errors. ZFS \
                          repaired what it could; a disk that keeps doing this is on its way out."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "truenas_pool_device_errors",
                "NAS disk errors",
                RuleKind::Threshold,
                "dumbmonit_truenas_pool_device_errors",
            )
        },
        // Un pool ZFS ralentit nettement passé 80 % et se fragmente ; à 90 % il
        // devient pénible, à 100 % il refuse d'écrire.
        Rule {
            description: "A pool is more than 85% full. ZFS slows down well before it is full."
                .to_string(),
            operator: Operator::Gt,
            threshold: 85.0,
            clear_threshold: Some(80.0),
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            escalate_after: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "truenas_pool_almost_full",
                "NAS pool almost full",
                RuleKind::Threshold,
                "dumbmonit_truenas_pool_used_percent",
            )
        },
        // Une vérification qui trouve des erreurs a trouvé des données abîmées ;
        // elle les a réparées si la redondance le permettait, pas sinon.
        Rule {
            description: "The last scrub of this pool found errors: data on disk was damaged."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "truenas_scrub_errors",
                "NAS scrub found errors",
                RuleKind::Threshold,
                "dumbmonit_truenas_pool_last_scrub_errors",
            )
        },
        // Sans vérification, une corruption silencieuse n'est découverte qu'à la
        // lecture — souvent le jour de la restauration. TrueNAS en planifie une
        // tous les 35 jours par défaut : 45 jours laissent passer un retard.
        Rule {
            description: "This pool has not completed a scrub in more than 45 days: silent \
                          corruption would only be found when the data is read."
                .to_string(),
            operator: Operator::Gt,
            threshold: 45.0 * 86_400.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "truenas_scrub_overdue",
                "NAS scrub overdue",
                RuleKind::Threshold,
                "dumbmonit_truenas_pool_last_scrub_age_seconds",
            )
        },
        // Le journal SMART garde les échecs : la série vaut 1 tant qu'un test
        // du journal a échoué, et n'existe pas pour un disque jamais testé.
        Rule {
            description: "A SMART self-test of this disk failed. Replace it before it takes the \
                          pool's redundancy with it."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "truenas_disk_smart_failed",
                "NAS disk failed its SMART test",
                RuleKind::Threshold,
                "dumbmonit_truenas_disk_smart_failed",
            )
        },
        Rule {
            description: "A disk of this NAS is above 55 °C.".to_string(),
            operator: Operator::Gt,
            threshold: 55.0,
            clear_threshold: Some(50.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "°C".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "truenas_disk_hot",
                "NAS disk too hot",
                RuleKind::Threshold,
                "dumbmonit_truenas_disk_temperature_celsius",
            )
        },
        // Un quota plein refuse les écritures de ce jeu de données seul, alors
        // que le pool a de la place : l'application qui écrit dedans tombe.
        Rule {
            description: "A dataset has used more than 90% of its quota. At 100% its writes fail \
                          while the pool still has room."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(85.0),
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "truenas_dataset_quota_full",
                "NAS dataset near its quota",
                RuleKind::Threshold,
                "dumbmonit_truenas_dataset_quota_used_percent",
            )
        },
        // Une réplication en échec laisse la copie distante vieillir sans rien
        // dire. La série n'existe que pour une tâche activée.
        Rule {
            description: "A replication task failed: the copy on the other side is getting older."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "truenas_replication_failed",
                "NAS replication failed",
                RuleKind::Threshold,
                "dumbmonit_truenas_replication_error",
            )
        },
        Rule {
            description: "A periodic snapshot task failed: no new snapshot is being taken."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "truenas_snapshot_task_failed",
                "NAS snapshot task failed",
                RuleKind::Threshold,
                "dumbmonit_truenas_snapshot_task_error",
            )
        },
        // Une tâche d'instantanés qui ne tourne plus ne tombe pas en erreur :
        // elle se tait. Huit jours laissent passer une tâche hebdomadaire.
        Rule {
            description: "A periodic snapshot task has not run for more than eight days."
                .to_string(),
            operator: Operator::Gt,
            threshold: 8.0 * 86_400.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "truenas_snapshots_stale",
                "NAS snapshots stale",
                RuleKind::Threshold,
                "dumbmonit_truenas_snapshot_task_last_run_age_seconds",
            )
        },
        // TrueNAS calcule lui-même l'état des pools, SMART, la capacité, les
        // certificats, l'onduleur : relayer ses alertes graves est le filet le
        // plus large. La série de chaque niveau existe toujours, zéro compris,
        // ce qui fait retomber la règle quand l'alerte disparaît.
        Rule {
            description: "TrueNAS has raised an alert of level ERROR or above. Its own sentence is \
                          on the device page."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "truenas_alert_raised",
                "TrueNAS alert raised",
                RuleKind::Threshold,
                "sum by (target, host) (dumbmonit_truenas_alerts{level=~\"ERROR|CRITICAL|ALERT|EMERGENCY\"})",
            )
        },
        // Seuls les services qui démarrent avec le NAS sont mesurés : un service
        // qu'on a éteint exprès n'a pas de série.
        Rule {
            description: "A service set to start with the NAS is stopped.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "truenas_service_down",
                "NAS service stopped",
                RuleKind::Threshold,
                "dumbmonit_truenas_service_running",
            )
        },
        // --- fin du bloc TrueNAS ---
        // --- Serveurs de journaux et de métriques (`collectors/observability`) ---
        //
        // Surveiller la surveillance. Ces serveurs tombent rarement tout à fait :
        // ils répondent, l'interface s'ouvre, et ils refusent en silence une partie
        // de ce qu'on leur envoie. Les règles visent ces refus-là. Les compteurs
        // passent par `increase_prometheus`, jamais `increase`, pour qu'un serveur
        // ajouté avec un historique de rejets ne sonne pas dès la première mesure.
        //
        // VictoriaMetrics et VictoriaLogs passent en lecture seule sous
        // `-storage.minFreeDiskSpaceBytes` d'espace libre : toute écriture est
        // refusée. C'est une panne franche de l'ingestion.
        Rule {
            description: "The storage switched to read-only because free disk space fell below \
                          -storage.minFreeDiskSpaceBytes: every new sample or log line is refused."
                .to_string(),
            operator: Operator::Ge,
            threshold: 1.0,
            for_duration: Duration::from_secs(2 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "victoria_read_only",
                "Metrics or log storage read-only",
                RuleKind::Threshold,
                "dumbmonit_victoriametrics_read_only or dumbmonit_victorialogs_read_only",
            )
        },
        // La marge avant la lecture seule, en pourcentage du disque : moins de
        // 10 % laisse le temps d'agrandir le volume ou de raccourcir la rétention.
        Rule {
            description: "Less than 10% of the disk is left before the storage switches to \
                          read-only and refuses new data."
                .to_string(),
            operator: Operator::Lt,
            threshold: 10.0,
            clear_threshold: Some(12.0),
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "victoria_disk_headroom_low",
                "Metrics or log storage almost read-only",
                RuleKind::Threshold,
                "dumbmonit_victoriametrics_disk_headroom_percent \
                 or dumbmonit_victorialogs_disk_headroom_percent",
            )
        },
        Rule {
            description: "The server does not answer OK on /health.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(3 * 60),
            severity: Severity::Critical,
            ..base(
                "victoria_unhealthy",
                "Metrics or log server unhealthy",
                RuleKind::Threshold,
                "dumbmonit_victoriametrics_healthy or dumbmonit_victorialogs_healthy",
            )
        },
        // Lignes refusées : horodatage hors rétention ou trop loin dans le futur,
        // étiquettes trop longues, limite de séries. Chacune est une donnée perdue.
        Rule {
            description: "The server refused samples or log lines in the last hour: timestamps \
                          outside retention, labels too long, or a series limit reached."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "victoria_rows_rejected",
                "Metrics or log data refused",
                RuleKind::Threshold,
                "increase_prometheus(dumbmonit_victoriametrics_rows_rejected_total[1h]) \
                 or increase_prometheus(dumbmonit_victorialogs_rows_rejected_total[1h])",
            )
        },
        // Le seuil que la documentation de VictoriaMetrics donne lui-même : au-delà
        // de 5 % d'insertions lentes, le cache des séries ne tient plus en mémoire.
        // Une demi-heure écarte la vague de séries neuves d'un déploiement.
        Rule {
            description: "More than 5% of new samples take the slow insert path: VictoriaMetrics \
                          lacks memory for its active series."
                .to_string(),
            operator: Operator::Gt,
            threshold: 5.0,
            clear_threshold: Some(3.0),
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "victoriametrics_slow_inserts",
                "VictoriaMetrics slow inserts",
                RuleKind::Threshold,
                "100 * increase_prometheus(dumbmonit_victoriametrics_slow_inserts_total[15m]) \
                 / increase_prometheus(dumbmonit_victoriametrics_rows_added_total[15m])",
            )
        },
        // Loki : `/ready` dit s'il accepte d'écrire et de lire. Cinq minutes
        // couvrent le démarrage d'un ingester qui rejoue son WAL.
        Rule {
            description: "Loki does not answer ready on /ready: it cannot accept or serve logs."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            ..base("loki_not_ready", "Loki not ready", RuleKind::Threshold, "dumbmonit_loki_ready")
        },
        Rule {
            description: "Loki refused log lines in the last hour: too old, rate or stream limit \
                          reached, or line too long. The sender drops them."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "loki_lines_discarded",
                "Loki refusing log lines",
                RuleKind::Threshold,
                "increase_prometheus(dumbmonit_loki_discarded_lines_total[1h])",
            )
        },
        // Un bloc qui ne part pas vers le stockage reste en mémoire : il est
        // perdu au prochain redémarrage.
        Rule {
            description: "Loki failed to write chunks to storage: they are held in memory and \
                          lost if Loki restarts."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "loki_flush_failing",
                "Loki cannot flush to storage",
                RuleKind::Threshold,
                "increase_prometheus(dumbmonit_loki_flush_failures_total[30m])",
            )
        },
        Rule {
            description: "Loki could not write to its write-ahead log because the disk is full."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "loki_wal_disk_full",
                "Loki write-ahead log disk full",
                RuleKind::Threshold,
                "increase_prometheus(dumbmonit_loki_wal_disk_full_failures_total[30m])",
            )
        },
        Rule {
            description: "Loki answered more than five requests with a server error in fifteen \
                          minutes."
                .to_string(),
            operator: Operator::Gt,
            threshold: 5.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "loki_request_errors",
                "Loki request errors",
                RuleKind::Threshold,
                "increase_prometheus(dumbmonit_loki_request_errors_total[15m])",
            )
        },
        // Graylog : la panne typique est OpenSearch qui ne suit plus. Graylog
        // continue d'accepter, range dans son journal, et les recherches ne
        // montrent plus rien de récent.
        Rule {
            description: "Graylog has paused message processing on this node: messages pile up \
                          in the journal."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            ..base(
                "graylog_not_processing",
                "Graylog not processing",
                RuleKind::Threshold,
                "dumbmonit_graylog_processing",
            )
        },
        // 2 rouge, 3 injoignable depuis Graylog.
        Rule {
            description: "Graylog's search cluster is red or unreachable: new messages cannot be \
                          indexed."
                .to_string(),
            operator: Operator::Ge,
            threshold: 2.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "graylog_search_cluster_down",
                "Graylog search cluster down",
                RuleKind::Threshold,
                "dumbmonit_graylog_indexer_status",
            )
        },
        Rule {
            description: "Graylog's search cluster has been yellow for half an hour: replica \
                          shards are not assigned, one more node lost would lose data."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "graylog_search_cluster_yellow",
                "Graylog search cluster yellow",
                RuleKind::Threshold,
                "dumbmonit_graylog_indexer_status == 1",
            )
        },
        // Graylog prévient lui-même vers 95 % ; à moitié plein, il reste de quoi
        // réparer la sortie avant de perdre des messages.
        Rule {
            description: "Graylog's journal is more than half full: the output does not keep up \
                          and messages will be dropped once it is full."
                .to_string(),
            operator: Operator::Gt,
            threshold: 50.0,
            clear_threshold: Some(40.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            escalate_after: Some(Duration::from_secs(3600)),
            ..base(
                "graylog_journal_filling",
                "Graylog journal filling",
                RuleKind::Threshold,
                "dumbmonit_graylog_journal_used_percent",
            )
        },
        Rule {
            description: "A Graylog input failed to start: whatever sends to it is not received."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "graylog_input_failed",
                "Graylog input failed",
                RuleKind::Threshold,
                "dumbmonit_graylog_inputs_failed",
            )
        },
        Rule {
            description: "Graylog failed to write messages to its search cluster in the last \
                          hour."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "graylog_indexing_failures",
                "Graylog indexing failures",
                RuleKind::Threshold,
                "increase_prometheus(dumbmonit_graylog_output_failures_total[1h]) \
                 + increase_prometheus(dumbmonit_graylog_processing_failures_total[1h])",
            )
        },
        // --- fin du bloc serveurs de journaux et de métriques ---
        // --- Applications auto-hébergées (`collectors/selfhosted`) ---
        //
        // Ces applications tombent de côté : la page s'ouvre, et ce qui compte
        // (la synchronisation, les miniatures, l'import de documents, le scan
        // de bibliothèque) ne se fait plus. Les règles visent ces états-là ;
        // l'application qui ne répond plus du tout relève de « injoignable ».
        Rule {
            description: "Nextcloud has been in maintenance mode for half an hour: nobody can \
                          log in or sync."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            escalate_after: Some(Duration::from_secs(3 * 3600)),
            ..base(
                "nextcloud_maintenance",
                "Nextcloud stuck in maintenance",
                RuleKind::Threshold,
                "dumbmonit_nextcloud_maintenance",
            )
        },
        // Après une mise à jour du code, Nextcloud refuse tout tant que
        // `occ upgrade` n'a pas tourné.
        Rule {
            description: "Nextcloud waits for occ upgrade after an update and serves nothing \
                          until someone runs it."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "nextcloud_upgrade_pending",
                "Nextcloud database upgrade pending",
                RuleKind::Threshold,
                "dumbmonit_nextcloud_needs_db_upgrade",
            )
        },
        Rule {
            description: "A newer Nextcloud release is available.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "nextcloud_update_available",
                "Nextcloud update available",
                RuleKind::Threshold,
                "dumbmonit_nextcloud_update_available",
            )
        },
        // OPcache plein : PHP recompile chaque script, tout devient lent.
        Rule {
            description: "PHP OPcache on the Nextcloud server is full: pages get slow. Raise \
                          opcache.memory_consumption."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "nextcloud_opcache_full",
                "Nextcloud OPcache full",
                RuleKind::Threshold,
                "dumbmonit_nextcloud_opcache_full",
            )
        },
        // Nextcloud ne donne que l'espace libre du dossier de données, pas sa
        // taille : un seuil absolu, en Gio.
        Rule {
            description: "Less than 5 GiB free for Nextcloud's data: uploads and syncs will \
                          start failing."
                .to_string(),
            operator: Operator::Lt,
            threshold: 5.0,
            clear_threshold: Some(6.0),
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "GiB".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "nextcloud_free_space_low",
                "Nextcloud free space low",
                RuleKind::Threshold,
                "dumbmonit_nextcloud_free_space_bytes / 1073741824",
            )
        },
        // Immich : des travaux en attente dans une file qui n'est pas en
        // pause et n'en exécute aucun. Une heure couvre les creux entre deux
        // lots d'un gros import.
        Rule {
            description: "Immich jobs have waited for an hour in a queue that is not paused and \
                          runs nothing: new photos get no thumbnail or metadata."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "immich_jobs_stalled",
                "Immich jobs stalled",
                RuleKind::Threshold,
                "dumbmonit_immich_queue_waiting > 0 \
                 and on (target, queue) dumbmonit_immich_queue_active == 0 \
                 unless on (target, queue) dumbmonit_immich_queue_paused == 1",
            )
        },
        Rule {
            description: "An Immich job queue has been paused for a day.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(24 * 3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "immich_queue_paused",
                "Immich queue paused",
                RuleKind::Threshold,
                "dumbmonit_immich_queue_paused",
            )
        },
        Rule {
            description: "The disk holding Immich's library is more than 90% full.".to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(88.0),
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "immich_storage_almost_full",
                "Immich storage almost full",
                RuleKind::Threshold,
                "dumbmonit_immich_storage_used_percent",
            )
        },
        Rule {
            description: "A newer Immich release is available.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "immich_update_available",
                "Immich update available",
                RuleKind::Threshold,
                "dumbmonit_immich_update_available",
            )
        },
        // Paperless : 0 OK, 1 avertissement, 2 erreur. Sans Redis ni Celery,
        // plus rien n'est importé.
        Rule {
            description: "Paperless-ngx cannot reach Redis or Celery: nothing new is imported, \
                          from the consume folder or from mail."
                .to_string(),
            operator: Operator::Ge,
            threshold: 2.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "paperless_broker_down",
                "Paperless task queue down",
                RuleKind::Threshold,
                "max by (target) (dumbmonit_paperless_component_status{component=~\"redis|celery\"})",
            )
        },
        // Le contrôle d'intégrité en erreur, c'est un fichier manquant ou
        // dont la somme ne correspond plus : à voir avant la prochaine sauvegarde.
        Rule {
            description: "Paperless-ngx reports an error in its search index, classifier or \
                          sanity check (missing or altered files)."
                .to_string(),
            operator: Operator::Ge,
            threshold: 2.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "paperless_component_error",
                "Paperless index or sanity check error",
                RuleKind::Threshold,
                "dumbmonit_paperless_component_status{component=~\"index|classifier|sanity_check\"}",
            )
        },
        Rule {
            description: "Paperless-ngx has database migrations left to apply: run them before \
                          it misbehaves."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "paperless_migrations_pending",
                "Paperless migrations pending",
                RuleKind::Threshold,
                "dumbmonit_paperless_unapplied_migrations",
            )
        },
        Rule {
            description: "A Paperless-ngx task (an import, a scheduled job) failed recently and \
                          was not dismissed."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "paperless_task_failed",
                "Paperless task failed",
                RuleKind::Threshold,
                "dumbmonit_paperless_tasks_failed_recent",
            )
        },
        Rule {
            description: "The disk holding Paperless-ngx's documents is more than 90% full."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(88.0),
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "paperless_storage_almost_full",
                "Paperless storage almost full",
                RuleKind::Threshold,
                "dumbmonit_paperless_storage_used_percent",
            )
        },
        Rule {
            description: "A newer Paperless-ngx release is available.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "paperless_update_available",
                "Paperless update available",
                RuleKind::Threshold,
                "dumbmonit_paperless_update_available",
            )
        },
        // Jellyfin : le scan de bibliothèque en échec est le cas typique.
        Rule {
            description: "A Jellyfin scheduled task failed on its last run (a library scan, a \
                          cleanup)."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "jellyfin_task_failed",
                "Jellyfin scheduled task failed",
                RuleKind::Threshold,
                "dumbmonit_jellyfin_scheduled_tasks_failed",
            )
        },
        Rule {
            description: "A Jellyfin plugin failed to load or does not support this version."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "jellyfin_plugin_broken",
                "Jellyfin plugin broken",
                RuleKind::Threshold,
                "dumbmonit_jellyfin_plugins_broken",
            )
        },
        Rule {
            description: "Jellyfin has waited a day for a restart, after a plugin install or \
                          update."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(24 * 3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "jellyfin_restart_pending",
                "Jellyfin restart pending",
                RuleKind::Threshold,
                "dumbmonit_jellyfin_pending_restart",
            )
        },
        Rule {
            description: "A newer Plex Media Server release is available.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "plex_update_available",
                "Plex update available",
                RuleKind::Threshold,
                "dumbmonit_plex_update_available",
            )
        },
        // --- fin du bloc applications auto-hébergées ---
        // --- Pi-hole (`collectors/pihole`) ---
        //
        // Le blocage se suspend d'un clic pour cinq minutes ou une heure :
        // quinze minutes laissent passer la pause courte, pas l'oubli.
        Rule {
            description: "Pi-hole has not been blocking for fifteen minutes: ads and trackers go \
                          through."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            ..base(
                "pihole_blocking_disabled",
                "Pi-hole blocking disabled",
                RuleKind::Threshold,
                "dumbmonit_pihole_blocking_enabled",
            )
        },
        // Gravity se reconstruit chaque semaine par défaut : huit jours
        // laissent passer une semaine normale, pas deux mises à jour manquées.
        Rule {
            description: "Pi-hole's blocklists have not been rebuilt for more than eight days: \
                          the weekly gravity update fails."
                .to_string(),
            operator: Operator::Gt,
            threshold: 8.0 * 86_400.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pihole_gravity_stale",
                "Pi-hole blocklists stale",
                RuleKind::Threshold,
                "dumbmonit_pihole_gravity_age_seconds",
            )
        },
        Rule {
            description: "A newer Pi-hole is available (the container image, or core, web \
                          interface or FTL outside a container)."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pihole_update_available",
                "Pi-hole update available",
                RuleKind::Threshold,
                "dumbmonit_pihole_updates_available",
            )
        },
        Rule {
            description: "Pi-hole lists messages in its diagnosis page: a blocklist that failed \
                          to download, a client rate-limited, a host overloaded."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pihole_diagnosis_messages",
                "Pi-hole diagnosis messages",
                RuleKind::Threshold,
                "dumbmonit_pihole_messages",
            )
        },
        // --- AdGuard Home (`collectors/adguard`) ---
        //
        // La protection coupée : « désactiver pour une heure » oublié, ou
        // désactivée tout court. Quinze minutes laissent passer la pause de
        // dix minutes qu'on prend pour débloquer un site.
        Rule {
            description: "AdGuard Home's protection has been off for fifteen minutes: nothing is \
                          filtered."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "adguard_protection_disabled",
                "AdGuard Home protection off",
                RuleKind::Threshold,
                "dumbmonit_adguard_protection_enabled",
            )
        },
        // Le serveur DNS arrêté alors que l'interface répond : plus aucune
        // résolution pour les clients qui n'ont que lui.
        Rule {
            description: "AdGuard Home's DNS server is not running: clients that use it cannot \
                          resolve names."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "adguard_dns_not_running",
                "AdGuard Home DNS not running",
                RuleKind::Threshold,
                "dumbmonit_adguard_running",
            )
        },
        // Un amont qui échoue au test d'AdGuard Home, ou reste muet quatre
        // secondes. Dix minutes écartent une perte de paquet isolée.
        Rule {
            description: "An upstream DNS server of AdGuard Home fails its test: queries sent to \
                          it fail or wait."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "adguard_upstream_down",
                "AdGuard Home upstream failing",
                RuleKind::Threshold,
                "dumbmonit_adguard_upstream_up",
            )
        },
        // Les listes se mettent à jour toutes les 24 h par défaut : trois
        // jours sans mise à jour, c'est que le téléchargement échoue.
        Rule {
            description: "An enabled AdGuard Home filter list has not been updated for three \
                          days: blocking falls behind."
                .to_string(),
            operator: Operator::Gt,
            threshold: 3.0 * 86_400.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "adguard_filters_stale",
                "AdGuard Home filter lists stale",
                RuleKind::Threshold,
                "dumbmonit_adguard_filter_oldest_update_age_seconds",
            )
        },
        Rule {
            description: "A newer AdGuard Home version is available.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "adguard_update_available",
                "AdGuard Home update available",
                RuleKind::Threshold,
                "dumbmonit_adguard_update_available",
            )
        },
        // --- Onduleurs derrière NUT (`collectors/nut`) ---
        //
        // Les séries portent `ups` : la notification dit quel onduleur d'un
        // même serveur NUT. Les drapeaux de `ups.status` valent 0 ou 1 et sont
        // émis à chaque lecture : les règles retombent d'elles-mêmes au retour
        // du secteur. Mêmes délais et gravités que les règles SNMP de l'UPS-MIB.
        Rule {
            description: "The UPS is powering the load from battery.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(900)),
            ..base(
                "nut_on_battery",
                "UPS on battery (NUT)",
                RuleKind::Threshold,
                "dumbmonit_nut_ups_on_battery",
            )
        },
        // `LB` : le seuil de l'onduleur ou de NUT (`battery.charge.low`,
        // `battery.runtime.low`) est franchi, upsmon déclenche l'arrêt.
        Rule {
            description: "The UPS reports a low battery: the machines it powers are about to \
                          shut down."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(1800)),
            ..base(
                "nut_battery_low",
                "UPS battery low (NUT)",
                RuleKind::Threshold,
                "dumbmonit_nut_ups_low_battery",
            )
        },
        // `RB` : l'onduleur a jugé la batterie usée, souvent après un autotest.
        Rule {
            description: "The UPS asks for its battery to be replaced: it may not hold the load \
                          through the next outage."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "nut_replace_battery",
                "UPS battery needs replacing (NUT)",
                RuleKind::Threshold,
                "dumbmonit_nut_ups_replace_battery",
            )
        },
        // Au-delà de 80 %, l'autonomie fond et un pic de démarrage peut
        // déclencher la surcharge.
        Rule {
            description: "The UPS load has stayed above 80% for ten minutes.".to_string(),
            operator: Operator::Gt,
            threshold: 80.0,
            clear_threshold: Some(75.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "nut_load_high",
                "UPS load high (NUT)",
                RuleKind::Threshold,
                "dumbmonit_nut_ups_load_percent",
            )
        },
        // L'autonomie estimée : sur secteur, moins de cinq minutes dit une
        // batterie usée ou un onduleur trop petit pour sa charge.
        Rule {
            description: "The UPS estimates less than five minutes of runtime on battery."
                .to_string(),
            operator: Operator::Lt,
            threshold: 300.0,
            clear_threshold: Some(360.0),
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "nut_runtime_short",
                "UPS runtime short (NUT)",
                RuleKind::Threshold,
                "dumbmonit_nut_battery_runtime_seconds",
            )
        },
        // `upsd` répond mais n'a plus de nouvelles de l'onduleur : câble USB,
        // pilote arrêté. Pendant ce temps, une coupure passerait inaperçue.
        Rule {
            description: "The NUT server has no fresh data from the UPS: its cable, driver or \
                          the UPS itself stopped answering, and an outage would go unseen."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            escalate_after: Some(Duration::from_secs(3600)),
            ..base(
                "nut_data_stale",
                "UPS data stale (NUT)",
                RuleKind::Threshold,
                "dumbmonit_nut_ups_data_stale",
            )
        },
        // --- MikroTik RouterOS (`collectors/mikrotik`) ---
        //
        // Les séries d'interface portent `interface` et `type`, celles de
        // capteur `sensor` : la notification dit « ether1 » ou « psu2-state ».
        Rule {
            description: "The router's CPU has been above 90% for fifteen minutes: routing, \
                          the firewall or a script is saturating it."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(80.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "mikrotik_cpu_high",
                "MikroTik CPU high",
                RuleKind::Threshold,
                "dumbmonit_mikrotik_cpu_load_percent",
            )
        },
        // Une fuite (connexions suivies, listes d'adresses gonflées par un
        // script) monte lentement : quinze minutes au-dessus de 90 %.
        Rule {
            description: "More than 90% of the router's memory is in use.".to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(85.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "mikrotik_memory_high",
                "MikroTik memory high",
                RuleKind::Threshold,
                "dumbmonit_mikrotik_memory_used_percent",
            )
        },
        // Le stockage flash d'un hAP fait 16 Mo : des fichiers de sauvegarde ou
        // un paquet téléchargé suffisent à le remplir, et la configuration ne
        // s'enregistre plus.
        Rule {
            description: "The router's storage is more than 90% full: configuration changes \
                          and upgrades may fail."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(85.0),
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "mikrotik_storage_almost_full",
                "MikroTik storage almost full",
                RuleKind::Threshold,
                "dumbmonit_mikrotik_storage_used_percent",
            )
        },
        // Les processeurs des CCR tournent couramment entre 50 et 70 °C ;
        // quatre-vingts degrés tenus dix minutes, c'est un ventilateur ou une
        // armoire qui ne refroidit plus.
        Rule {
            description: "A temperature sensor of the router is above 80 °C.".to_string(),
            operator: Operator::Gt,
            threshold: 80.0,
            clear_threshold: Some(75.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "°C".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "mikrotik_temperature_high",
                "MikroTik temperature high",
                RuleKind::Threshold,
                "dumbmonit_mikrotik_temperature_celsius",
            )
        },
        // `psu1-state`, `fan-state` : tout ce qui n'est pas « ok ». Une
        // alimentation redondante morte ne coupe rien — jusqu'à la seconde.
        Rule {
            description: "The router reports a failed power supply or fan.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "mikrotik_hardware_failed",
                "MikroTik power supply or fan failed",
                RuleKind::Threshold,
                "dumbmonit_mikrotik_health_ok",
            )
        },
        // Même seuil que la règle IF-MIB : cinquante erreurs dans l'heure.
        Rule {
            description: "An interface of the router logged more than fifty errors in the last \
                          hour: usually a failing cable, a dirty fibre or a duplex mismatch."
                .to_string(),
            operator: Operator::Gt,
            threshold: 50.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "mikrotik_interface_errors",
                "MikroTik interface accumulating errors",
                RuleKind::Threshold,
                "increase_prometheus(dumbmonit_mikrotik_interface_rx_errors_total[1h]) \
                 + increase_prometheus(dumbmonit_mikrotik_interface_tx_errors_total[1h])",
            )
        },
        // `link-downs` compte les pertes de lien : plus de deux en trente
        // minutes, c'est un lien qui bagote. Un port débranché pour de bon ne
        // compte qu'une fois, et ne sonne donc pas.
        Rule {
            description: "An interface of the router lost its link more than twice in thirty \
                          minutes."
                .to_string(),
            operator: Operator::Gt,
            threshold: 2.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "mikrotik_link_flapping",
                "MikroTik link flapping",
                RuleKind::Threshold,
                "increase_prometheus(dumbmonit_mikrotik_interface_link_downs_total[30m])",
            )
        },
        // Seulement si le routeur vérifie lui-même (planificateur de la notice) :
        // un compte en lecture ne peut pas lancer la vérification.
        Rule {
            description: "A newer RouterOS version is available on the router's update channel."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "mikrotik_update_available",
                "MikroTik RouterOS update available",
                RuleKind::Threshold,
                "dumbmonit_mikrotik_update_available",
            )
        },
        // RouterBOOT ne suit pas RouterOS tout seul : après une mise à jour, il
        // faut `/system routerboard upgrade` puis un redémarrage.
        Rule {
            description: "The RouterBOOT firmware is older than the one bundled with the \
                          installed RouterOS: it is upgraded only on request, then at the next \
                          reboot."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "mikrotik_firmware_upgrade_pending",
                "MikroTik firmware upgrade pending",
                RuleKind::Threshold,
                "dumbmonit_mikrotik_firmware_upgrade_pending",
            )
        },
        // --- UniFi Network (`collectors/unifi`) ---
        //
        // Les séries d'équipement portent `device`, `model` et `type` : la
        // notification dit « Living room AP ». Un équipement en attente
        // d'adoption n'a pas de `device_up` : il n'est pas encore à nous.
        Rule {
            description: "A UniFi device adopted by the controller is offline, isolated or failed \
                          to adopt."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            escalate_after: Some(Duration::from_secs(3600)),
            ..base("unifi_device_offline", "UniFi device offline", RuleKind::Threshold, "dumbmonit_unifi_device_up")
        },
        // `www` en erreur : la passerelle ne joint plus Internet. Absent sans
        // passerelle UniFi, la règle ne dit alors rien.
        Rule {
            description: "The UniFi gateway reports no Internet connection.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(3 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "unifi_internet_down",
                "UniFi Internet down",
                RuleKind::Threshold,
                "dumbmonit_unifi_internet_up",
            )
        },
        // Un second lien WAN tombé ne coupe rien, et c'est justement pourquoi
        // il faut le dire : la redondance est perdue sans que personne le voie.
        Rule {
            description: "A WAN link of the UniFi gateway is down.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "unifi_wan_link_down",
                "UniFi WAN link down",
                RuleKind::Threshold,
                "dumbmonit_unifi_wan_link_up",
            )
        },
        Rule {
            description: "The UniFi gateway's processor has been above 90% for fifteen minutes."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(80.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            ..base(
                "unifi_gateway_cpu_high",
                "UniFi gateway CPU high",
                RuleKind::Threshold,
                "dumbmonit_unifi_device_cpu_percent{type=\"gateway\"}",
            )
        },
        Rule {
            description: "A device appeared on the network and waits to be adopted in UniFi."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "unifi_device_pending_adoption",
                "UniFi device waiting for adoption",
                RuleKind::Threshold,
                "dumbmonit_unifi_devices{state=\"pending\"}",
            )
        },
        Rule {
            description: "A firmware update is available for UniFi devices.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "unifi_firmware_update",
                "UniFi firmware update available",
                RuleKind::Threshold,
                "dumbmonit_unifi_devices_upgradable",
            )
        },
        Rule {
            description: "UniFi logged critical events or raised alarms in the last 24 hours."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base("unifi_alarms", "UniFi alarms", RuleKind::Threshold, "dumbmonit_unifi_alarms")
        },
        // --- Home Assistant (`collectors/homeassistant`) ---
        Rule {
            description: "Home Assistant started in recovery mode: its configuration could not be \
                          loaded and automations do not run."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(2 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "homeassistant_recovery_mode",
                "Home Assistant in recovery mode",
                RuleKind::Threshold,
                "dumbmonit_homeassistant_recovery_mode",
            )
        },
        // Démarrage bloqué : le cœur répond mais reste en `STARTING` ou `NOT_RUNNING`.
        Rule {
            description: "Home Assistant answers but has not finished starting for ten minutes."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            ..base(
                "homeassistant_not_running",
                "Home Assistant not running",
                RuleKind::Threshold,
                "dumbmonit_homeassistant_running",
            )
        },
        // Une intégration qui décroche fait passer d'un coup toutes ses entités
        // `unavailable` : c'est la hausse qui compte, pas le total, qu'une
        // installation réelle n'a jamais à zéro.
        Rule {
            description: "At least five more entities are unavailable than an hour ago: an \
                          integration, a hub or a radio stopped."
                .to_string(),
            operator: Operator::Ge,
            threshold: 5.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "homeassistant_entities_unavailable",
                "Home Assistant entities went unavailable",
                RuleKind::Threshold,
                "sum by (target) (dumbmonit_homeassistant_entities_unavailable) \
                 - sum by (target) (dumbmonit_homeassistant_entities_unavailable offset 1h)",
            )
        },
        // Une série par pile faible, valeur = niveau (0 pour un capteur binaire) :
        // toute série présente déclenche.
        Rule {
            description: "A battery reported by Home Assistant is low.".to_string(),
            operator: Operator::Ge,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(3 * 24 * 3600)),
            unit: "%".to_string(),
            ..base(
                "homeassistant_battery_low",
                "Home Assistant battery low",
                RuleKind::Threshold,
                "dumbmonit_homeassistant_battery_low",
            )
        },
        Rule {
            description: "Home Assistant has updates waiting: core, operating system, add-ons or \
                          device firmware."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "homeassistant_update_available",
                "Home Assistant update available",
                RuleKind::Threshold,
                "dumbmonit_homeassistant_updates_available",
            )
        },
        Rule {
            description: "Home Assistant has an open repair of error or critical severity."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "homeassistant_repair",
                "Home Assistant repair to address",
                RuleKind::Threshold,
                "dumbmonit_homeassistant_repair{severity=~\"error|critical\"}",
            )
        },
        // --- VMware vSphere (`collectors/vsphere`) ---
        //
        // `host_connection_state` : 2 « ne répond plus ». Un hôte déconnecté par
        // un administrateur (1) est un choix, pas une panne.
        Rule {
            description: "A vSphere host is not responding to vCenter.".to_string(),
            operator: Operator::Ge,
            threshold: 2.0,
            for_duration: Duration::from_secs(3 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "vsphere_host_not_responding",
                "vSphere host not responding",
                RuleKind::Threshold,
                "dumbmonit_vsphere_host_connection_state",
            )
        },
        // `overallStatus` : 2 rouge. Le gris (3) dit « inconnu », pas « en panne ».
        Rule {
            description: "vSphere reports a host's overall health as red.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "vsphere_host_red",
                "vSphere host health red",
                RuleKind::Threshold,
                "dumbmonit_vsphere_host_status == 2",
            )
        },
        Rule {
            description: "A vSphere host has been in maintenance mode for a day: its VMs were \
                          moved away and it may have been forgotten there."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(24 * 3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "vsphere_host_maintenance",
                "vSphere host left in maintenance",
                RuleKind::Threshold,
                "dumbmonit_vsphere_host_maintenance",
            )
        },
        Rule {
            description: "A vSphere datastore is 90% full or more.".to_string(),
            operator: Operator::Ge,
            threshold: 90.0,
            clear_threshold: Some(85.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            escalate_after: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "vsphere_datastore_almost_full",
                "vSphere datastore almost full",
                RuleKind::Threshold,
                "dumbmonit_vsphere_datastore_used_percent",
            )
        },
        Rule {
            description: "A vSphere datastore is inaccessible: the VMs stored on it cannot run."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "vsphere_datastore_inaccessible",
                "vSphere datastore inaccessible",
                RuleKind::Threshold,
                "dumbmonit_vsphere_datastore_accessible",
            )
        },
        // 2 : outils installés mais arrêtés dans une VM allumée — souvent une
        // VM figée. Absents (3) : un choix, pas une alerte.
        Rule {
            description: "VMware Tools are installed but not running in a powered-on VM: the \
                          guest may be hung."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "vsphere_vm_tools_not_running",
                "vSphere VM tools not running",
                RuleKind::Threshold,
                "dumbmonit_vsphere_vm_tools_status == 2",
            )
        },
        // Une série par alarme rouge non acquittée, nommée par l'alarme et l'objet.
        Rule {
            description: "vSphere raised a red alarm that nobody acknowledged.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "vsphere_alarm_red",
                "vSphere red alarm",
                RuleKind::Threshold,
                "dumbmonit_vsphere_alarm{status=\"red\"}",
            )
        },
        // --- Proxmox VE : invités, disques, ZFS, paquets (`collectors/proxmox`) ---
        //
        // Les séries d'invité portent `name` et `vmid` : la notification dit
        // « nextcloud (202) ». Celles de disque portent `node` et `disk`, celles
        // de pool `node` et `pool` : « pve1 · /dev/sda », « pve2 · tank ».
        //
        // Processeur d'un invité : le pourcentage est celui des cœurs alloués,
        // une machine à 100 % n'a que ses propres cœurs à saturer. Quinze
        // minutes écartent les pics de démarrage et de sauvegarde.
        Rule {
            description:
                "A VM or container has used more than 90% of its allocated cores for fifteen minutes."
                    .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pve_guest_cpu_high",
                "VM or container CPU high",
                RuleKind::Threshold,
                "dumbmonit_proxmox_guest_cpu_percent",
            )
        },
        // Mémoire d'un invité : PVE mesure la mémoire consommée vue de l'hôte,
        // qui plafonne naturellement près de 100 % avec un ballon ou un cache
        // de fichiers actif — d'où 95 % et dix minutes.
        Rule {
            description: "A VM or container has used more than 95% of its memory for ten minutes."
                .to_string(),
            operator: Operator::Gt,
            threshold: 95.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pve_guest_memory_high",
                "VM or container memory high",
                RuleKind::Threshold,
                "dumbmonit_proxmox_guest_memory_percent",
            )
        },
        // Disque racine d'un invité : connu pour un conteneur, et pour une
        // machine virtuelle dont l'agent QEMU répond. Sans mesure, pas de série
        // et donc pas d'alerte — jamais un faux « 0 % ».
        Rule {
            description: "The root disk of a VM or container is more than 90% full.".to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pve_guest_disk_almost_full",
                "VM or container disk almost full",
                RuleKind::Threshold,
                "dumbmonit_proxmox_guest_disk_used_percent",
            )
        },
        // Usure d'un SSD ou NVMe : 0 % neuf, 100 % en fin de vie d'après le
        // constructeur. À 90 %, il est temps de commander le remplaçant.
        Rule {
            description: "An SSD or NVMe disk of a node has used more than 90% of its rated life."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pve_disk_wearout",
                "Proxmox disk wearing out",
                RuleKind::Threshold,
                "dumbmonit_proxmox_node_disk_wearout_percent",
            )
        },
        // SMART en échec : le disque lui-même se déclare en fin de vie.
        Rule {
            description: "A disk of a Proxmox node reports a failed SMART health check.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pve_disk_smart_failed",
                "Proxmox disk SMART failure",
                RuleKind::Threshold,
                "dumbmonit_proxmox_node_disk_smart_failed",
            )
        },
        // Pool ZFS dégradé : un disque manque, la redondance est consommée.
        Rule {
            description: "A ZFS pool of a Proxmox node is not ONLINE: a disk is missing or faulted."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pve_zfs_pool_degraded",
                "Proxmox ZFS pool degraded",
                RuleKind::Threshold,
                "dumbmonit_proxmox_node_zfs_pool_degraded",
            )
        },
        // Correctifs de sécurité : un seul suffit, contrairement au décompte
        // général des mises à jour qui attend vingt paquets.
        Rule {
            description: "Security updates are pending on this Proxmox node.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pve_security_updates_pending",
                "Proxmox security updates pending",
                RuleKind::Threshold,
                "dumbmonit_proxmox_node_updates_security_pending",
            )
        },
        // Paquets mis à jour : quelqu'un a passé `apt upgrade` sur le nœud. Une
        // information, pas une panne — mais celle qui explique le redémarrage
        // ou le comportement changé qu'on constate ensuite. La série reste
        // publiée une heure avec le détail en étiquette `changes`, puis se
        // résout seule.
        Rule {
            description: "Installed Proxmox packages changed since the previous probe.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(60),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pve_packages_changed",
                "Proxmox packages changed",
                RuleKind::Threshold,
                "dumbmonit_proxmox_node_packages_changed",
            )
        },
        // --- fin du bloc Proxmox VE : invités, disques, ZFS, paquets ---
        // --- Synology DSM : stockage, disques, mémoire et rythme Active Backup
        // (`collectors/synology/{metrics,devices,rhythm}.rs`) ---
        //
        // Les états de DSM sont publiés sur une échelle commune : 0 normal, 1 à
        // surveiller, 2 critique — et l'inconnu vaut 1, jamais 0. Une règle sur
        // « > 0 » attrape donc aussi un état inédit, ce qui est voulu : mieux vaut
        // un avertissement qu'un disque en panne sous un libellé nouveau.
        Rule {
            description: "The SMART status of this disk is no longer \"normal\", as reported \
                          by DSM."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "synology_disk_smart_warning",
                "Synology disk SMART warning",
                RuleKind::Threshold,
                "dumbmonit_synology_disk_smart_status",
            )
        },
        // Un disque « crashed » a déjà quitté la grappe : c'est une panne, pas un
        // avertissement.
        Rule {
            description: "DSM reports this disk as crashed or failed.".to_string(),
            operator: Operator::Ge,
            threshold: 2.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "synology_disk_failed",
                "Synology disk failed",
                RuleKind::Threshold,
                "dumbmonit_synology_disk_status",
            )
        },
        Rule {
            description: "The bad-sector count of this disk exceeds the threshold set in DSM."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "synology_disk_bad_sectors",
                "Synology disk bad sectors",
                RuleKind::Threshold,
                "dumbmonit_synology_disk_bad_sector_exceeded",
            )
        },
        // Des secteurs illisibles qui apparaissent sont le signe le plus précoce
        // d'une panne : `delta` sur une jauge compare la valeur d'il y a un jour à
        // celle d'aujourd'hui. Un disque remplacé repart de zéro, ce qui donne une
        // variation négative et ne déclenche pas.
        Rule {
            description: "New unreadable (UNC) sectors appeared on this disk in the last \
                          24 hours."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "synology_disk_bad_sectors_growing",
                "Synology disk bad sectors growing",
                RuleKind::Threshold,
                "delta(dumbmonit_synology_disk_unc_count[24h])",
            )
        },
        Rule {
            description: "This SSD has less than 10% of its rated life left.".to_string(),
            operator: Operator::Lt,
            threshold: 10.0,
            clear_threshold: Some(12.0),
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "synology_ssd_wearout",
                "Synology SSD wearing out",
                RuleKind::Threshold,
                "dumbmonit_synology_disk_remaining_life_percent",
            )
        },
        // Un volume dégradé fonctionne encore, sans redondance : la prochaine
        // panne est celle qui perd les données. Même seuil pour « crashed ».
        Rule {
            description: "This volume is degraded or crashed: its redundancy is gone.".to_string(),
            operator: Operator::Ge,
            threshold: 2.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "synology_volume_degraded",
                "Synology volume degraded",
                RuleKind::Threshold,
                "dumbmonit_synology_volume_status",
            )
        },
        // Un groupe de stockage se dégrade avant le volume posé dessus : sur un
        // RAID 5 qui perd un disque, DSM laisse le volume en « normal » tant que
        // la reconstruction tient. Sans cette règle, la perte de redondance ne se
        // voit qu'au second disque — c'est-à-dire trop tard.
        Rule {
            description: "This storage pool is degraded or crashed: a disk failed and the \
                          redundancy is gone or rebuilding."
                .to_string(),
            operator: Operator::Ge,
            threshold: 2.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "synology_pool_degraded",
                "Synology storage pool degraded",
                RuleKind::Threshold,
                "dumbmonit_synology_pool_status or dumbmonit_synology_ssd_cache_status",
            )
        },
        Rule {
            description: "Volume 90% full or more.".to_string(),
            operator: Operator::Ge,
            threshold: 90.0,
            clear_threshold: Some(88.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            escalate_after: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "synology_volume_almost_full",
                "Synology volume almost full",
                RuleKind::Threshold,
                "dumbmonit_synology_volume_used_percent",
            )
        },
        // 55 °C est le seuil à partir duquel les fabricants de disques mécaniques
        // et Synology parlent de surchauffe ; l'hystérésis évite les allers-retours
        // d'un disque qui oscille autour du seuil pendant une reconstruction.
        Rule {
            description: "A disk of this NAS is above 55 °C, or DSM raised its temperature \
                          warning."
                .to_string(),
            operator: Operator::Gt,
            threshold: 55.0,
            clear_threshold: Some(52.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "°C".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "synology_temperature_high",
                "Synology temperature high",
                RuleKind::Threshold,
                // Le drapeau de DSM vaut 0 ou 1 : multiplié par 100, il dépasse le
                // seuil à lui seul quand le NAS lui-même se déclare en surchauffe.
                "dumbmonit_synology_disk_temperature_celsius \
                 or (100 * dumbmonit_synology_temperature_warning > 55)",
            )
        },
        Rule {
            description: "Memory usage of the NAS above 95% for fifteen minutes.".to_string(),
            operator: Operator::Gt,
            threshold: 95.0,
            clear_threshold: Some(90.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "synology_memory_high",
                "Synology memory high",
                RuleKind::Threshold,
                "dumbmonit_synology_memory_usage_percent",
            )
        },
        // Rythme Active Backup par appareil : la série vaut 1 seulement quand le
        // modèle juge l'appareil en retard *par rapport à ses propres habitudes*
        // (jours de repos exclus, tolérance dérivée de son intervalle habituel).
        // Une demi-heure de `for` laisse passer une relecture de l'historique.
        Rule {
            description: "No successful Active Backup for Business run for this device for \
                          longer than its usual rhythm allows (off-days excluded)."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "synology_abb_device_overdue",
                "Active Backup device overdue",
                RuleKind::Threshold,
                "dumbmonit_abb_device_overdue",
            )
        },
        // Deux tentatives échouées d'affilée : ce n'est plus un portable refermé
        // au mauvais moment, c'est un appareil qui ne se sauvegarde plus.
        Rule {
            description: "The last two or more Active Backup for Business attempts of this \
                          device failed."
                .to_string(),
            operator: Operator::Ge,
            threshold: 2.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "synology_abb_device_failing",
                "Active Backup device failing",
                RuleKind::Threshold,
                "dumbmonit_abb_device_consecutive_failures",
            )
        },
        // --- Redis / Valkey (`collectors/redis`) ---
        // Au plafond, Redis évince ou refuse les écritures selon la politique ;
        // sans `maxmemory`, la série n'existe pas et la règle se tait.
        Rule {
            description: "Redis uses more than 90% of maxmemory: at the limit it evicts keys or refuses every write, depending on maxmemory-policy."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(85.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base("redis_memory_near_limit", "Redis memory near maxmemory", RuleKind::Threshold, "dumbmonit_redis_memory_used_percent")
        },
        Rule {
            description: "Redis refused client connections in the last 15 minutes: maxclients is reached."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base("redis_rejecting_connections", "Redis refusing connections", RuleKind::Threshold, "increase_prometheus(dumbmonit_redis_rejected_connections_total[15m])")
        },
        Rule {
            description: "This Redis replica lost its link to the primary: it serves stale data and will not take over cleanly."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(2 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base("redis_replication_broken", "Redis replication link down", RuleKind::Threshold, "dumbmonit_redis_master_link_up")
        },
        // Vu du primaire : secondes depuis le dernier acquittement d'une réplique,
        // qui en envoie un par seconde.
        Rule {
            description: "A Redis replica has not acknowledged the replication stream for more than 30 seconds."
                .to_string(),
            operator: Operator::Gt,
            threshold: 30.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base("redis_replica_lagging", "Redis replica lagging", RuleKind::Threshold, "dumbmonit_redis_replica_lag_seconds")
        },
        Rule {
            description: "The last Redis snapshot (BGSAVE) failed: with stop-writes-on-bgsave-error, Redis now refuses every write."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base("redis_save_failed", "Redis snapshot failed", RuleKind::Threshold, "dumbmonit_redis_rdb_last_save_ok")
        },
        Rule {
            description: "Redis could not write its append-only file: recent writes are not on disk."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base("redis_aof_write_failed", "Redis append-only file write failed", RuleKind::Threshold, "dumbmonit_redis_aof_last_write_ok")
        },
        // --- MongoDB (`collectors/mongodb`) ---
        Rule {
            description: "The MongoDB replica set has no primary: no write is accepted until one is elected."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(2 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base("mongodb_no_primary", "MongoDB replica set without primary", RuleKind::Threshold, "dumbmonit_mongodb_replset_primary_present")
        },
        Rule {
            description: "A member of the MongoDB replica set is unreachable from the others."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base("mongodb_member_unhealthy", "MongoDB member unreachable", RuleKind::Threshold, "dumbmonit_mongodb_replset_member_health")
        },
        Rule {
            description: "A MongoDB secondary is more than a minute behind the primary: if it is elected, the latest writes are missing."
                .to_string(),
            operator: Operator::Gt,
            threshold: 60.0,
            clear_threshold: Some(30.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base("mongodb_replication_lag", "MongoDB replication lag", RuleKind::Threshold, "dumbmonit_mongodb_replset_member_lag_seconds")
        },
        Rule {
            description: "MongoDB uses more than 80% of its available connections: new clients will be refused."
                .to_string(),
            operator: Operator::Gt,
            threshold: 80.0,
            clear_threshold: Some(75.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base("mongodb_connections_high", "MongoDB connections near the limit", RuleKind::Threshold, "dumbmonit_mongodb_connections_used_percent")
        },
        // 20 % : le seuil `eviction_dirty_trigger` de WiredTiger, au-delà
        // duquel les threads applicatifs évincent eux-mêmes.
        Rule {
            description: "More than 20% of the WiredTiger cache is dirty: application threads have to evict pages themselves and queries slow down."
                .to_string(),
            operator: Operator::Gt,
            threshold: 20.0,
            clear_threshold: Some(15.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base("mongodb_cache_dirty", "MongoDB cache under pressure", RuleKind::Threshold, "dumbmonit_mongodb_wiredtiger_cache_dirty_percent")
        },
        Rule {
            description: "MongoDB raised regular assertions in the last hour: internal server errors, worth a look in its log."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base("mongodb_assertions", "MongoDB internal errors", RuleKind::Threshold, "increase_prometheus(dumbmonit_mongodb_asserts_total{type=\"regular\"}[1h])")
        },
        // --- RabbitMQ (`collectors/rabbitmq`) ---
        Rule {
            description: "A RabbitMQ node raised a memory or disk alarm: every publisher in the cluster is blocked."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base("rabbitmq_resource_alarm", "RabbitMQ memory or disk alarm", RuleKind::Threshold, "dumbmonit_rabbitmq_alarm")
        },
        Rule {
            description: "A RabbitMQ cluster node is not running."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(2 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base("rabbitmq_node_down", "RabbitMQ node down", RuleKind::Threshold, "dumbmonit_rabbitmq_node_running")
        },
        Rule {
            description: "RabbitMQ nodes see a network partition: the cluster is split and queues may diverge."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(2 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base("rabbitmq_partition", "RabbitMQ network partition", RuleKind::Threshold, "dumbmonit_rabbitmq_node_partitions")
        },
        // Des messages en attente et personne pour les lire : le service qui
        // vidait la file est tombé. La valeur est le nombre de messages.
        Rule {
            description: "A RabbitMQ queue holds messages and has had no consumer for 15 minutes: whatever read it has stopped."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base("rabbitmq_queue_no_consumer", "RabbitMQ queue without consumer", RuleKind::Threshold, "dumbmonit_rabbitmq_queue_messages_ready and dumbmonit_rabbitmq_queue_consumers == 0")
        },
        Rule {
            description: "A RabbitMQ queue is not running: a quorum queue without majority, or a queue whose node is down."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base("rabbitmq_queue_not_running", "RabbitMQ queue unavailable", RuleKind::Threshold, "dumbmonit_rabbitmq_queue_running")
        },
        // --- CrowdSec (`collectors/crowdsec`) ---
        Rule {
            description: "The CrowdSec Local API does not answer: no decision is taken or handed to the bouncers."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base("crowdsec_lapi_down", "CrowdSec Local API down", RuleKind::Threshold, "dumbmonit_crowdsec_lapi_up")
        },
        // CrowdSec ne publie pas l'heure du dernier tirage : le compteur de
        // requêtes du bouncer qui ne bouge plus en tient lieu.
        Rule {
            description: "A CrowdSec bouncer has not called the Local API for 30 minutes: it enforces a frozen list."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base("crowdsec_bouncer_stale", "CrowdSec bouncer stopped pulling", RuleKind::Threshold, "changes_prometheus(dumbmonit_crowdsec_bouncer_requests_total[30m])")
        },
        Rule {
            description: "CrowdSec has not read a single log line in six hours: its acquisition is broken and it sees no attack."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base("crowdsec_no_logs_read", "CrowdSec reads no logs", RuleKind::Threshold, "increase_prometheus(dumbmonit_crowdsec_lines_read_total[6h])")
        },
        // --- Proxys inverses (`collectors/{traefik,caddy,npm}`) ---
        Rule {
            description: "A Traefik router is disabled by a configuration error: its sites are not served."
                .to_string(),
            operator: Operator::Ge,
            threshold: 2.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base("traefik_router_disabled", "Traefik router disabled", RuleKind::Threshold, "dumbmonit_traefik_router_status")
        },
        Rule {
            description: "A backend server of a Traefik service fails its health check."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base("traefik_server_down", "Traefik backend server down", RuleKind::Threshold, "dumbmonit_traefik_server_up")
        },
        Rule {
            description: "No backend server of a Traefik service answers its health check: its sites get 503."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(2 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base("traefik_service_down", "Traefik service without a server", RuleKind::Threshold, "dumbmonit_traefik_service_servers_up")
        },
        // ACME obtient un certificat en quelques secondes : une heure sans,
        // c'est un résolveur qui échoue, et Traefik sert son certificat
        // auto-signé par défaut.
        Rule {
            description: "A Traefik router has waited an hour for a certificate from its resolver: Traefik serves its default self-signed certificate instead."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base("traefik_certificate_missing", "Traefik certificate not obtained", RuleKind::Threshold, "dumbmonit_traefik_resolver_routers_uncovered")
        },
        Rule {
            description: "A Caddy upstream fails its active or passive health check."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base("caddy_upstream_unhealthy", "Caddy upstream unhealthy", RuleKind::Threshold, "dumbmonit_caddy_upstream_healthy")
        },
        Rule {
            description: "Caddy refused its last configuration reload: it still runs the previous configuration."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base("caddy_reload_failed", "Caddy configuration reload failed", RuleKind::Threshold, "dumbmonit_caddy_config_last_reload_ok")
        },
        Rule {
            description: "Nginx Proxy Manager could not load a host's configuration (nginx -t failed): the host is not served."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base("npm_host_offline", "Nginx Proxy Manager host offline", RuleKind::Threshold, "dumbmonit_npm_host_online")
        },
        // Let's Encrypt et ACME renouvellent trente jours avant l'échéance : à
        // quatorze jours, le renouvellement a échoué au moins une semaine.
        Rule {
            description: "A certificate served by the reverse proxy expires in less than fourteen days: its renewal is failing."
                .to_string(),
            operator: Operator::Lt,
            threshold: 14.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "d".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base("proxy_cert_expiring", "Proxy certificate not renewed", RuleKind::Threshold, "dumbmonit_traefik_cert_expiry_days or dumbmonit_npm_cert_expiry_days")
        },
        // Part de 5xx sur dix minutes, avec un plancher de cinquante requêtes :
        // une erreur sur deux requêtes nocturnes n'est pas un incident.
        Rule {
            description: "More than 5% of a reverse proxy's answers were 5xx errors over ten minutes, with at least fifty requests."
                .to_string(),
            operator: Operator::Gt,
            threshold: 5.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "proxy_5xx_rate",
                "Reverse proxy answering 5xx",
                RuleKind::Threshold,
                "(100 * increase_prometheus(dumbmonit_traefik_requests_5xx_total[10m]) \
                 / increase_prometheus(dumbmonit_traefik_requests_total[10m]) \
                 and increase_prometheus(dumbmonit_traefik_requests_total[10m]) >= 50) \
                 or (100 * increase_prometheus(dumbmonit_caddy_requests_5xx_total[10m]) \
                 / increase_prometheus(dumbmonit_caddy_requests_total[10m]) \
                 and increase_prometheus(dumbmonit_caddy_requests_total[10m]) >= 50)",
            )
        },
        // --- Noms de domaine (`collectors/domain`) ---
        // Deux règles disjointes (`>= 7` / `< 7`), comme pour les certificats.
        Rule {
            description: "The domain expires in less than thirty days.".to_string(),
            operator: Operator::Lt,
            threshold: 30.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "d".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base("domain_expiring", "Domain expiring", RuleKind::Threshold, "dumbmonit_domain_expiry_days >= 7")
        },
        Rule {
            description: "The domain expires in less than seven days, or has expired.".to_string(),
            operator: Operator::Lt,
            threshold: 7.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Critical,
            unit: "d".to_string(),
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base("domain_expiring_soon", "Domain about to expire", RuleKind::Threshold, "dumbmonit_domain_expiry_days")
        },
        Rule {
            description: "The registry put the domain on hold (clientHold or serverHold): it is taken out of the DNS."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base("domain_on_hold", "Domain on hold", RuleKind::Threshold, "dumbmonit_domain_on_hold")
        },
        Rule {
            description: "The domain is in its redemption period or pending deletion: it has expired and will be released."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base("domain_redemption", "Domain in redemption", RuleKind::Threshold, "dumbmonit_domain_redemption")
        },
        // Réponse DNS modifiée (`uptime/dns`, option `alert_on_change`) :
        // l'empreinte ne change que si l'ensemble des valeurs change, et
        // `changes_prometheus` ne compte pas la première valeur d'une série
        // neuve. L'alerte se résout d'elle-même une heure plus tard.
        Rule {
            description: "The answer of a DNS check changed: a record was added, removed or replaced."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base("dns_answer_changed", "DNS answer changed", RuleKind::Threshold, "changes_prometheus(dumbmonit_probe_dns_answer_fingerprint[1h])")
        },
        // --- fin du bloc Synology DSM ---
        // La sauvegarde locale de DumbMonit lui-même.
        //
        // Elle est la seule chose qui rende une mise à jour rattrapable, et
        // c'est aussi la seule que personne ne va vérifier : on ne regarde un
        // répertoire de sauvegardes que le jour où l'on en a besoin. La série
        // n'existe qu'une fois la première sauvegarde écrite — instance dont la
        // planification est coupée, pas d'alerte, pas de faux reproche — et
        // porte l'heure de la dernière **réussite**, jamais celle d'une
        // tentative ratée. Deux jours au seuil : le rythme livré est quotidien,
        // une nuit manquée peut être un redémarrage.
        Rule {
            description: "The scheduled local backup of the DumbMonit database has not run for \
                          more than two days."
                .to_string(),
            operator: Operator::Gt,
            threshold: 2.0 * 24.0 * 3600.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "instance_backup_missing",
                "DumbMonit backup did not run",
                RuleKind::Threshold,
                "time() - dumbmonit_instance_backup_last_success_seconds",
            )
        },
    ]
}

/// Construit une requête prédictive « ce système de fichiers sera plein dans moins
/// de `days` jours », pour l'assistant de création de règle de l'interface.
///
/// Le calcul reste intégralement côté VictoriaMetrics : rien n'est extrapolé en Rust,
/// ce qui évite d'avoir à rapatrier l'historique de chaque série à chaque cycle.
pub fn predict_full_query(metric: &str, window_hours: u32, days: u32) -> String {
    let horizon = u64::from(days) * 24 * 3600;
    format!(
        "predict_linear({metric}[{window_hours}h], {horizon}) and deriv({metric}[{window_hours}h]) > 0"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_regles_livrees_couvrent_les_cas_annonces() {
        let rules = builtin_rules();
        let uids: Vec<&str> = rules.iter().map(|r| r.uid.as_str()).collect();
        for attendu in [
            RULE_HOST_DOWN,
            "cpu_high",
            "disk_almost_full",
            "ups_on_battery",
            "ups_battery_low",
            // Ports réseau (IF-MIB) et matériel serveur (Redfish).
            "port_errors",
            "port_flapping",
            "redfish_fan_failed",
            "redfish_temperature_critical",
            "redfish_psu_redundancy_lost",
            "redfish_psu_failed",
            "redfish_drive_failure_predicted",
            "redfish_system_critical",
            "fs_will_be_full",
            "backup_too_old",
            "pbs_datastore_almost_full",
            "pbs_datastore_will_be_full",
            "pbs_backup_too_old",
            "pbs_backup_verification_failed",
            "pbs_task_failed",
            "pbs_gc_too_old",
            "service_down",
            "push_missed",
            "service_flapping",
            "service_slow",
            "tls_cert_expiring",
            "tls_cert_expired",
            "container_stopped",
            "container_unhealthy",
            "container_restarting",
            "container_update_available",
            "plakar_backup_too_old",
            "plakar_backup_failed",
            // Matériel vu par l'agent (`crates/agent/src/collect/{smart,zfs,sensors}.rs`).
            "disk_smart_failed",
            "zfs_pool_degraded",
            "zfs_scrub_errors",
            "sensor_temperature_critical",
            "synology_abb_task_failed",
            "synology_abb_backup_too_old",
            "synology_abb_task_disabled",
            "pve_guest_stopped",
            "pve_ha_resource_error",
            "pve_cluster_no_quorum",
            "pve_node_offline",
            "pve_storage_almost_full",
            "pve_backup_job_failed",
            "pve_snapshot_old",
            "pve_replication_failed",
            "pve_ceph_health_error",
            "pve_ceph_health_warning",
            "pve_updates_pending",
            "pve_certificate_expiring",
            "pbs_sync_failed",
            "pbs_updates_pending",
            "pbs_prune_failed",
            "pbs_verify_job_failed",
            "pbs_gc_failed",
            "pbs_disk_smart_failed",
            "pbs_disk_wearout",
            "pbs_zpool_degraded",
            // Proxmox Datacenter Manager (`collectors/pdm`).
            "pdm_remote_unreachable",
            "pdm_remote_version_behind",
            "pdm_task_failed",
            "pdm_node_disk_almost_full",
            "pdm_certificate_expiring",
            "pdm_updates_pending",
            "pbs_service_down",
            "pbs_certificate_expiring",
            "pbs_version_stale",
            "pbs_tape_job_failed",
            "pbs_job_never_run",
            "pbs_gc_bad_chunks",
            "pbs_datastore_unmounted",
            "pve_guest_cpu_high",
            "pve_guest_memory_high",
            "pve_guest_disk_almost_full",
            "pve_disk_wearout",
            "pve_disk_smart_failed",
            // Synology DSM (`collectors/synology`).
            "synology_disk_smart_warning",
            "synology_disk_failed",
            "synology_disk_bad_sectors",
            "synology_disk_bad_sectors_growing",
            "synology_ssd_wearout",
            "synology_volume_degraded",
            "synology_volume_almost_full",
            "synology_temperature_high",
            "synology_memory_high",
            "synology_abb_device_overdue",
            "synology_abb_device_failing",
            "pve_zfs_pool_degraded",
            "pve_security_updates_pending",
            "pve_packages_changed",
            // Proxmox Mail Gateway (`collectors/pmg`).
            "pmg_queue_growing",
            "pmg_queue_stuck",
            "pmg_filter_service_down",
            "pmg_virus_signatures_stale",
            "pmg_spam_rules_stale",
            "pmg_quarantine_growing",
            "pmg_cluster_degraded",
            "pmg_certificate_expiring",
            "pmg_updates_pending",
            // MDaemon et SecurityGateway (`collectors/mdaemon`).
            "mdaemon_service_down",
            "mdaemon_api_down",
            "mdaemon_queue_growing",
            "mdaemon_bad_queue_not_empty",
            "mdaemon_retry_queue_high",
            "securitygateway_service_down",
            "securitygateway_api_down",
            "securitygateway_queue_growing",
            // OPNsense (`collectors/opnsense`).
            "opnsense_gateway_down",
            "opnsense_gateway_loss",
            "opnsense_gateway_latency",
            "opnsense_state_table_filling",
            "opnsense_mbuf_exhausted",
            "opnsense_vpn_tunnel_down",
            "opnsense_resolver_down",
            "opnsense_core_service_down",
            "opnsense_temperature_high",
            "opnsense_carp_maintenance",
            "opnsense_reboot_pending",
            "opnsense_updates_pending",
            // TrueNAS (`collectors/truenas`).
            "truenas_pool_degraded",
            "truenas_pool_device_errors",
            "truenas_pool_almost_full",
            "truenas_scrub_errors",
            "truenas_scrub_overdue",
            "truenas_disk_smart_failed",
            "truenas_disk_hot",
            "truenas_dataset_quota_full",
            "truenas_replication_failed",
            "truenas_snapshot_task_failed",
            "truenas_snapshots_stale",
            "truenas_alert_raised",
            "truenas_service_down",
            // Serveurs de journaux et de métriques (`collectors/observability`).
            "victoria_read_only",
            "victoria_disk_headroom_low",
            "victoria_unhealthy",
            "victoria_rows_rejected",
            "victoriametrics_slow_inserts",
            "loki_not_ready",
            "loki_lines_discarded",
            "loki_flush_failing",
            "loki_wal_disk_full",
            "loki_request_errors",
            "graylog_not_processing",
            "graylog_search_cluster_down",
            "graylog_search_cluster_yellow",
            "graylog_journal_filling",
            "graylog_input_failed",
            "graylog_indexing_failures",
            // Applications auto-hébergées (`collectors/selfhosted`).
            "nextcloud_maintenance",
            "nextcloud_upgrade_pending",
            "nextcloud_update_available",
            "nextcloud_opcache_full",
            "nextcloud_free_space_low",
            "immich_jobs_stalled",
            "immich_queue_paused",
            "immich_storage_almost_full",
            "immich_update_available",
            "paperless_broker_down",
            "paperless_component_error",
            "paperless_migrations_pending",
            "paperless_task_failed",
            "paperless_storage_almost_full",
            "paperless_update_available",
            "jellyfin_task_failed",
            "jellyfin_plugin_broken",
            "jellyfin_restart_pending",
            "plex_update_available",
            // Pi-hole (`collectors/pihole`).
            "pihole_blocking_disabled",
            "pihole_gravity_stale",
            "pihole_update_available",
            "pihole_diagnosis_messages",
            // AdGuard Home (`collectors/adguard`).
            "adguard_protection_disabled",
            "adguard_dns_not_running",
            "adguard_upstream_down",
            "adguard_filters_stale",
            "adguard_update_available",
            // Onduleurs derrière NUT (`collectors/nut`).
            "nut_on_battery",
            "nut_battery_low",
            "nut_replace_battery",
            "nut_load_high",
            "nut_runtime_short",
            "nut_data_stale",
            // MikroTik RouterOS (`collectors/mikrotik`).
            "mikrotik_cpu_high",
            "mikrotik_memory_high",
            "mikrotik_storage_almost_full",
            "mikrotik_temperature_high",
            "mikrotik_hardware_failed",
            "mikrotik_interface_errors",
            "mikrotik_link_flapping",
            "mikrotik_update_available",
            "mikrotik_firmware_upgrade_pending",
            // UniFi, Home Assistant, vSphere (`collectors/{unifi,homeassistant,vsphere}`).
            "unifi_device_offline",
            "unifi_internet_down",
            "unifi_wan_link_down",
            "unifi_gateway_cpu_high",
            "unifi_device_pending_adoption",
            "unifi_firmware_update",
            "unifi_alarms",
            "homeassistant_recovery_mode",
            "homeassistant_not_running",
            "homeassistant_entities_unavailable",
            "homeassistant_battery_low",
            "homeassistant_update_available",
            "homeassistant_repair",
            "vsphere_host_not_responding",
            "vsphere_host_red",
            "vsphere_host_maintenance",
            "vsphere_datastore_almost_full",
            "vsphere_datastore_inaccessible",
            "vsphere_vm_tools_not_running",
            "vsphere_alarm_red",
            // Redis, MongoDB, RabbitMQ, CrowdSec.
            "redis_memory_near_limit",
            "redis_rejecting_connections",
            "redis_replication_broken",
            "redis_replica_lagging",
            "redis_save_failed",
            "redis_aof_write_failed",
            "mongodb_no_primary",
            "mongodb_member_unhealthy",
            "mongodb_replication_lag",
            "mongodb_connections_high",
            "mongodb_cache_dirty",
            "mongodb_assertions",
            "rabbitmq_resource_alarm",
            "rabbitmq_node_down",
            "rabbitmq_partition",
            "rabbitmq_queue_no_consumer",
            "rabbitmq_queue_not_running",
            "crowdsec_lapi_down",
            "crowdsec_bouncer_stale",
            "crowdsec_no_logs_read",
            "traefik_router_disabled",
            "traefik_server_down",
            "traefik_service_down",
            "traefik_certificate_missing",
            "caddy_upstream_unhealthy",
            "caddy_reload_failed",
            "npm_host_offline",
            "proxy_cert_expiring",
            "proxy_5xx_rate",
            "domain_expiring",
            "domain_expiring_soon",
            "domain_on_hold",
            "domain_redemption",
            "dns_answer_changed",
            // Sauvegarde locale de l'instance (`backup/local.rs`).
            "instance_backup_missing",
        ] {
            assert!(uids.contains(&attendu), "missing built-in rule: {attendu}");
        }
    }

    /// Une règle qui vise une métrique inexistante ne déclenche jamais et ne se
    /// signale pas : elle donne l'illusion d'une surveillance en place. Ce test fige
    /// donc la correspondance avec les noms réellement produits par les collecteurs.
    #[test]
    fn les_regles_livrees_ne_visent_que_des_metriques_produites() {
        // Noms produits par les profils SNMP (`profiles/*.yaml`), les collecteurs
        // Proxmox VE et PBS et le registre, préfixe `dumbmonit_` inclus.
        const PRODUITES: &[&str] = &[
            "dumbmonit_up",
            "dumbmonit_cpu_load_percent",
            "dumbmonit_cpu_usage_percent",
            "dumbmonit_storage_bytes_used",
            "dumbmonit_storage_bytes_total",
            "dumbmonit_ups_output_source",
            "dumbmonit_ups_battery_status",
            // IF-MIB (`profiles/if-mib.yaml`).
            "dumbmonit_if_errors_in",
            "dumbmonit_if_errors_out",
            "dumbmonit_if_oper_status",
            // Redfish (`collectors/redfish/metrics.rs`).
            "dumbmonit_redfish_fan_health",
            "dumbmonit_redfish_temperature_celsius",
            "dumbmonit_redfish_temperature_upper_critical_celsius",
            "dumbmonit_redfish_power_redundancy_health",
            "dumbmonit_redfish_psu_health",
            "dumbmonit_redfish_drive_failure_predicted",
            "dumbmonit_redfish_system_health",
            "dumbmonit_proxmox_node_cpu_percent",
            "dumbmonit_proxmox_node_rootfs_percent",
            "dumbmonit_proxmox_storage_used_percent",
            "dumbmonit_proxmox_backup_last_age_seconds",
            // Proxmox Backup Server (`collectors/pbs/metrics.rs`, `backup.rs`).
            "dumbmonit_pbs_datastore_used_percent",
            "dumbmonit_pbs_datastore_estimated_full_seconds",
            "dumbmonit_pbs_backup_last_age_seconds",
            "dumbmonit_pbs_backup_last_verified",
            "dumbmonit_pbs_tasks_failed",
            "dumbmonit_pbs_gc_last_success_age_seconds",
            // Moniteurs de disponibilité (`collectors/uptime/outcome.rs`, `tls/mod.rs`).
            "dumbmonit_probe_success",
            "dumbmonit_probe_duration_seconds",
            "dumbmonit_probe_ssl_cert_expiry_days",
            // Agent : conteneurs Docker et sauvegardes Plakar (`crates/agent`).
            "dumbmonit_container_up",
            "dumbmonit_container_health",
            "dumbmonit_container_restart_count",
            "dumbmonit_container_update_available",
            "dumbmonit_backup_last_success_seconds",
            "dumbmonit_backup_last_status",
            // Agent : matériel de la machine (`crates/agent/src/collect/
            // {smart,zfs,sensors}.rs`).
            "dumbmonit_agent_disk_smart_ok",
            "dumbmonit_agent_zfs_pool_health",
            "dumbmonit_agent_zfs_pool_scrub_errors",
            "dumbmonit_agent_zfs_pool_data_errors",
            "dumbmonit_agent_sensor_temperature_celsius",
            "dumbmonit_agent_sensor_temperature_critical_celsius",
            // Synology Active Backup for Business (`collectors/synology/abb.rs`).
            "dumbmonit_abb_task_last_status",
            "dumbmonit_abb_task_last_success_seconds",
            "dumbmonit_abb_task_enabled",
            // Synology DSM : stockage, disques, mémoire et rythme Active Backup
            // (`collectors/synology/{metrics,devices}.rs`).
            "dumbmonit_synology_disk_smart_status",
            "dumbmonit_synology_disk_status",
            "dumbmonit_synology_disk_bad_sector_exceeded",
            "dumbmonit_synology_disk_unc_count",
            "dumbmonit_synology_disk_remaining_life_percent",
            "dumbmonit_synology_volume_status",
            "dumbmonit_synology_pool_status",
            "dumbmonit_synology_ssd_cache_status",
            "dumbmonit_synology_volume_used_percent",
            "dumbmonit_synology_disk_temperature_celsius",
            "dumbmonit_synology_temperature_warning",
            "dumbmonit_synology_memory_usage_percent",
            "dumbmonit_abb_device_overdue",
            "dumbmonit_abb_device_consecutive_failures",
            // Sauvegarde locale de l'instance (`backup/local.rs`).
            "dumbmonit_instance_backup_last_success_seconds",
            // Proxmox VE, parité avec Pulse (`collectors/proxmox/{metrics,ha,
            // snapshots,replication,ceph}.rs`).
            "dumbmonit_proxmox_guest_running",
            "dumbmonit_proxmox_ha_resource_error",
            "dumbmonit_proxmox_cluster_quorate",
            "dumbmonit_proxmox_node_up",
            "dumbmonit_proxmox_backup_job_last_ok",
            "dumbmonit_proxmox_guest_snapshot_oldest_age_seconds",
            "dumbmonit_proxmox_replication_job_error",
            "dumbmonit_proxmox_ceph_health",
            "dumbmonit_proxmox_node_updates_pending",
            "dumbmonit_proxmox_node_certificate_expiry_days",
            // PBS, travaux de synchronisation et mises à jour (`collectors/pbs/jobs.rs`).
            "dumbmonit_pbs_sync_job_last_ok",
            "dumbmonit_pbs_node_updates_pending",
            // Proxmox VE, invités, disques, ZFS et paquets
            // (`collectors/proxmox/{metrics,guest,disks,apt}.rs`).
            "dumbmonit_proxmox_guest_cpu_percent",
            "dumbmonit_proxmox_guest_memory_percent",
            "dumbmonit_proxmox_guest_disk_used_percent",
            "dumbmonit_proxmox_node_disk_wearout_percent",
            "dumbmonit_proxmox_node_disk_smart_failed",
            "dumbmonit_proxmox_node_zfs_pool_degraded",
            "dumbmonit_proxmox_node_updates_security_pending",
            "dumbmonit_proxmox_node_packages_changed",
            // Proxmox VE, profondeur ajoutée aux nœuds, à Ceph, à la HA et aux
            // sauvegardes (`collectors/proxmox/{node,ceph,ha,backup,resources}.rs`).
            "dumbmonit_proxmox_node_thinpool_used_percent",
            "dumbmonit_proxmox_node_thinpool_metadata_used_percent",
            "dumbmonit_proxmox_node_core_services_down",
            "dumbmonit_proxmox_node_interface_offline",
            "dumbmonit_proxmox_ceph_osd_up",
            "dumbmonit_proxmox_ceph_osd_used_percent",
            "dumbmonit_proxmox_ceph_pool_used_percent",
            "dumbmonit_proxmox_ceph_flag",
            "dumbmonit_proxmox_ha_lrm_stale",
            "dumbmonit_proxmox_backup_job_guest_excluded_volumes",
            "dumbmonit_proxmox_guest_locked",
            // PBS, travaux, GC et disques (`collectors/pbs/{jobs,backup,metrics}.rs`).
            "dumbmonit_pbs_job_last_ok",
            "dumbmonit_pbs_gc_last_run_ok",
            "dumbmonit_pbs_node_disk_smart_failed",
            "dumbmonit_pbs_node_disk_wearout_percent",
            "dumbmonit_pbs_node_zfs_pool_degraded",
            "dumbmonit_pbs_node_service_active",
            "dumbmonit_pbs_node_certificate_expires_seconds",
            "dumbmonit_pbs_node_running_version_stale",
            "dumbmonit_pbs_tape_backup_job_last_ok",
            "dumbmonit_pbs_job_never_run",
            "dumbmonit_pbs_gc_bad_chunks",
            "dumbmonit_pbs_datastore_removable_unmounted",
            // Proxmox Mail Gateway (`collectors/pmg/metrics.rs`).
            "dumbmonit_pmg_queue_messages",
            "dumbmonit_pmg_queue_oldest_age_seconds",
            "dumbmonit_pmg_quarantine_messages",
            "dumbmonit_pmg_service_running",
            "dumbmonit_pmg_signature_age_seconds",
            "dumbmonit_pmg_cluster_node_insync",
            "dumbmonit_pmg_certificate_expires_in_seconds",
            "dumbmonit_pmg_node_updates_pending",
            // Proxmox Datacenter Manager (`collectors/pdm/metrics.rs`).
            "dumbmonit_pdm_remote_reachable",
            "dumbmonit_pdm_remote_version_behind",
            "dumbmonit_pdm_remote_tasks_failed",
            "dumbmonit_pdm_node_rootfs_percent",
            "dumbmonit_pdm_node_certificate_expiry_days",
            "dumbmonit_pdm_node_updates_pending",
            // OPNsense (`collectors/opnsense/metrics.rs`).
            "dumbmonit_opnsense_gateway_up",
            "dumbmonit_opnsense_gateway_loss_percent",
            "dumbmonit_opnsense_gateway_delay_seconds",
            "dumbmonit_opnsense_pf_states_used_percent",
            "dumbmonit_opnsense_mbuf_used_percent",
            "dumbmonit_opnsense_vpn_tunnel_up",
            "dumbmonit_opnsense_unbound_running",
            "dumbmonit_opnsense_service_running",
            "dumbmonit_opnsense_temperature_celsius",
            "dumbmonit_opnsense_carp_maintenance_mode",
            "dumbmonit_opnsense_firmware_reboot_required",
            "dumbmonit_opnsense_firmware_updates_pending",
            // TrueNAS (`collectors/truenas/metrics.rs`).
            "dumbmonit_truenas_pool_healthy",
            "dumbmonit_truenas_pool_device_errors",
            "dumbmonit_truenas_pool_used_percent",
            "dumbmonit_truenas_pool_last_scrub_errors",
            "dumbmonit_truenas_pool_last_scrub_age_seconds",
            "dumbmonit_truenas_disk_smart_failed",
            "dumbmonit_truenas_disk_temperature_celsius",
            "dumbmonit_truenas_dataset_quota_used_percent",
            "dumbmonit_truenas_replication_error",
            "dumbmonit_truenas_snapshot_task_error",
            "dumbmonit_truenas_snapshot_task_last_run_age_seconds",
            "dumbmonit_truenas_alerts",
            "dumbmonit_truenas_service_running",
            // Serveurs de journaux et de métriques (`collectors/observability/{victoria,
            // loki,graylog}.rs`).
            "dumbmonit_victoriametrics_read_only",
            "dumbmonit_victorialogs_read_only",
            "dumbmonit_victoriametrics_disk_headroom_percent",
            "dumbmonit_victorialogs_disk_headroom_percent",
            "dumbmonit_victoriametrics_healthy",
            "dumbmonit_victorialogs_healthy",
            "dumbmonit_victoriametrics_rows_rejected_total",
            "dumbmonit_victorialogs_rows_rejected_total",
            "dumbmonit_victoriametrics_slow_inserts_total",
            "dumbmonit_victoriametrics_rows_added_total",
            "dumbmonit_loki_ready",
            "dumbmonit_loki_discarded_lines_total",
            "dumbmonit_loki_flush_failures_total",
            "dumbmonit_loki_wal_disk_full_failures_total",
            "dumbmonit_loki_request_errors_total",
            "dumbmonit_graylog_processing",
            "dumbmonit_graylog_indexer_status",
            "dumbmonit_graylog_journal_used_percent",
            "dumbmonit_graylog_inputs_failed",
            "dumbmonit_graylog_output_failures_total",
            "dumbmonit_graylog_processing_failures_total",
            // Pi-hole (`collectors/pihole/metrics.rs`).
            "dumbmonit_pihole_blocking_enabled",
            "dumbmonit_pihole_gravity_age_seconds",
            "dumbmonit_pihole_updates_available",
            "dumbmonit_pihole_messages",
            // AdGuard Home (`collectors/adguard/metrics.rs`).
            "dumbmonit_adguard_protection_enabled",
            "dumbmonit_adguard_running",
            "dumbmonit_adguard_upstream_up",
            "dumbmonit_adguard_filter_oldest_update_age_seconds",
            "dumbmonit_adguard_update_available",
            // Onduleurs derrière NUT (`collectors/nut/metrics.rs`).
            "dumbmonit_nut_ups_on_battery",
            "dumbmonit_nut_ups_low_battery",
            "dumbmonit_nut_ups_replace_battery",
            "dumbmonit_nut_ups_load_percent",
            "dumbmonit_nut_battery_runtime_seconds",
            "dumbmonit_nut_ups_data_stale",
            // MikroTik RouterOS (`collectors/mikrotik/metrics.rs`).
            "dumbmonit_mikrotik_cpu_load_percent",
            "dumbmonit_mikrotik_memory_used_percent",
            "dumbmonit_mikrotik_storage_used_percent",
            "dumbmonit_mikrotik_temperature_celsius",
            "dumbmonit_mikrotik_health_ok",
            "dumbmonit_mikrotik_interface_rx_errors_total",
            "dumbmonit_mikrotik_interface_tx_errors_total",
            "dumbmonit_mikrotik_interface_link_downs_total",
            "dumbmonit_mikrotik_update_available",
            "dumbmonit_mikrotik_firmware_upgrade_pending",
            // MDaemon et SecurityGateway (`collectors/mdaemon`).
            "dumbmonit_mdaemon_service_up",
            "dumbmonit_mdaemon_api_up",
            // MDaemon, compteurs de performance lus par l'agent Windows
            // (`crates/agent/src/collect/perf_counters.rs`).
            "dumbmonit_mdaemon_queue_messages",
            "dumbmonit_securitygateway_service_up",
            "dumbmonit_securitygateway_api_up",
            "dumbmonit_securitygateway_counter",
            // Applications auto-hébergées (`collectors/selfhosted/{nextcloud,immich,
            // paperless,jellyfin,plex}.rs`).
            "dumbmonit_nextcloud_maintenance",
            "dumbmonit_nextcloud_needs_db_upgrade",
            "dumbmonit_nextcloud_update_available",
            "dumbmonit_nextcloud_opcache_full",
            "dumbmonit_nextcloud_free_space_bytes",
            "dumbmonit_immich_queue_waiting",
            "dumbmonit_immich_queue_active",
            "dumbmonit_immich_queue_paused",
            "dumbmonit_immich_storage_used_percent",
            "dumbmonit_immich_update_available",
            "dumbmonit_paperless_component_status",
            "dumbmonit_paperless_unapplied_migrations",
            "dumbmonit_paperless_tasks_failed_recent",
            "dumbmonit_paperless_storage_used_percent",
            "dumbmonit_paperless_update_available",
            "dumbmonit_jellyfin_scheduled_tasks_failed",
            "dumbmonit_jellyfin_plugins_broken",
            "dumbmonit_jellyfin_pending_restart",
            "dumbmonit_plex_update_available",
            // UniFi, Home Assistant, vSphere (`collectors/{unifi,homeassistant,vsphere}`).
            "dumbmonit_unifi_device_up",
            "dumbmonit_unifi_internet_up",
            "dumbmonit_unifi_wan_link_up",
            "dumbmonit_unifi_device_cpu_percent",
            "dumbmonit_unifi_devices",
            "dumbmonit_unifi_devices_upgradable",
            "dumbmonit_unifi_alarms",
            "dumbmonit_homeassistant_recovery_mode",
            "dumbmonit_homeassistant_running",
            "dumbmonit_homeassistant_entities_unavailable",
            "dumbmonit_homeassistant_battery_low",
            "dumbmonit_homeassistant_updates_available",
            "dumbmonit_homeassistant_repair",
            "dumbmonit_vsphere_host_connection_state",
            "dumbmonit_vsphere_host_status",
            "dumbmonit_vsphere_host_maintenance",
            "dumbmonit_vsphere_datastore_used_percent",
            "dumbmonit_vsphere_datastore_accessible",
            "dumbmonit_vsphere_vm_tools_status",
            "dumbmonit_vsphere_alarm",
            // Redis, MongoDB, RabbitMQ et CrowdSec (`collectors/{redis/info,
            // mongodb/status,rabbitmq,crowdsec}.rs`).
            "dumbmonit_redis_memory_used_percent",
            "dumbmonit_redis_rejected_connections_total",
            "dumbmonit_redis_master_link_up",
            "dumbmonit_redis_replica_lag_seconds",
            "dumbmonit_redis_rdb_last_save_ok",
            "dumbmonit_redis_aof_last_write_ok",
            "dumbmonit_mongodb_replset_primary_present",
            "dumbmonit_mongodb_replset_member_health",
            "dumbmonit_mongodb_replset_member_lag_seconds",
            "dumbmonit_mongodb_connections_used_percent",
            "dumbmonit_mongodb_wiredtiger_cache_dirty_percent",
            "dumbmonit_mongodb_asserts_total",
            "dumbmonit_rabbitmq_alarm",
            "dumbmonit_rabbitmq_node_running",
            "dumbmonit_rabbitmq_node_partitions",
            "dumbmonit_rabbitmq_queue_messages_ready",
            "dumbmonit_rabbitmq_queue_consumers",
            "dumbmonit_rabbitmq_queue_running",
            "dumbmonit_crowdsec_lapi_up",
            "dumbmonit_crowdsec_bouncer_requests_total",
            "dumbmonit_crowdsec_lines_read_total",
            // Proxys inverses et domaines (`collectors/{traefik,caddy,npm,domain}`).
            "dumbmonit_traefik_router_status",
            "dumbmonit_traefik_server_up",
            "dumbmonit_traefik_service_servers_up",
            "dumbmonit_traefik_resolver_routers_uncovered",
            "dumbmonit_traefik_cert_expiry_days",
            "dumbmonit_traefik_requests_total",
            "dumbmonit_traefik_requests_5xx_total",
            "dumbmonit_caddy_upstream_healthy",
            "dumbmonit_caddy_config_last_reload_ok",
            "dumbmonit_caddy_requests_total",
            "dumbmonit_caddy_requests_5xx_total",
            "dumbmonit_npm_host_online",
            "dumbmonit_npm_cert_expiry_days",
            "dumbmonit_domain_expiry_days",
            "dumbmonit_domain_on_hold",
            "dumbmonit_domain_redemption",
            // Sonde DNS, option `alert_on_change` (`uptime/dns`).
            "dumbmonit_probe_dns_answer_fingerprint",
        ];

        for rule in builtin_rules() {
            for mot in rule.query.split(|c: char| !c.is_alphanumeric() && c != '_') {
                if mot.starts_with("dumbmonit_") {
                    assert!(
                        PRODUITES.contains(&mot),
                        "rule \"{}\" targets \"{mot}\", which no collector produces",
                        rule.uid
                    );
                }
            }
        }
    }

    #[test]
    fn les_identifiants_des_regles_livrees_sont_uniques() {
        let rules = builtin_rules();
        let mut uids: Vec<&str> = rules.iter().map(|r| r.uid.as_str()).collect();
        uids.sort_unstable();
        let total = uids.len();
        uids.dedup();
        assert_eq!(uids.len(), total, "two built-in rules share a uid");
    }

    #[test]
    fn les_regles_livrees_sont_actives_et_sans_canal_impose() {
        for rule in builtin_rules() {
            assert!(rule.enabled, "{} should be enabled", rule.uid);
            assert!(rule.builtin);
            assert!(rule.channels.is_empty(), "{} must not impose any channel", rule.uid);
            assert!(matches!(rule.selector, TargetSelector::All));
        }
    }

    #[test]
    fn seule_la_regle_injoignable_porte_l_uid_reserve() {
        let host_down: Vec<_> = builtin_rules().into_iter().filter(|r| r.is_host_down()).collect();
        assert_eq!(host_down.len(), 1);
        assert_eq!(host_down[0].severity, Severity::Critical);
    }

    #[test]
    fn la_requete_predictive_reste_du_metricsql() {
        let query = predict_full_query("dumbmonit_fs_used_percent", 6, 4);
        assert_eq!(
            query,
            "predict_linear(dumbmonit_fs_used_percent[6h], 345600) \
             and deriv(dumbmonit_fs_used_percent[6h]) > 0"
        );
    }

    #[test]
    fn toutes_les_regles_livrees_ont_un_for_non_nul_sauf_justification() {
        for rule in builtin_rules() {
            assert!(
                rule.for_duration > Duration::ZERO,
                "{} would fire on the first point, with no noise filter",
                rule.uid
            );
        }
    }
}
