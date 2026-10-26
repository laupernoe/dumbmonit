//! Réglages de la sonde POP3.

use dumbmonit_proto::{ProbeError, Target};

use crate::uptime::mailbox::{Common, Ports};

/// Ports POP3 : 995 (TLS implicite), 110 (`STLS`, ou clair).
pub const PORTS: Ports = Ports { implicit: 995, starttls: 110, none: 110 };

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
    fn les_defauts_sont_chiffres_sur_le_port_995() {
        let options = from_target(&cible("pop3", "pop.exemple.fr", &[])).unwrap();
        assert_eq!((options.security, options.port), (Security::Implicit, 995));
    }

    #[test]
    fn stls_utilise_le_port_110() {
        let target = cible("pop3", "pop.exemple.fr", &[("security", "starttls")]);
        assert_eq!(from_target(&target).unwrap().port, 110);
    }
}
