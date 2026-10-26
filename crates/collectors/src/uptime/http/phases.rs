//! Décomposition du temps d'attente d'une requête HTTP en phases.
//!
//! `reqwest` n'expose pas d'instants intermédiaires, mais deux points d'accroche
//! permettent de les relever sur la requête **réellement mesurée** :
//!
//! * le résolveur DNS du client, enveloppé par [`TimedResolver`] : durée de la
//!   résolution ;
//! * le connecteur, enveloppé par [`SetupTimer`] (couche `tower`) : durée totale
//!   d'établissement de la connexion, résolution, TCP et TLS compris.
//!
//! Les deux écrivent dans une [`PhaseClock`] propre à la sonde en cours. C'est pour
//! cela que le client est construit à chaque sonde : un client partagé entre cibles
//! ne saurait pas à quelle requête rattacher un temps de connexion. Un client neuf
//! n'a pas de connexion à réutiliser, si bien que chaque contrôle mesure une vraie
//! ouverture de connexion, comme le ferait un premier accès.
//!
//! La couche de connexion ne sépare pas TCP de TLS : la part de TLS est donc prise
//! sur la poignée de main relevée par le contrôle de certificat juste avant la
//! requête, et plafonnée au temps de connexion observé. Sans ce relevé
//! (`check_certificate = false`), le TLS reste inclus dans `connect`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use reqwest::dns::{Name, Resolve, Resolving};
use tower_layer::Layer;
use tower_service::Service;

use crate::uptime::guard::GuardedResolver;

/// Compteurs partagés entre le résolveur, le connecteur et la sonde.
#[derive(Default)]
pub(super) struct PhaseClock {
    dns_ns: AtomicU64,
    setup_ns: AtomicU64,
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

impl PhaseClock {
    /// Remise à zéro avant chaque saut : seule la dernière requête est décomposée.
    pub(super) fn reset(&self) {
        self.dns_ns.store(0, Ordering::Relaxed);
        self.setup_ns.store(0, Ordering::Relaxed);
    }

    fn add_dns(&self, duration: Duration) {
        self.dns_ns.fetch_add(nanos(duration), Ordering::Relaxed);
    }

    fn add_setup(&self, duration: Duration) {
        self.setup_ns.fetch_add(nanos(duration), Ordering::Relaxed);
    }

    fn dns(&self) -> Duration {
        Duration::from_nanos(self.dns_ns.load(Ordering::Relaxed))
    }

    fn setup(&self) -> Duration {
        Duration::from_nanos(self.setup_ns.load(Ordering::Relaxed))
    }

    /// Décompose l'attente `first_byte` (envoi de la requête jusqu'aux en-têtes).
    pub(super) fn decompose(&self, first_byte: Duration, tls_hint: Option<Duration>) -> Phases {
        decompose(first_byte, self.dns(), self.setup(), tls_hint)
    }
}

/// Phases d'une requête. Leur somme vaut le temps écoulé de l'envoi à la fin de la
/// lecture du corps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct Phases {
    pub(super) dns: Duration,
    pub(super) connect: Duration,
    pub(super) tls: Duration,
    /// Attente du serveur : de la requête envoyée aux en-têtes reçus.
    pub(super) processing: Duration,
    /// Lecture du corps (nulle quand le corps n'est pas rapatrié).
    pub(super) transfer: Duration,
}

impl Phases {
    /// Libellés de l'étiquette `phase` et durées, dans l'ordre d'empilement.
    pub(super) fn series(&self) -> [(&'static str, f64); 5] {
        [
            ("dns", self.dns.as_secs_f64()),
            ("connect", self.connect.as_secs_f64()),
            ("tls", self.tls.as_secs_f64()),
            ("processing", self.processing.as_secs_f64()),
            ("transfer", self.transfer.as_secs_f64()),
        ]
    }
}

/// Calcul pur de la décomposition, borné pour que la somme reste exacte même si
/// les relevés se chevauchent de quelques microsecondes.
fn decompose(
    first_byte: Duration,
    dns_raw: Duration,
    setup_raw: Duration,
    tls_hint: Option<Duration>,
) -> Phases {
    let setup = setup_raw.min(first_byte);
    let dns = dns_raw.min(setup);
    let connection = setup - dns;
    let tls = tls_hint.unwrap_or_default().min(connection);
    Phases {
        dns,
        connect: connection - tls,
        tls,
        processing: first_byte - setup,
        transfer: Duration::ZERO,
    }
}

/// Résolveur du garde-fou, chronométré.
pub(super) struct TimedResolver {
    inner: GuardedResolver,
    clock: Arc<PhaseClock>,
}

impl TimedResolver {
    pub(super) fn new(allow_private: bool, clock: Arc<PhaseClock>) -> Self {
        Self { inner: GuardedResolver { allow_private }, clock }
    }
}

impl Resolve for TimedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let started = Instant::now();
        let resolution = self.inner.resolve(name);
        let clock = Arc::clone(&self.clock);
        Box::pin(async move {
            let result = resolution.await;
            clock.add_dns(started.elapsed());
            result
        })
    }
}

