//! Lecture, vérification et extraction d'un paquet, sans réseau.

use std::collections::BTreeMap;

use dumbmonit_pack::{Pack, PackError};
use dumbmonit_proto::{MetricKind, Sample};

/// Un paquet minimal valide ; les tests y ajoutent ou y cassent une section.
const BASE: &str = r#"
schema: 1
id: demo-app
version: 1.2.0
label: Demo app
sources:
  - id: status
    type: http
    path: /api/status
metrics:
  - name: up_value
    source: status
    json: $.up
"#;

fn errors(yaml: &str) -> Vec<String> {
    match Pack::parse(yaml) {
        Ok(_) => panic!("le paquet aurait dû être refusé"),
        Err(PackError { errors }) => errors,
    }
}

fn assert_refused(yaml: &str, needle: &str) {
    let errors = errors(yaml);
    assert!(
        errors.iter().any(|error| error.contains(needle)),
        "« {needle} » absent de {errors:#?}"
    );
}

fn extract(yaml: &str, bodies: &[(&str, &str)]) -> Vec<Sample> {
    let pack = Pack::parse(yaml).unwrap_or_else(|error| panic!("{error}"));
    let bodies: BTreeMap<String, Vec<u8>> =
        bodies.iter().map(|(id, body)| (id.to_string(), body.as_bytes().to_vec())).collect();
    let mut samples = pack.extract(&bodies, 0).expect("extraction");
    samples.sort_by_key(Sample::series_key);
    samples
}

fn keys(samples: &[Sample]) -> Vec<String> {
    samples.iter().map(|s| format!("{} {}", s.series_key(), s.value)).collect()
}

#[test]
fn un_paquet_minimal_est_accepte_et_prend_son_type_et_son_prefixe() {
    let pack = Pack::parse(BASE).unwrap();
    assert_eq!(pack.id(), "demo-app");
    assert_eq!(pack.kind(), "pack.demo-app");
    assert_eq!(pack.metric_prefix(), "demo_app_");
    assert_eq!(pack.metric_names().collect::<Vec<_>>(), ["dumbmonit_demo_app_up_value"]);
    assert_eq!(pack.sha256().len(), 64);
    assert!(pack.has_collector());
    let description = pack.description();
    assert_eq!(description.kind, "pack.demo-app");
    assert_eq!(description.credential_types, ["none"]);
    assert!(description.options.iter().any(|o| o.key == "allow_private_targets"));
}

#[test]
fn les_erreurs_de_forme_sont_toutes_rendues_dun_coup() {
    let yaml = BASE
        .replace("id: demo-app", "id: Demo_App")
        .replace("version: 1.2.0", "version: one")
        .replace("schema: 1", "schema: 2");
    let errors = errors(&yaml);
    assert!(errors.len() >= 3, "{errors:#?}");
    assert!(errors.iter().any(|e| e.starts_with("schema:")));
    assert!(errors.iter().any(|e| e.starts_with("id:")));
    assert!(errors.iter().any(|e| e.starts_with("version:")));
}

#[test]
fn une_cle_inconnue_est_une_erreur_et_pas_un_reglage_ignore() {
    assert_refused(&format!("{BASE}treshold: 3\n"), "unknown field");
}

#[test]
fn une_version_de_dumbmonit_trop_recente_est_refusee() {
    assert_refused(&BASE.replace("label:", "requires: \">=99\"\nlabel:"), "needs DumbMonit >=99");
    assert!(Pack::parse(&BASE.replace("label:", "requires: \">=0.1\"\nlabel:")).is_ok());
}

#[test]
fn un_chemin_ne_peut_pas_designer_un_autre_hote() {
    for path in ["http://evil.lan/x", "//evil.lan/x", "api/status", "/a\\b"] {
        let yaml = BASE.replace("path: /api/status", &format!("path: \"{path}\""));
        assert_refused(&yaml, "sources[status].path");
    }
}

