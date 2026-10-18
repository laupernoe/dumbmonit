//! Connexion LDAP contre un vrai OpenLDAP. Ignoré par défaut : il faut un
//! annuaire. Pour le lancer :
//!
//! ```text
//! docker run -d --name dm-openldap -p 3389:389 \
//!   -e LDAP_ORGANISATION=Example -e LDAP_DOMAIN=example.org \
//!   -e LDAP_ADMIN_PASSWORD=admin-du-test osixia/openldap:1.5.0
//! DUMBMONIT_TEST_LDAP_URL=ldap://127.0.0.1:3389 \
//! DUMBMONIT_TEST_LDAP_ADMIN_DN=cn=admin,dc=example,dc=org \
//! DUMBMONIT_TEST_LDAP_ADMIN_PASSWORD=admin-du-test \
//! DUMBMONIT_TEST_LDAP_BASE_DN=dc=example,dc=org \
//!   cargo test -p dumbmonit-server --test ldap_directory -- --ignored
//! ```
//!
//! Avec `DUMBMONIT_TEST_LDAP_CA_FILE` (l'autorité que l'image génère,
//! `/container/service/slapd/assets/certs/ca.crt`) et une URL dont le nom
//! d'hôte est celui du certificat (`--hostname ldap.example.org`), le test
//! vérifie aussi StartTLS, et LDAPS avec `DUMBMONIT_TEST_LDAP_LDAPS_URL`.
//!
//! Le test peuple lui-même l'annuaire (unités `ou=dm-people` et `ou=dm-groups`,
//! recréées à chaque passage) avec le compte d'administration, qui sert aussi
//! de compte de service.

mod common;

use std::collections::HashSet;

use axum::http::StatusCode;
use common::{PASSWORD, TestApp, setup_with};
use dumbmonit_server::auth::ldap::{GroupMode, LdapConfig, LdapEnv};
use ldap3::{LdapConnAsync, Mod, Scope, SearchEntry};
use serde_json::{Value, json};

struct Directory {
    url: String,
    admin_dn: String,
    admin_password: String,
    base_dn: String,
}

impl Directory {
    fn from_env() -> Self {
        let var = |key: &str| std::env::var(key).unwrap_or_else(|_| panic!("{key} manquante"));
        Self {
            url: var("DUMBMONIT_TEST_LDAP_URL"),
            admin_dn: var("DUMBMONIT_TEST_LDAP_ADMIN_DN"),
            admin_password: var("DUMBMONIT_TEST_LDAP_ADMIN_PASSWORD"),
            base_dn: var("DUMBMONIT_TEST_LDAP_BASE_DN"),
        }
    }

    fn people(&self) -> String {
        format!("ou=dm-people,{}", self.base_dn)
    }

    fn groups(&self) -> String {
        format!("ou=dm-groups,{}", self.base_dn)
    }

    fn user_dn(&self, uid: &str) -> String {
        format!("uid={uid},{}", self.people())
    }

    fn group_dn(&self, cn: &str) -> String {
        format!("cn={cn},{}", self.groups())
    }

    async fn admin(&self) -> ldap3::Ldap {
        let (conn, mut ldap) = LdapConnAsync::new(&self.url).await.expect("connexion à l'annuaire");
        tokio::spawn(async move {
            let _ = conn.drive().await;
        });
        ldap.simple_bind(&self.admin_dn, &self.admin_password)
            .await
            .unwrap()
            .success()
            .expect("liaison d'administration");
        ldap
    }

