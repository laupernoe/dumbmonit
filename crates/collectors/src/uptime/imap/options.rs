//! Réglages de la sonde IMAP.

use dumbmonit_proto::{ProbeError, Target};

use crate::uptime::mailbox::{Common, Ports};

/// Ports IMAP : 993 (TLS implicite), 143 (`STARTTLS`, ou clair).
pub const PORTS: Ports = Ports { implicit: 993, starttls: 143, none: 143 };

pub type Options = Common;

pub fn from_target(target: &Target) -> Result<Options, ProbeError> {
    Common::from_target(target, PORTS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::mailbox::Security;
    use crate::uptime::tags::test_support::cible;

    #[test]
    fn les_defauts_sont_chiffres_sur_le_port_993() {
        let options = from_target(&cible("imap", "imap.exemple.fr", &[])).unwrap();
        assert_eq!((options.security, options.port), (Security::Implicit, 993));
        assert_eq!(options.server_name, "imap.exemple.fr");
        assert!(options.login.is_none());
    }

    #[test]
    fn starttls_utilise_le_port_143() {
        let target = cible("imap", "imap.exemple.fr", &[("security", "starttls")]);
        assert_eq!(from_target(&target).unwrap().port, 143);
    }
}