#[test]
fn un_identifiant_ne_peut_aller_que_dans_un_entete() {
    let yaml = BASE.replace(
        "path: /api/status",
        "path: /api/status?key={{credential.token}}\n    headers: {}",
    );
    assert_refused(
        &format!("{yaml}credentials: [api_token]\n"),
        "credentials may only appear in headers",
    );

    let yaml = BASE.replace(
        "path: /api/status",
        "path: /api/status\n    auth: none\n    headers:\n      X-Api-Key: \"{{credential.token}}\"",
    );
    assert_refused(&yaml, "needs api_token in credentials");
    assert!(Pack::parse(&format!("{yaml}credentials: [api_token]\n")).is_ok());
}

#[test]
fn une_option_citee_doit_etre_declaree() {
    let yaml = BASE.replace("path: /api/status", "path: /api/{{option.site}}");
    assert_refused(&yaml, "{{option.site}} is not a declared option");
    let declared = format!("{yaml}options:\n  - key: site\n    label: Site\n    default: main\n");
    assert!(Pack::parse(&declared).is_ok());
    let reserved = format!("{BASE}options:\n  - key: port\n    label: Port\n");
    assert_refused(&reserved, "reserved");
}

#[test]
fn les_regles_ne_citent_que_des_metriques_produites() {
    let rule = |expr: &str| {
        format!(
            "{BASE}rules:\n  - name: down\n    expr: {expr}\n    threshold: 1\n    op: \"<\"\n    for: 5m\n"
        )
    };
    assert!(Pack::parse(&rule("dumbmonit_demo_app_up_value")).is_ok());
    assert_refused(&rule("dumbmonit_demo_app_other"), "dumbmonit_demo_app_other is not produced");
    assert_refused(&rule("dumbmonit_cpu_usage_percent"), "is not produced by this pack");
    assert_refused(&rule("up_value"), "at least one metric of this pack");

    let pack = Pack::parse(&rule("min_over_time(dumbmonit_demo_app_up_value[5m])")).unwrap();
    let installed = &pack.rules()[0];
    assert_eq!(installed.uid, "pack:demo-app:down");
    assert_eq!(installed.for_secs, 300);
    assert_eq!(installed.op, "<");
}

#[test]
fn un_seuil_peut_venir_de_la_valeur_par_defaut_dune_option() {
    let yaml = format!(
        "{BASE}options:\n  - key: limit\n    label: Limit\n    input: number\n    default: 42\n\
         rules:\n  - name: high\n    expr: dumbmonit_demo_app_up_value\n    threshold: \"{{{{option.limit}}}}\"\n"
    );
    let pack = Pack::parse(&yaml).unwrap();
    assert_eq!(pack.rules()[0].threshold, 42.0);
    assert_eq!(pack.rules()[0].threshold_option.as_deref(), Some("limit"));
    assert_refused(&yaml.replace("default: 42", "default: lots"), "is not a number");
}

#[test]
fn un_nom_de_metrique_ne_peut_etre_produit_quune_fois() {
    let yaml = format!("{BASE}  - name: up_value\n    source: status\n    json: $.other\n");
    assert_refused(&yaml, "is produced twice");
}

#[test]
fn le_jsonpath_lit_les_valeurs_et_les_etiquettes_voisines() {
    let yaml = r#"
schema: 1
id: sensors
version: 1.0.0
label: Sensors
sources:
  - id: all
    type: http
    path: /sensors
metrics:
  - name: temperature_celsius
    source: all
    json: $.sensors[*].value
    labels:
      sensor: $.name
  - name: relay_on
    source: all
    json: $.relay
  - name: mode
    source: all
    json: $.mode
    map: { eco: 1, boost: 2 }
  - name: written_bytes
    source: all
    kind: counter
    json: $.written_kib
    scale: 1024
"#;
    let body = r#"{"relay": true, "mode": "boost", "written_kib": 2,
                   "sensors": [{"name": "attic", "value": 21.5}, {"name": "cellar", "value": "12"},
                               {"name": "broken", "value": null}]}"#;
    let samples = extract(yaml, &[("all", body)]);
    assert_eq!(
        keys(&samples),
        [
            "sensors_mode 2",
            "sensors_relay_on 1",
            "sensors_temperature_celsius{sensor=\"attic\"} 21.5",
            "sensors_temperature_celsius{sensor=\"cellar\"} 12",
            "sensors_written_bytes 2048",
        ]
    );
    assert_eq!(samples[4].kind, MetricKind::Counter);
}

