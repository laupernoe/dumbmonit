//! La migration 0232 remplace les noms lisibles des jetons d'enrôlement par des
//! identifiants opaques, sans toucher à ceux qui en sont déjà.

use dumbmonit_server::db;

fn is_opaque(name: &str) -> bool {
    name.strip_prefix("tok_").is_some_and(|rest| {
        rest.len() == 10
            && rest.bytes().all(|b| b.is_ascii_lowercase() || (b'2'..=b'7').contains(&b))
    })
}

#[tokio::test]
async fn existing_token_names_become_opaque_and_stay_so() {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let path = dir.path().join(db::DATABASE_FILE);
    let old = db::open_up_to(&path, 231).await.expect("base ancienne");
    for (hash, name) in [
        ("h1", "Agent enrolment · 2026-11-02"),
        ("h2", "portable de Marie"),
        ("h3", "Fleet enrolment · 2026-10-01"),
        ("h4", "tok_abcdefghij"),
    ] {
        sqlx::query("INSERT INTO agent_tokens (name, token_hash, prefix) VALUES (?, ?, 'dmon_x')")
            .bind(name)
            .bind(hash)
            .execute(&old)
            .await
            .expect("jeton ancien");
    }
    old.close().await;

    let pool = db::open(&path).await.expect("migration");
    let names: Vec<(String, String)> =
        sqlx::query_as("SELECT token_hash, name FROM agent_tokens ORDER BY token_hash")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(names.len(), 4);
    for (_, name) in &names {
        assert!(is_opaque(name), "{name}");
    }
    // Un jeton déjà au bon format garde son identifiant.
    assert_eq!(names[3].1, "tok_abcdefghij");
    let mut distinct: Vec<&String> = names.iter().map(|(_, n)| n).collect();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), 4);
    pool.close().await;

    // Une seconde ouverture ne retouche rien.
    let again = db::open(&path).await.expect("réouverture");
    let after: Vec<(String, String)> =
        sqlx::query_as("SELECT token_hash, name FROM agent_tokens ORDER BY token_hash")
            .fetch_all(&again)
            .await
            .unwrap();
    assert_eq!(names, after);
}