    /// Recrée les unités de test et leur contenu.
    async fn seed(&self) {
        let mut ldap = self.admin().await;
        for ou in [self.people(), self.groups()] {
            if let Ok(result) =
                ldap.search(&ou, Scope::OneLevel, "(objectClass=*)", vec!["1.1"]).await
                && let Ok((entries, _)) = result.success()
            {
                for entry in entries {
                    let dn = SearchEntry::construct(entry).dn;
                    let _ = ldap.delete(&dn).await;
                }
            }
            let _ = ldap.delete(&ou).await;
            let name = ou.split(',').next().unwrap().trim_start_matches("ou=").to_string();
            ldap.add(
                &ou,
                vec![
                    ("objectClass", HashSet::from(["organizationalUnit"])),
                    ("ou", HashSet::from([name.as_str()])),
                ],
            )
            .await
            .unwrap()
            .success()
            .expect("unité de test");
        }
        for (uid, cn, password) in [
            ("alice", "Alice Admin", "alice-secret"),
            ("bob", "Bob Viewer", "bob-secret"),
            ("carol", "Carol Nogroup", "carol-secret"),
        ] {
            let mail = format!("{uid}@example.org");
            ldap.add(
                &self.user_dn(uid),
                vec![
                    ("objectClass", HashSet::from(["inetOrgPerson"])),
                    ("uid", HashSet::from([uid])),
                    ("cn", HashSet::from([cn])),
                    ("sn", HashSet::from([uid])),
                    ("mail", HashSet::from([mail.as_str()])),
                    ("userPassword", HashSet::from([password])),
                ],
            )
            .await
            .unwrap()
            .success()
            .expect("utilisateur de test");
        }
        for (cn, member) in [("dm-admins", "alice"), ("dm-staff", "bob")] {
            let member = self.user_dn(member);
            ldap.add(
                &self.group_dn(cn),
                vec![
                    ("objectClass", HashSet::from(["groupOfNames"])),
                    ("cn", HashSet::from([cn])),
                    ("member", HashSet::from([member.as_str()])),
                ],
            )
            .await
            .unwrap()
            .success()
            .expect("groupe de test");
        }
        let _ = ldap.unbind().await;
    }

    /// Remplace les membres d'un groupe (`groupOfNames` en exige au moins un).
    async fn set_members(&self, cn: &str, members: &[String]) {
        let mut ldap = self.admin().await;
        let members: HashSet<&str> = members.iter().map(String::as_str).collect();
        ldap.modify(&self.group_dn(cn), vec![Mod::Replace("member", members)])
            .await
            .unwrap()
            .success()
            .expect("modification du groupe");
        let _ = ldap.unbind().await;
    }

    fn config(&self) -> LdapConfig {
        LdapConfig {
            url: self.url.clone(),
            bind_dn: self.admin_dn.clone(),
            bind_password: self.admin_password.clone(),
            base_dn: self.people(),
            user_filter: "(&(objectClass=inetOrgPerson)(uid={username}))".into(),
            username_attribute: "uid".into(),
            display_name_attribute: "cn".into(),
            email_attribute: "mail".into(),
            group_mode: GroupMode::Search,
            group_base_dn: self.groups(),
            group_filter: "(&(objectClass=groupOfNames)(member={dn}))".into(),
            admin_groups: vec!["dm-admins".into()],
            viewer_groups: vec![self.group_dn("dm-staff")],
            ..LdapConfig::default()
        }
        .normalized()
    }
}

async fn user(app: &TestApp, admin: &str, username: &str) -> Option<Value> {
    let list = app.get("/api/users", Some(admin)).await;
    list.body.as_array().unwrap().iter().find(|u| u["username"] == username).cloned()
}

