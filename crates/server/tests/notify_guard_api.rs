//! Les canaux de notification ne joignent jamais le serveur lui-même.
//!
//! Binaire de test à part : la liste des adresses internes est globale au
//! processus, et ne doit pas déborder sur les autres tests d'envoi.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use dumbmonit_server::notify::http::{Request, send};
use dumbmonit_server::notify::{NotifyError, guard, http_client};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Serveur HTTP minimal : compte les connexions et répond toujours la même chose.
async fn serveur(reponse: impl Fn(SocketAddr) -> String + Send + Sync + 'static) -> Ecoute {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let connexions = Arc::new(AtomicUsize::new(0));
    let compteur = connexions.clone();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            compteur.fetch_add(1, Ordering::SeqCst);
            let texte = reponse(address);
            tokio::spawn(async move {
                let mut tampon = [0u8; 4096];
                let _ = socket.read(&mut tampon).await;
                let _ = socket.write_all(texte.as_bytes()).await;
                let _ = socket.shutdown().await;
            });
        }
    });
    Ecoute { address, connexions }
}

struct Ecoute {
    address: SocketAddr,
    connexions: Arc<AtomicUsize>,
}

impl Ecoute {
    fn port(&self) -> u16 {
        self.address.port()
    }
    fn touche(&self) -> bool {
        self.connexions.load(Ordering::SeqCst) > 0
    }
}

fn ok(_: SocketAddr) -> String {
    "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
}

fn est_un_refus(resultat: Result<(), NotifyError>) -> bool {
    match resultat {
        Err(NotifyError::Config(message)) => message.contains("DumbMonit itself"),
        _ => false,
    }
}

#[tokio::test]
async fn un_canal_ne_joint_ni_la_base_de_metriques_ni_le_serveur() {
    // « interne » joue le VictoriaMetrics embarqué ; « homelab » un service
    // légitime de la même machine, qui doit rester joignable.
    let interne = serveur(ok).await;
    let homelab = serveur(ok).await;
    guard::protect([interne.address]);
    let http = http_client();

    // Adresse littérale, puis nom résolu vers la boucle locale.
    for url in [
        format!("http://127.0.0.1:{}/api/v1/admin/tsdb/delete_series", interne.port()),
        format!("http://localhost:{}/api/v1/import", interne.port()),
        format!("http://[::ffff:127.0.0.1]:{}/", interne.port()),
        format!("http://0.0.0.0:{}/", interne.port()),
    ] {
        let resultat = send(&http, Request::post(&url).json(serde_json::json!({}))).await;
        assert!(est_un_refus(resultat), "{url} must be refused as a configuration error");
    }
    assert!(!interne.touche(), "no connection may reach the internal address");

    // La boucle locale reste permise sur un autre port, y compris par un nom :
    // c'est le cas d'un Gotify ou d'un Home Assistant sur la même machine.
    for url in [
        format!("http://127.0.0.1:{}/message", homelab.port()),
        format!("http://localhost:{}/message", homelab.port()),
    ] {
        let resultat = send(&http, Request::post(&url).json(serde_json::json!({}))).await;
        assert!(resultat.is_ok(), "{url}: {resultat:?}");
    }
    assert!(homelab.touche());
}

#[tokio::test]
async fn une_redirection_vers_une_adresse_interne_est_refusee() {
    let interne = serveur(ok).await;
    guard::protect([interne.address]);
    let port_interne = interne.port();
    // Même hôte, passage à HTTPS sur un autre port : la seule redirection qui
    // change de port que la politique « même hôte » laisse passer.
    let rebond = serveur(move |address| {
        format!(
            "HTTP/1.1 307 Temporary Redirect\r\nLocation: https://{}:{port_interne}/\r\n\
             Content-Length: 0\r\nConnection: close\r\n\r\n",
            address.ip()
        )
    })
    .await;
    let rebond_par_nom = serveur(move |_| {
        format!(
            "HTTP/1.1 308 Permanent Redirect\r\nLocation: https://localhost:{port_interne}/\r\n\
             Content-Length: 0\r\nConnection: close\r\n\r\n"
        )
    })
    .await;
    let http = http_client();

    for url in [
        format!("http://127.0.0.1:{}/hook", rebond.port()),
        format!("http://localhost:{}/hook", rebond_par_nom.port()),
    ] {
        let resultat = send(&http, Request::post(&url).json(serde_json::json!({}))).await;
        assert!(est_un_refus(resultat), "{url}: the redirect must be refused");
    }
    assert!(rebond.touche() && rebond_par_nom.touche(), "the first hop is allowed");
    assert!(!interne.touche(), "the redirect never reaches the internal address");
}

#[tokio::test]
async fn un_relais_smtp_interne_est_refuse_avant_toute_connexion() {
    let interne = serveur(ok).await;
    guard::protect([interne.address]);
    let config = dumbmonit_server::notify::ChannelConfig {
        id: 1,
        name: "mail".into(),
        kind: "smtp".into(),
        enabled: true,
        settings: serde_json::json!({
            "host": "127.0.0.1",
            "port": interne.port(),
            "security": "none",
            "from": "dumbmonit@exemple.org",
            "to": ["admin@exemple.org"],
        }),
        secrets: serde_json::json!({}),
    };
    let report = dumbmonit_server::notify::deliver(
        &http_client(),
        &config,
        &dumbmonit_server::notify::test_message("trial"),
    )
    .await;
    assert!(report.error.is_some_and(|e| e.contains("DumbMonit itself")));
    assert!(!interne.touche());
}