#[test]
fn une_valeur_hors_de_lenumeration_ne_produit_rien() {
    let yaml = BASE.replace("json: $.up", "json: $.up\n    map: { yes: 1, no: 0 }");
    assert!(extract(&yaml, &[("status", r#"{"up": "maybe"}"#)]).is_empty());
    assert_eq!(keys(&extract(&yaml, &[("status", r#"{"up": "yes"}"#)])), ["demo_app_up_value 1"]);
}

#[test]
fn la_decouverte_fait_une_serie_par_ligne_dans_la_limite_de_max_rows() {
    let yaml = r#"
schema: 1
id: disks
version: 1.0.0
label: Disks
sources:
  - id: list
    type: http
    path: /disks
discover:
  - name: disks
    source: list
    rows: $.disks[*]
    max_rows: 2
    labels:
      disk: $.name
      model: $.info.model
    metrics:
      - name: used_bytes
        json: $.used
      - name: healthy
        json: $.state
        map: { ok: 1, degraded: 0 }
"#;
    let body = r#"{"disks": [
        {"name": "sda", "used": 10, "state": "ok", "info": {"model": "X1"}},
        {"name": "sdb", "used": 20, "state": "degraded", "info": {"model": "X2"}},
        {"name": "sdc", "used": 30, "state": "ok", "info": {"model": "X3"}}]}"#;
    assert_eq!(
        keys(&extract(yaml, &[("list", body)])),
        [
            "disks_healthy{disk=\"sda\",model=\"X1\"} 1",
            "disks_healthy{disk=\"sdb\",model=\"X2\"} 0",
            "disks_used_bytes{disk=\"sda\",model=\"X1\"} 10",
            "disks_used_bytes{disk=\"sdb\",model=\"X2\"} 20",
        ]
    );
}

#[test]
fn une_expression_reguliere_lit_une_page_texte() {
    let yaml = r#"
schema: 1
id: status-page
version: 1.0.0
label: Status page
sources:
  - id: page
    type: http
    path: /server-status?auto
    format: text
metrics:
  - name: busy_workers
    source: page
    regex: 'BusyWorkers: (\d+)'
  - name: queue_length
    source: page
    regex: 'queue (?P<queue>\w+) = (?P<value>[\d.]+)'
    labels:
      queue: queue
"#;
    let page = "Total Accesses: 12\nBusyWorkers: 3\nqueue mail = 4\nqueue web = 0.5\n";
    assert_eq!(
        keys(&extract(yaml, &[("page", page)])),
        [
            "status_page_busy_workers 3",
            "status_page_queue_length{queue=\"mail\"} 4",
            "status_page_queue_length{queue=\"web\"} 0.5",
        ]
    );
    assert_refused(
        &yaml.replace("labels:\n      queue: queue", "labels:\n      queue: nope"),
        "no named group",
    );
}

#[test]
fn une_source_prometheus_retient_renomme_et_ecarte() {
    let yaml = r#"
schema: 1
id: exporter
version: 1.0.0
label: Exporter
sources:
  - id: metrics
    type: prometheus
    path: /metrics
    keep: [app_requests_total, app_latency_seconds, app_Queue]
    rename:
      app_requests_total: requests_total
      app_Queue: queue
    drop:
      path: ^/health
metrics:
  - name: build_info
    source: metrics
    prom: app_build_info
    labels:
      version: version
"#;
    let text = r#"# TYPE app_requests_total counter
app_requests_total{path="/api",instance="10.0.0.1:80",target="x"} 12
app_requests_total{path="/health"} 99
# TYPE app_latency_seconds histogram
app_latency_seconds_bucket{le="0.1"} 3
app_latency_seconds_sum 0.2
app_latency_seconds_count 3
# TYPE app_Queue gauge
app_Queue 4
app_ignored 1
app_build_info{version="1.2.3",commit="abc"} 1
"#;
    let samples = extract(yaml, &[("metrics", text)]);
    assert_eq!(
        keys(&samples),
        [
            "exporter_app_latency_seconds_bucket{le=\"0.1\"} 3",
            "exporter_app_latency_seconds_count 3",
            "exporter_app_latency_seconds_sum 0.2",
            "exporter_build_info{version=\"1.2.3\"} 1",
            "exporter_queue 4",
            "exporter_requests_total{exported_instance=\"10.0.0.1:80\",exported_target=\"x\",path=\"/api\"} 12",
        ]
    );
    let requests = samples.iter().find(|s| s.metric == "exporter_requests_total").unwrap();
    assert_eq!(requests.kind, MetricKind::Counter);
    let queue = samples.iter().find(|s| s.metric == "exporter_queue").unwrap();
    assert_eq!(queue.kind, MetricKind::Gauge);

    // Les séries d'un histogramme se citent dans une règle, sans compter comme
    // des métriques déclarées.
    assert_eq!(Pack::parse(yaml).unwrap().metric_names().count(), 4);
    let rule = format!(
        "{yaml}rules:\n  - name: slow\n    expr: rate(dumbmonit_exporter_app_latency_seconds_sum[5m])\n    threshold: 1\n"
    );
    assert!(Pack::parse(&rule).is_ok());

    // Sans renommage, un nom en majuscules n'est pas un nom de métrique DumbMonit.
    assert_refused(&yaml.replace("      app_Queue: queue\n", ""), "rename it");
}

#[test]
fn un_extracteur_doit_correspondre_au_format_de_sa_source() {
    assert_refused(
        &BASE.replace("json: $.up", "regex: 'up (\\d+)'"),
        "regex needs an http source in text format",
    );
    assert_refused(&BASE.replace("json: $.up", "json: $.up\n    prom: up"), "exactly one of");
    assert_refused(&BASE.replace("json: $.up", "json: \"$[\""), "invalid JSONPath");
    assert_refused(&BASE.replace("source: status", "source: nope"), "unknown source");
}

#[test]
fn une_etiquette_ne_peut_pas_usurper_lidentite_de_la_cible() {
    let yaml = BASE.replace("json: $.up", "json: $.up\n    labels:\n      host: $.name");
    assert_refused(&yaml, "\"host\" is reserved");
    let yaml = BASE.replace("json: $.up", "json: $.up\n    labels:\n      tag_site: $.name");
    assert_refused(&yaml, "\"tag_site\" is reserved");
}

#[test]
fn un_paquet_snmp_reprend_le_format_des_profils() {
    let yaml = r#"
schema: 1
id: acme-ups
version: 1.0.0
label: ACME UPS
snmp_profiles:
  - id: acme-ups
    name: ACME UPS
    match:
      sysobjectid: [1.3.6.1.4.1.99999]
    metrics:
      - name: acme_ups_battery_percent
        oid: 1.3.6.1.4.1.99999.1.1.0
        kind: gauge
rules:
  - name: battery_low
    expr: dumbmonit_acme_ups_battery_percent
    op: "<"
    threshold: 30
"#;
    let pack = Pack::parse(yaml).unwrap_or_else(|error| panic!("{error}"));
    assert!(!pack.has_collector(), "un paquet SNMP seul n'a pas de type à lui");
    assert_eq!(pack.snmp_profiles()[0].id, "acme-ups");
    assert_refused(
        &yaml.replace("name: acme_ups_battery", "name: battery"),
        "must start with \"acme_ups_\"",
    );
    assert_refused(
        &yaml.replace("  - id: acme-ups\n", "  - id: if-mib\n"),
        "must be \"acme-ups\" or start with",
    );
}