/// Couche qui chronomètre l'établissement complet d'une connexion.
#[derive(Clone)]
pub(super) struct SetupTimer {
    clock: Arc<PhaseClock>,
}

impl SetupTimer {
    pub(super) fn new(clock: Arc<PhaseClock>) -> Self {
        Self { clock }
    }
}

impl<S> Layer<S> for SetupTimer {
    type Service = TimedConnector<S>;

    fn layer(&self, inner: S) -> TimedConnector<S> {
        TimedConnector { inner, clock: Arc::clone(&self.clock) }
    }
}

#[derive(Clone)]
pub(super) struct TimedConnector<S> {
    inner: S,
    clock: Arc<PhaseClock>,
}

impl<S, R> Service<R> for TimedConnector<S>
where
    S: Service<R>,
    S::Future: Send + 'static,
    S::Response: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<S::Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: R) -> Self::Future {
        let started = Instant::now();
        let connecting = self.inner.call(request);
        let clock = Arc::clone(&self.clock);
        Box::pin(async move {
            let result = connecting.await;
            clock.add_setup(started.elapsed());
            result
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    fn somme(phases: &Phases) -> Duration {
        phases.dns + phases.connect + phases.tls + phases.processing + phases.transfer
    }

    #[test]
    fn la_decomposition_somme_au_premier_octet() {
        let phases = decompose(ms(100), ms(10), ms(50), Some(ms(20)));
        assert_eq!(phases.dns, ms(10));
        assert_eq!(phases.tls, ms(20));
        assert_eq!(phases.connect, ms(20));
        assert_eq!(phases.processing, ms(50));
        assert_eq!(somme(&phases), ms(100));
    }

    #[test]
    fn une_connexion_reutilisee_na_ni_dns_ni_connexion_ni_tls() {
        let phases = decompose(ms(40), Duration::ZERO, Duration::ZERO, Some(ms(15)));
        assert_eq!(phases.dns, Duration::ZERO);
        assert_eq!(phases.connect, Duration::ZERO);
        assert_eq!(phases.tls, Duration::ZERO);
        assert_eq!(phases.processing, ms(40));
    }

    #[test]
    fn la_part_de_tls_est_plafonnee_au_temps_de_connexion() {
        let phases = decompose(ms(100), ms(5), ms(25), Some(ms(500)));
        assert_eq!(phases.tls, ms(20));
        assert_eq!(phases.connect, Duration::ZERO);
        assert_eq!(somme(&phases), ms(100));
    }

    #[test]
    fn sans_releve_tls_la_poignee_de_main_reste_dans_la_connexion() {
        let phases = decompose(ms(100), ms(5), ms(45), None);
        assert_eq!(phases.tls, Duration::ZERO);
        assert_eq!(phases.connect, ms(40));
    }

    #[test]
    fn des_releves_qui_debordent_ne_font_pas_depasser_la_somme() {
        let phases = decompose(ms(30), ms(50), ms(80), Some(ms(10)));
        assert_eq!(somme(&phases), ms(30));
        assert_eq!(phases.processing, Duration::ZERO);
    }

    #[test]
    fn les_series_sont_dans_lordre_dempilement() {
        let noms: Vec<&str> = Phases::default().series().iter().map(|(nom, _)| *nom).collect();
        assert_eq!(noms, ["dns", "connect", "tls", "processing", "transfer"]);
    }
}
