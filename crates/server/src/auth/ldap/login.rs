//! De l'entrée d'annuaire au compte DumbMonit (provisionnement juste-à-temps).
//!
//! À chaque connexion acceptée par l'annuaire, le compte est retrouvé par son
//! identité stable, ou créé ; son nom affiché et son rôle suivent l'annuaire.
//! Un utilisateur qui n'appartient plus à aucun groupe autorisé est désactivé
//! (ses sessions fermées), et réactivé s'il retrouve un groupe — sauf si c'est
//! un administrateur qui l'a désactivé.

use anyhow::{Context, Result, anyhow};
use sqlx::SqlitePool;
use tracing::{info, warn};

use super::LdapConfig;
use super::client::DirectoryUser;
use super::roles::{self, Decision};
use crate::auth::users::{self, LdapSync, NewUser, Role, User};

/// Pourquoi l'annuaire a dit oui mais DumbMonit dit non. Journalisé, jamais
/// montré à l'écran de connexion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denied {
    /// Membre d'aucun groupe autorisé.
    NoAllowedGroup,
    /// Désactivé par un administrateur.
    Disabled,
    /// L'identifiant est déjà pris par un compte qui n'est pas celui-ci
    /// (compte local, SSO, ou autre entrée d'annuaire).
    UsernameTaken,
}

impl Denied {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoAllowedGroup => "no_allowed_group",
            Self::Disabled => "disabled",
            Self::UsernameTaken => "username_taken",
        }
    }
}

/// Retrouve, crée ou met à jour le compte de cet utilisateur d'annuaire.
pub async fn provision(
    pool: &SqlitePool,
    config: &LdapConfig,
    entry: &DirectoryUser,
) -> Result<Result<User, Denied>> {
    let existing = match users::by_ldap(pool, &entry.id).await? {
        Some(user) => Some(user),
        None => match users::by_username(pool, &entry.username).await? {
            Some(user) if linkable(&user) => Some(user),
            Some(user) => {
                warn!(
                    user = %user.username,
                    "LDAP login matches an account that is not linked to this directory entry; \
                     refused (delete or rename that account to let the directory user in)"
                );
                return Ok(Err(Denied::UsernameTaken));
            }
            None => None,
        },
    };

    let current = existing.as_ref().map(|user| user.role);
    let decision = roles::decide(
        &entry.groups,
        &config.admin_groups,
        &config.operator_groups,
        &config.viewer_groups,
        current,
    );

    let Some(user) = existing else {
        let role = match decision {
            Decision::Deny => return Ok(Err(Denied::NoAllowedGroup)),
            Decision::Role(role) => role,
            Decision::Keep => Role::Viewer,
        };
        return create(pool, entry, role).await.map(Ok);
    };

    if decision == Decision::Deny {
        lose_access(pool, &user).await?;
        return Ok(Err(Denied::NoAllowedGroup));
    }
    if user.disabled && !user.disabled_by_ldap {
        return Ok(Err(Denied::Disabled));
    }

    let mut role = match decision {
        Decision::Role(role) if role != user.role => Some(role),
        _ => None,
    };
    // Une erreur de groupe dans l'annuaire ne doit pas pouvoir rétrograder le
    // dernier administrateur.
    if role.is_some() && user.role.is_admin() && users::active_admin_count(pool).await? <= 1 {
        warn!(user = %user.username, "LDAP groups would demote the last admin; role kept");
        role = None;
    }
    users::sync_ldap(
        pool,
        user.id,
        &LdapSync {
            ldap_id: &entry.id,
            ldap_dn: &entry.dn,
            display_name: &entry.display_name,
            role,
        },
    )
    .await?;
    if user.ldap_id.as_deref() != Some(entry.id.as_str()) {
        info!(user = %user.username, "account linked to its LDAP directory entry");
    }
    if user.disabled {
        info!(user = %user.username, "LDAP account re-enabled: it is back in an allowed group");
    }
    crate::auth::teams::sync_idp_groups(pool, user.id, &entry.groups).await?;
    crate::auth::clients::sync_idp_groups(pool, user.id, &entry.groups).await?;
    users::get(pool, user.id).await?.map(Ok).ok_or_else(|| anyhow!("account {} vanished", user.id))
}

/// Un compte existant que cette entrée d'annuaire peut prendre : ni mot de
/// passe local, ni identité SSO, ni autre entrée d'annuaire. C'est le cas d'un
/// compte créé d'avance par un administrateur, ou restauré d'une sauvegarde.
fn linkable(user: &User) -> bool {
    user.password_hash.is_none() && user.oidc_subject.is_none() && user.ldap_id.is_none()
}

/// L'utilisateur n'est plus dans aucun groupe autorisé : son compte est
/// désactivé et ses sessions fermées — sauf s'il est le dernier administrateur.
async fn lose_access(pool: &SqlitePool, user: &User) -> Result<()> {
    if user.disabled {
        return Ok(());
    }
    if user.role.is_admin() && users::active_admin_count(pool).await? <= 1 {
        warn!(
            user = %user.username,
            "LDAP user left every allowed group but is the last admin: access refused, \
             account kept enabled"
        );
        return Ok(());
    }
    users::disable_by_ldap(pool, user.id).await?;
    warn!(user = %user.username, "LDAP user left every allowed group: account disabled");
    Ok(())
}

async fn create(pool: &SqlitePool, entry: &DirectoryUser, role: Role) -> Result<User> {
    let id = users::insert(
        pool,
        NewUser {
            username: &entry.username,
            display_name: &entry.display_name,
            role,
            password_hash: None,
            oidc: None,
        },
    )
    .await?
    // Une autre connexion vient de créer le même identifiant : il sera
    // rattaché à la suivante. L'utilisateur réessaie.
    .context("the username was taken while the account was being created")?;
    users::sync_ldap(
        pool,
        id,
        &LdapSync {
            ldap_id: &entry.id,
            ldap_dn: &entry.dn,
            display_name: &entry.display_name,
            role: None,
        },
    )
    .await?;
    info!(user = %entry.username, role = role.as_str(), "account created from LDAP login");
    crate::auth::teams::sync_idp_groups(pool, id, &entry.groups).await?;
    crate::auth::clients::sync_idp_groups(pool, id, &entry.groups).await?;
    users::get(pool, id).await?.ok_or_else(|| anyhow!("account {id} vanished after creation"))
}
