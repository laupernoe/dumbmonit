use std::collections::BTreeMap;

use dumbmonit_proto::{Credential, Target};

use super::*;

const NOW_MS: i64 = 1_792_000_000_000;

fn target(address: &str, tags: &[(&str, &str)]) -> Target {
    Target {
        id: 7,
        name: "srv".into(),
        address: address.into(),
        kind: "winrm".into(),
        profile_id: None,
        parent_id: None,
        interval: Duration::from_secs(60),
        enabled: true,
        tags: tags.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        credential: Credential::UsernamePassword {
            username: "CORP\\svc-dumbmonit".into(),
            password: "secret".into(),
        },
        group_name: String::new(),
        position: 0,
    }
}

fn row(pairs: &[(&str, &str)]) -> Row {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

fn options(tags: &[(&str, &str)]) -> Options {
    Options::from_target(&target("srv01.corp.example.com", tags)).expect("options valides")
}

fn find<'a>(samples: &'a [Sample], metric: &str) -> Vec<&'a Sample> {
    samples.iter().filter(|s| s.metric == metric).collect()
}

fn value(samples: &[Sample], metric: &str) -> Option<f64> {
    find(samples, metric).first().map(|s| s.value)
}

fn replies() -> Replies {
    let os = wsman::parse_page(include_str!("testdata/enumerate_os.xml")).unwrap().rows;
    Replies {
        os,
        cpu: Some(vec![
            row(&[("Name", "0"), ("PercentProcessorTime", "10")]),
            row(&[("Name", "1"), ("PercentProcessorTime", "30")]),
            row(&[("Name", "_Total"), ("PercentProcessorTime", "20")]),
        ]),
        pagefile: Some(vec![row(&[("AllocatedBaseSize", "2048"), ("CurrentUsage", "512")])]),
        disks: Some(vec![row(&[
            ("DeviceID", "C:"),
            ("FileSystem", "NTFS"),
            ("Size", "100000"),
            ("FreeSpace", "25000"),
        ])]),
        services: Some(vec![
            row(&[("Name", "Spooler"), ("State", "Running"), ("StartMode", "Auto")]),
            row(&[("Name", "W3SVC"), ("State", "Stopped"), ("StartMode", "Auto")]),
            row(&[("Name", "gupdate"), ("State", "Stopped"), ("StartMode", "Auto")]),
            row(&[("Name", "Fax"), ("State", "Stopped"), ("StartMode", "Manual")]),
            row(&[("Name", "MSSQLSERVER"), ("State", "Stopped"), ("StartMode", "Auto")]),
        ]),
        events: vec![(
            "System".into(),
            Some(vec![
                row(&[("EventCode", "7034"), ("SourceName", "Service Control Manager")]),
                row(&[("EventCode", "7034"), ("SourceName", "Service Control Manager")]),
                row(&[("EventCode", "41"), ("SourceName", "Microsoft-Windows-Kernel-Power")]),
            ]),
        )],
    }
}

#[test]
fn les_metriques_portent_les_noms_de_l_agent_windows() {
    let out = samples(&replies(), &options(&[]), NOW_MS);
    assert_eq!(value(&out, "cpu_usage_percent"), Some(20.0));
    assert_eq!(value(&out, "cpu_count"), Some(2.0));
    assert_eq!(find(&out, "cpu_core_usage_percent").len(), 2);
    assert_eq!(value(&out, "memory_total_bytes"), Some(16_776_244.0 * 1024.0));
    assert_eq!(value(&out, "memory_available_bytes"), Some(9_980_612.0 * 1024.0));
    let used = value(&out, "memory_used_percent").unwrap();
    assert!((used - 40.51).abs() < 0.01, "{used}");
    assert_eq!(value(&out, "swap_used_percent"), Some(25.0));
    assert_eq!(value(&out, "process_count"), Some(112.0));
    let disk = find(&out, "filesystem_used_percent")[0];
    assert_eq!(disk.value, 75.0);
    assert_eq!(disk.labels["mountpoint"], "C:\\");
    assert_eq!(disk.labels["fstype"], "NTFS");
}

#[test]
fn l_uptime_vient_du_dernier_demarrage() {
    // 2026-10-20T06:11:12Z = 1_792_476_672 ; NOW_MS est antérieur : borné à zéro.
    let out = samples(&replies(), &options(&[]), NOW_MS);
    assert_eq!(value(&out, "uptime_seconds"), Some(0.0));
    let later = 1_792_476_672_000 + 3_600_000;
    let out = samples(&replies(), &options(&[]), later);
    assert_eq!(value(&out, "uptime_seconds"), Some(3600.0));
}

#[test]
fn les_services_automatiques_arretes_sont_signales_sauf_exceptions() {
    let out = samples(&replies(), &options(&[("ignore_services", "MSSQL*")]), NOW_MS);
    // Spooler tourne, gupdate est une exception intégrée, MSSQL* est ignoré, Fax est manuel.
    assert_eq!(value(&out, "windows_services_auto_stopped"), Some(1.0));
    let down: Vec<_> = find(&out, "service_up").into_iter().filter(|s| s.value == 0.0).collect();
    assert_eq!(down.len(), 1);
    assert_eq!(down[0].labels["service"], "W3SVC");
}