#[tokio::test]
#[ignore = "needs an OpenLDAP server, see the module documentation"]
async fn directory_users_sign_in_with_their_directory_password() {
    let directory = Directory::from_env();
    directory.seed().await;
    let config = directory.config();
    let app = setup_with(|c| {
        c.ldap = LdapEnv { config: config.clone() };
        c.ldap_allow_plain = !config.is_encrypted();
    })
    .await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;

    // Le diagnostic voit l'utilisateur, ses groupes et le rôle qu'il aurait.
    let report =
        app.post("/api/auth/ldap/test", json!({ "username": "alice" }), Some(&admin)).await;
    assert_eq!(report.body["ok"], true, "{}", report.body);
    assert_eq!(report.body["user"]["role"], "admin");
    assert_eq!(report.body["user"]["dn"], directory.user_dn("alice"));
    let report =
        app.post("/api/auth/ldap/test", json!({ "username": "carol" }), Some(&admin)).await;
    assert_eq!(report.body["user"]["allowed"], false, "{}", report.body);

    // Provisionnement juste-à-temps, rôle par les groupes.
    let alice = app.login_as("alice", "alice-secret").await;
    assert_eq!(alice.status, StatusCode::NO_CONTENT, "{}", alice.body);
    let account = user(&app, &admin, "alice").await.expect("compte créé");
    assert_eq!(account["role"], "admin");
    assert_eq!(account["auth"], "ldap");
    assert_eq!(account["display_name"], "Alice Admin");
    let me = app.get("/api/auth/me", Some(&alice.cookie())).await;
    assert_eq!(me.body["username"], "alice");

    let bob = app.login_as("bob", "bob-secret").await;
    assert_eq!(bob.status, StatusCode::NO_CONTENT, "{}", bob.body);
    assert_eq!(user(&app, &admin, "bob").await.unwrap()["role"], "viewer");

    // Mauvais mot de passe, mot de passe vide (liaison anonyme), inconnu,
    // utilisateur sans groupe autorisé : même réponse, aucun compte créé.
    for (username, password) in [
        ("bob", "pas-le-bon"),
        ("bob", ""),
        ("nobody", "x"),
        ("carol", "carol-secret"),
        // Tentatives d'injection dans le filtre.
        ("*", "alice-secret"),
        ("al*", "alice-secret"),
        ("alice)(uid=*", "alice-secret"),
    ] {
        let reply = app.login_as(username, password).await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{username}: {}", reply.body);
        assert_eq!(reply.body["error"], "Wrong username or password.");
    }
    assert!(user(&app, &admin, "carol").await.is_none());

    // Bob perd son groupe : refusé, compte désactivé. Il le retrouve : réactivé.
    directory.set_members("dm-staff", &[directory.user_dn("carol")]).await;
    let reply = app.login_as("bob", "bob-secret").await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(user(&app, &admin, "bob").await.unwrap()["disabled"], true);
    directory.set_members("dm-staff", &[directory.user_dn("bob")]).await;
    let reply = app.login_as("bob", "bob-secret").await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
    assert_eq!(user(&app, &admin, "bob").await.unwrap()["disabled"], false);

    // Un compte désactivé par un administrateur le reste, groupes ou pas.
    let id = user(&app, &admin, "bob").await.unwrap()["id"].as_i64().unwrap();
    let reply =
        app.put(&format!("/api/users/{id}"), json!({ "disabled": true }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(app.login_as("bob", "bob-secret").await.status, StatusCode::UNAUTHORIZED);

    // L'administrateur local entre toujours.
    assert_eq!(app.login_as("admin", PASSWORD).await.status, StatusCode::NO_CONTENT);

    // TLS, quand l'autorité de l'annuaire de test est fournie : StartTLS sur
    // l'URL `ldap://`, et LDAPS si son URL est donnée. Le nom d'hôte des URL
    // doit être celui du certificat.
    let Ok(ca_file) = std::env::var("DUMBMONIT_TEST_LDAP_CA_FILE") else { return };
    let ca = std::fs::read_to_string(&ca_file).expect("certificat de l'autorité");
    let admin = app.admin_cookie().await;
    let test = |url: &str, starttls: bool, ca: &str, verify: bool| {
        let config = directory.config();
        json!({
            "username": "alice",
            "config": {
                "url": url, "starttls": starttls, "tls_verify": verify, "ca_cert": ca,
                "bind_dn": config.bind_dn, "bind_password": config.bind_password,
                "base_dn": config.base_dn, "user_filter": config.user_filter,
                "username_attribute": "uid", "display_name_attribute": "cn",
                "group_mode": "search", "group_base_dn": config.group_base_dn,
                "group_filter": config.group_filter, "admin_groups": ["dm-admins"],
            }
        })
    };
    let mut urls = vec![(directory.url.clone(), true)];
    if let Ok(ldaps) = std::env::var("DUMBMONIT_TEST_LDAP_LDAPS_URL") {
        urls.push((ldaps, false));
    }
    for (url, starttls) in urls {
        let report =
            app.post("/api/auth/ldap/test", test(&url, starttls, &ca, true), Some(&admin)).await;
        assert_eq!(report.body["ok"], true, "{url} avec l'autorité : {}", report.body);
        assert_eq!(report.body["user"]["role"], "admin");

        // Sans l'autorité, le certificat auto-signé est refusé…
        let report =
            app.post("/api/auth/ldap/test", test(&url, starttls, "", true), Some(&admin)).await;
        assert_eq!(report.body["steps"][0]["ok"], false, "{url} sans autorité : {}", report.body);
        let detail = report.body["steps"][0]["detail"].as_str().unwrap().to_ascii_lowercase();
        assert!(detail.contains("certificate") || detail.contains("tls"), "{detail}");

        // … sauf vérification coupée, explicitement.
        let report =
            app.post("/api/auth/ldap/test", test(&url, starttls, "", false), Some(&admin)).await;
        assert_eq!(report.body["ok"], true, "{url} sans vérification : {}", report.body);
    }
}
