//! Tâches en échec d'un nœud : `GET /nodes/{node}/tasks?errors=1`.
//!
//! Les sauvegardes ont leurs propres métriques (`backup.rs`). Tout le reste —
//! démarrage d'un invité refusé, migration avortée, réplication, restauration,
//! mise à jour de l'index des paquets — ne laisse de trace que dans le journal
//! des tâches, que personne ne relit. On compte donc les tâches terminées en
//! échec sur une fenêtre courte, et la règle livrée avertit sans bruit.

use dumbmonit_proto::Sample;

use super::metrics::gauge;
use super::model::TaskEntry;

/// Fenêtre d'observation : assez longue pour qu'un échec nocturne soit vu au
/// matin, assez courte pour que l'alerte retombe d'elle-même.
pub const WINDOW_SECONDS: i64 = 6 * 3600;

/// Types de tâches qui ne sont pas des incidents : sessions interactives qu'on
/// ferme, ou interruptions voulues, et les sauvegardes déjà mesurées ailleurs.
const IGNORED_TYPES: [&str; 8] = [
    "vzdump",
    "vncproxy",
    "vncshell",
    "spiceproxy",
    "spiceshell",
    "termproxy",
    "aptupdate",
    "login",
];

/// Une tâche terminée en échec : ni succès, ni simple avertissement, ni arrêt
/// demandé par un humain.
fn is_failure(task: &TaskEntry, floor: i64) -> bool {
    let Some(status) = task.status.as_deref() else { return false };
    if !task.is_finished() || task.succeeded() {
        return false;
    }
    let status = status.trim();
    // `WARNINGS: 2` : la tâche a abouti.
    if status.is_empty() || status.to_ascii_uppercase().starts_with("WARNINGS") {
        return false;
    }
    if status.eq_ignore_ascii_case("interrupted by signal") {
        return false;
    }
    if task.task_type.as_deref().is_some_and(|kind| IGNORED_TYPES.contains(&kind)) {
        return false;
    }
    task.starttime.is_some_and(|start| start.0 as i64 >= floor)
}

/// Le nombre de tâches en échec sur la fenêtre, étiqueté par nœud.
pub fn failed_task_samples(
    node: &str,
    tasks: &[TaskEntry],
    now_s: i64,
    ts_ms: i64,
) -> Vec<Sample> {
    let floor = now_s - WINDOW_SECONDS;
    let failed = tasks.iter().filter(|task| is_failure(task, floor)).count();
    vec![gauge("node_tasks_failed", failed as f64, ts_ms).with_label("node", node)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tasks() -> Vec<TaskEntry> {
        serde_json::from_str(
            r#"[
              {"type":"qmstart","id":"104","starttime":1724099000,"endtime":1724099002,
               "status":"start failed: org.freedesktop.systemd1.UnitExists"},
              {"type":"qmigrate","id":"101","starttime":1724098000,"endtime":1724098100,
               "status":"migration aborted"},
              {"type":"qmstart","id":"100","starttime":1724097000,"endtime":1724097003,
               "status":"OK"},
              {"type":"vzdump","id":"","starttime":1724096000,"endtime":1724096100,
               "status":"job errors"},
              {"type":"vncproxy","id":"100","starttime":1724095000,"endtime":1724095001,
               "status":"command '/usr/bin/termproxy' failed: exit code 1"},
              {"type":"qmrestore","id":"105","starttime":1724094000,"endtime":1724094500,
               "status":"WARNINGS: 1"},
              {"type":"qmstop","id":"102","starttime":1724093000,"endtime":1724093001,
               "status":"interrupted by signal"},
              {"type":"qmstart","id":"103","starttime":1724092000},
              {"type":"srvreload","id":"networking","starttime":1724000000,"endtime":1724000001,
               "status":"command 'ifreload -a' failed"}
            ]"#,
        )
        .unwrap()
    }

    #[test]
    fn seules_les_vraies_pannes_recentes_comptent() {
        let samples = failed_task_samples("pve1", &tasks(), 1_724_100_000, 1000);
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].metric, "proxmox_node_tasks_failed");
        assert_eq!(samples[0].labels["node"], "pve1");
        // Le démarrage et la migration : le reste est un succès, un avertissement,
        // une session, une sauvegarde, un arrêt voulu, une tâche en cours, ou
        // trop ancien.
        assert_eq!(samples[0].value, 2.0);
    }

    #[test]
    fn un_noeud_sans_tache_en_echec_publie_zero() {
        let samples = failed_task_samples("pve1", &[], 1_724_100_000, 1000);
        assert_eq!(samples[0].value, 0.0);
    }
}