#[test]
fn un_service_choisi_absent_compte_comme_arrete() {
    let out = samples(&replies(), &options(&[("services", "spooler, Absent")]), NOW_MS);
    let up = find(&out, "service_up");
    let get = |name: &str| up.iter().find(|s| s.labels["service"] == name).map(|s| s.value);
    assert_eq!(get("Spooler"), Some(1.0));
    assert_eq!(get("Absent"), Some(0.0));
}

#[test]
fn le_journal_est_compte_par_identifiant() {
    let out = samples(&replies(), &options(&[]), NOW_MS);
    let errors = find(&out, "windows_eventlog_errors");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].value, 3.0);
    assert_eq!(errors[0].labels["log"], "System");
    let events = find(&out, "windows_eventlog_events");
    assert_eq!(events[0].labels["event_id"], "7034");
    assert_eq!(events[0].value, 2.0);
    assert_eq!(value(&out, "winrm_section_readable"), Some(1.0));
}

#[test]
fn une_section_illisible_est_marquee_sans_faire_echouer_la_collecte() {
    let mut r = replies();
    r.disks = None;
    r.events = vec![("Security".into(), None)];
    let out = samples(&r, &options(&[]), NOW_MS);
    assert!(find(&out, "filesystem_total_bytes").is_empty());
    let marks: BTreeMap<_, _> = find(&out, "winrm_section_readable")
        .into_iter()
        .map(|s| (s.labels["section"].clone(), s.value))
        .collect();
    assert_eq!(marks["disks"], 0.0);
    assert_eq!(marks["eventlog"], 0.0);
    assert_eq!(marks["cpu"], 1.0);
}

#[test]
fn les_dates_wmi_sont_lues_dans_les_deux_formes() {
    assert_eq!(parse_wmi_time("2026-10-20T06:11:12.500000+00:00"), Some(1_792_476_672));
    assert_eq!(parse_wmi_time("20261020061112.500000+000"), Some(1_792_476_672));
    // 08:11:12 à +120 minutes = 06:11:12 UTC.
    assert_eq!(parse_wmi_time("20261020081112.000000+120"), Some(1_792_476_672));
    assert_eq!(parse_wmi_time("n'importe quoi"), None);
}

#[test]
fn les_motifs_de_services_acceptent_un_joker() {
    assert!(matches_pattern("MSSQL*", "MSSQLSERVER"));
    assert!(matches_pattern("*update", "gupdate"));
    assert!(matches_pattern("*sql*", "MSSQLSERVER"));
    assert!(matches_pattern("spooler", "Spooler"));
    assert!(!matches_pattern("spooler", "Spooler2"));
}

#[test]
fn les_options_par_defaut_visent_https_5986() {
    let o = options(&[]);
    assert_eq!(o.endpoint, "https://srv01.corp.example.com:5986/wsman");
    assert_eq!(o.auth, AuthMode::Ntlm);
    assert_eq!(o.event_logs, vec!["System", "Application"]);
    assert_eq!(o.event_minutes, 60);
    assert!(!o.insecure_tls);
}

#[test]
fn le_trafic_non_chiffre_exige_la_case_et_refuse_basic() {
    let refused = Options::from_target(&target("http://srv01", &[]));
    assert!(matches!(refused, Err(ProbeError::Config(_))));
    let o = options(&[("allow_unencrypted", "true")]);
    assert_eq!(o.endpoint, "http://srv01.corp.example.com:5985/wsman");
    let basic = Options::from_target(&target(
        "srv01",
        &[("allow_unencrypted", "true"), ("auth", "basic")],
    ));
    assert!(matches!(basic, Err(ProbeError::Config(_))));
    assert_eq!(options(&[("auth", "basic")]).auth, AuthMode::Basic);
    assert!(Options::from_target(&target("srv01", &[("auth", "kerberos")])).is_err());
}

#[test]
fn les_noms_de_journaux_sont_valides_avant_d_entrer_dans_le_wql() {
    let bad = Options::from_target(&target("srv01", &[("event_logs", "System' OR 1=1")]));
    assert!(matches!(bad, Err(ProbeError::Config(_))));
    assert!(options(&[("event_logs", "none")]).event_logs.is_empty());
    assert_eq!(options(&[("event_logs", "")]).event_logs.len(), 2);
}

#[test]
fn la_requete_du_journal_cible_les_erreurs_ou_les_echecs_d_audit() {
    let wql = event_wql("System", "20261026100000.000000+000");
    assert!(wql.contains("Logfile = 'System'") && wql.contains("Type = 'Error'"));
    assert!(event_wql("Security", "x").contains("Audit Failure"));
}

#[test]
fn le_compte_est_decoupe_en_domaine_et_nom() {
    assert_eq!(split_account("CORP\\alice"), ("alice", "CORP"));
    assert_eq!(split_account("alice@corp.example.com"), ("alice@corp.example.com", ""));
    assert_eq!(split_account("alice"), ("alice", ""));
}

#[test]
fn l_identifiant_de_message_a_la_forme_d_un_uuid() {
    let id = message_id();
    assert_eq!(id.len(), 36);
    assert_eq!(id.matches('-').count(), 4);
}
