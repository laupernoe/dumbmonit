//! Astreintes : contacts et heures de chaque compte, plannings de rotation,
//! remplacements, politiques d'escalade en plusieurs étapes et leur journal.
//!
//! Le branchement sur l'alerting passe par un type de canal, `oncall` : un
//! canal de ce type désigne une politique d'escalade, et reçoit les alertes
//! par le routage existant (filtre par étiquettes, sévérité minimale, canaux
//! choisis par une règle). Au lieu d'envoyer un message, il ouvre une escalade
//! que [`runner::tick`] fait avancer à chaque cycle du moteur.
//!
//! * [`schedule`] — qui est d'astreinte à un instant donné (pur) ;
//! * [`escalation`] — quand appeler l'étape suivante, quand s'arrêter (pur) ;
//! * [`store`] — la base ;
//! * [`runner`] — les appels, les relèves annoncées, le message de test ;
//! * [`ical`] — l'export du planning d'une personne.

pub mod escalation;
pub mod ical;
pub mod runner;
pub mod schedule;
pub mod store;

use async_trait::async_trait;

use crate::notify::{ChannelConfig, Message, Notifier, NotifyError};

/// Type du canal qui déclenche une politique d'escalade.
pub const CHANNEL_KIND: &str = "oncall";

/// Réglage du canal : identifiant de la politique à déclencher.
pub const POLICY_SETTING: &str = "policy_id";

/// Le « notificateur » d'un canal d'astreinte. Il n'envoie rien lui-même :
/// le moteur d'alerting le reconnaît et ouvre une escalade à la place
/// (`alerting::engine`). Sa construction sert de validation du formulaire.
pub struct OncallChannel;

impl OncallChannel {
    pub fn new(config: &ChannelConfig) -> Result<Self, NotifyError> {
        store::policy_id_of(&config.settings).map(|_| Self).ok_or_else(|| {
            NotifyError::Config(format!(
                "missing setting \"{POLICY_SETTING}\": choose an escalation policy"
            ))
        })
    }
}

#[async_trait]
impl Notifier for OncallChannel {
    fn kind(&self) -> &'static str {
        CHANNEL_KIND
    }

    async fn send(&self, _message: &Message) -> Result<(), NotifyError> {
        Err(NotifyError::Config(
            "an on-call channel starts its escalation policy from the alerting engine; it does \
             not send messages by itself"
                .into(),
        ))
    }
}
