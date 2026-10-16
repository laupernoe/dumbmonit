use anyhow::{Context, Result};
use dumbmonit_server::config::Config;
use dumbmonit_server::state::{AppState, Inner};
use dumbmonit_server::{
    alerting, api, auth, collectors, crypto, db, demo, notify, scheduler, tsdb,
};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // `dumbmonit pack lint|test …` : outils d'auteur de paquet, sans serveur, sans
    // base, sans configuration — ils ne font que lire des fichiers.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("pack") {
        std::process::exit(dumbmonit_pack::cli::run(&args[1..]));
    }

    // `dumbmonit healthcheck` : sonde du HEALTHCHECK Docker (image sans shell ni
    // curl). Ni base, ni VictoriaMetrics, ni logs : seulement un code de sortie.
    if args.first().map(String::as_str) == Some("healthcheck") {
        // Conteneur lancé en `user: "0:0"` : la sonde tourne en root elle aussi.
        // Elle abandonne root avant tout, sans rien toucher sur disque.
        #[cfg(unix)]
        if let Err(error) = dumbmonit_server::privileges::drop_if_root() {
            eprintln!("healthcheck: cannot drop root: {error}");
            std::process::exit(1);
        }
        std::process::exit(dumbmonit_server::healthcheck::run());
    }

    init_tracing();

    let config = Config::from_env().context("invalid configuration")?;

    // Lancé en root (`user: "0:0"`, comme dans docker-compose.yml) : `/data` est
    // remis au nom de l'utilisateur du serveur, puis les privilèges sont
    // abandonnés pour de bon — avant la base, VictoriaMetrics et le réseau. Les
    // sous-commandes ci-dessus ne font jamais de chown (`healthcheck` abandonne
    // root sans rien toucher).
    #[cfg(unix)]
    let startup = dumbmonit_server::privileges::prepare(&config.data_dir)?;
    #[cfg(not(unix))]
    let startup = ();

    info!(version = env!("CARGO_PKG_VERSION"), workers = config.workers, "starting DumbMonit");

    // Le runtime est construit à la main plutôt que par `#[tokio::main]` : c'est
    // le seul moyen de fixer le nombre de threads d'après la configuration.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.workers)
        // Le pool bloquant ne sert qu'aux lectures de fichiers (`tokio::fs`) et à
        // quelques résolutions DNS : 512 threads par défaut, seize suffisent.
        .max_blocking_threads(16)
        .thread_name("dumbmonit-worker")
        .enable_all()
        .build()
        .context("building the async runtime")?;
    runtime.block_on(run(config, startup))
}

#[cfg(unix)]
type Startup = dumbmonit_server::privileges::Startup;
#[cfg(not(unix))]
type Startup = ();

/// Attente maximale du vidage final du tampon d'écriture : assez pour un lot
/// complet, bien en deçà du délai de grâce laissé au conteneur pour s'arrêter.
const FINAL_FLUSH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

async fn run(mut config: Config, startup: Startup) -> Result<()> {
    tokio::fs::create_dir_all(&config.data_dir)
        .await
        .with_context(|| format!("creating directory {}", config.data_dir.display()))?;
    ensure_data_dir_writable(&config.data_dir, startup).await?;

    let secret = resolve_secret(&config).await?;
    if config.demo {
        // Démonstration publique : la base repart de zéro à chaque démarrage, et
        // rien ne doit jamais partir vers l'extérieur (voir `demo`).
        warn!(
            "DUMBMONIT_DEMO is set: read-only public demo, database recreated with fictional data"
        );
        demo::reset_database(&config.database_path()).await?;
        notify::disable_sending();
    } else {
        db::adopt_legacy_database(&config.data_dir).await?;
    }
    let pool = db::open_with(&config.database_path(), config.db_pool_size).await?;
    let cipher = db::init_cipher(&pool, &secret).await?;
    if config.reset_password {
        auth::reset_password(&pool).await?;
        warn!(
            "DUMBMONIT_RESET_PASSWORD is set: all accounts and sessions removed — \
             remove the variable once the first admin has been created again"
        );
    }
    // Les en-têtes de webhook rangés en clair par une version antérieure passent
    // dans les secrets chiffrés. Un échec n'empêche pas de démarrer : l'API ne
    // les renvoie de toute façon plus.
    if let Err(error) = db::alerts::lift_legacy_webhook_headers(&pool, &cipher).await {
        warn!(?error, "webhook headers could not be moved to secrets");
    }
    info!(database = %config.database_path().display(), "database ready");
    // Paire VAPID des notifications Web Push : générée au premier démarrage,
    // chiffrée en base. Un échec ici n'empêche pas de surveiller : il sera
    // retenté à la première demande de l'interface.
    if let Err(error) =
        dumbmonit_server::notify::webpush::vapid::load_or_create(&pool, &cipher).await
    {
        warn!(?error, "Web Push keys unavailable");
    }

    // Pas encore d'administrateur : la création du premier compte exige le code
    // affiché ici, que seul qui lit le journal du serveur connaît.
    let setup_code = config.setup_code.get_or_insert_with(auth::setup_code::generate).clone();
    if !config.demo && auth::users::count(&pool).await? == 0 {
        auth::setup_code::announce(&setup_code);
    }

    let victoria_url = config.effective_victoria_url();
    // Clé des routes d'administration du VictoriaMetrics embarqué, propre à ce
    // démarrage : elle n'est écrite nulle part et n'existe que dans ce processus.
    let vm_admin_key = dumbmonit_server::crypto::generate_secret();
    let mut victoria = tsdb::Victoria::new(&victoria_url)?;
    if config.vm_embedded() {
        victoria = victoria.with_admin_key(vm_admin_key.clone());
    }
    let embedded_vm = if config.vm_embedded() {
        // Sans URL externe, l'image se suffit : VictoriaMetrics est lancé ici même
        // et le démarrage attend qu'il réponde — une erreur à ce stade (binaire
        // absent, port pris) doit arrêter le serveur, pas le laisser tourner à vide.
        let vm = tsdb::EmbeddedVm::start(
            tsdb::EmbeddedConfig {
                binary: config.vm_binary.clone(),
                data_path: config.vm_data_path(),
                listen: config.vm_listen.clone(),
                retention: config.vm_retention.clone(),
                memory: config.vm_memory.clone(),
                admin_key: vm_admin_key,
            },
            &victoria,
        )
        .await
        .context("starting the embedded VictoriaMetrics")?;
        Some(vm)
    } else {
        match victoria.health().await {
            Ok(()) => info!(url = %victoria_url, "VictoriaMetrics reachable"),
            // On ne bloque pas le démarrage : l'ordre de lancement des conteneurs
            // n'est pas garanti, et le tampon d'écriture retentera de lui-même.
            Err(error) => {
                warn!(url = %victoria_url, %error, "VictoriaMetrics unreachable at startup")
            }
        }
        None
    };

    let sink = tsdb::spawn_writer_with(
        victoria.clone(),
        config.write_flush_interval,
        config.write_flush_size,
    );

    // Équipements simulés de la démonstration : lancés avant le premier client
    // HTTP partagé, qui doit connaître leurs adresses dès sa construction.
    let estate = if config.demo {
        let estate = demo::estate::start().await.context("starting the demo devices")?;
        dumbmonit_collectors::http::set_resolve_overrides(estate.resolve.clone());
        Some(estate)
    } else {
        None
    };

    // Paquets d'intégration activés : relus ici pour les profils SNMP qu'ils
    // apportent, que le collecteur SNMP doit connaître dès sa construction. Leurs
    // collecteurs HTTP sont enregistrés plus bas, dans l'étage d'exécution.
    let packs = dumbmonit_server::packs::load_enabled(&pool).await?;

    // Une vérification de site interrompue par un arrêt du serveur a laissé son
    // verrou : il est levé avant que le collecteur ne reprenne.
    dumbmonit_server::webchange::store::release_all(&pool).await?;

    // Liste unique, partagée avec l'agent relais (`collectors::builtin`).
    let mut registry = collectors::builtin(&pool, &config, &packs);
    if config.demo {
        demo::register_synthetic(&mut registry);
    }
    // Types définis par les paquets : dans l'étage du registre que l'API
    // d'installation modifie ensuite sans redémarrage.
    for pack in &packs {
        if let Err(error) = dumbmonit_server::packs::activate(&registry, pack) {
            warn!(pack = pack.id(), %error, "integration pack not registered");
        }
    }
    info!(collectors = ?registry.kinds(), "collectors registered");

    let seeded = match &estate {
        Some(estate) => Some(
            demo::seed::run(&pool, &cipher, &sink, &estate.devices)
                .await
                .context("seeding the demo estate")?,
        ),
        None => None,
    };

    // Les canaux de notification ne doivent jamais viser ce serveur ni sa base
    // de métriques, dont certaines routes ne demandent aucune authentification.
    dumbmonit_server::notify::guard::protect_config(&config).await;

    let bind = config.bind;
    // Gardé pour le vidage final : les clones de l'état partagé restent tenus par
    // les tâches de fond jusqu'à la fin du runtime.
    let final_sink = sink.clone();
    let state = AppState::new(Inner { config, pool, cipher, victoria, sink, collectors: registry });

    scheduler::spawn(state.clone());
    dumbmonit_server::reports::spawn(state.clone());
    alerting::spawn(state.clone());
    dumbmonit_server::security::spawn(state.clone());
    // Mesures de l'instance elle-même et battement de cœur sortant (`selfmon`).
    dumbmonit_server::selfmon::spawn(state.clone());
    collectors::agent::spawn_policy_scheduler(state.clone());
    match seeded {
        Some(seeded) => demo::spawn(state.clone(), seeded),
        // Sauvegardes locales planifiées de la base, et la mesure qui dit qu'elles
        // ont bien lieu (`backup::local`). Inutiles pour une base jetable.
        None => dumbmonit_server::backup::local::spawn(state.clone()),
    }

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("cannot listen on {bind}"))?;
    info!(%bind, "interface available");

    // `ConnectInfo` : l'adresse du client, pour les compteurs de tentatives et
    // le journal d'audit (voir `auth::client_ip`).
    axum::serve(
        listener,
        api::router(state).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("HTTP server error")?;

    // Vidage explicite du tampon d'écriture, avant l'arrêt de VictoriaMetrics :
    // la tâche d'écriture serait sinon annulée avec le runtime, et les derniers
    // échantillons perdus à chaque arrêt du conteneur.
    if !final_sink.flush(FINAL_FLUSH_TIMEOUT).await {
        warn!(timeout = ?FINAL_FLUSH_TIMEOUT, "final write flush failed or timed out, samples lost");
    }
    if let Some(vm) = embedded_vm {
        vm.stop().await;
    }

    info!("clean shutdown");
    Ok(())
}

fn init_tracing() {
    // Lecture brute des deux noms : `env_var` avertirait avant que l'abonné
    // n'existe, et l'avertissement serait perdu. Il est rejoué juste après.
    let raw = std::env::var("DUMBMONIT_LOG").or_else(|_| std::env::var("EZYMONIT_LOG")).ok();
    let filter = raw
        .as_deref()
        .and_then(|directives| EnvFilter::try_new(directives).ok())
        .unwrap_or_else(|| EnvFilter::new("info,sqlx=warn,hyper=warn"));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).init();
    let _ = dumbmonit_server::config::env_var("DUMBMONIT_LOG");
}

/// Vérifie tout de suite que le répertoire de données est accessible, avec un
/// message qui dit quoi faire selon la cause probable (volume nommé, bind mount,
/// SELinux…). Sans ce contrôle, l'erreur arrive plus loin, sur `secret.key` ou
/// sur la base, sous une forme qui n'explique rien.
async fn ensure_data_dir_writable(data_dir: &std::path::Path, startup: Startup) -> Result<()> {
    let probe = data_dir.join(".write-test");
    let outcome = match tokio::fs::write(&probe, b"").await {
        Ok(()) => tokio::fs::remove_file(&probe).await,
        Err(error) => Err(error),
    };
    match outcome {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            #[cfg(unix)]
            {
                use dumbmonit_server::privileges;
                let diagnosis = privileges::diagnose(data_dir, startup);
                anyhow::bail!("{}", privileges::explain_not_writable(data_dir, &diagnosis));
            }
            #[cfg(not(unix))]
            {
                let () = startup;
                anyhow::bail!("{} is not writable by the server", data_dir.display());
            }
        }
        Err(error) => {
            Err(anyhow::Error::from(error).context(format!("writing to {}", data_dir.display())))
        }
    }
}

/// Détermine le secret d'instance : variable d'environnement si fournie, sinon
/// fichier persistant, sinon génération au premier démarrage.
///
/// Le fichier permet à `docker compose up` de fonctionner sans configuration, tout
/// en gardant les identifiants déchiffrables après un redémarrage.
async fn resolve_secret(config: &Config) -> Result<String> {
    if let Some(secret) = &config.secret {
        info!("instance secret provided by the environment");
        return Ok(secret.clone());
    }

    let path = config.secret_path();
    match tokio::fs::read_to_string(&path).await {
        Ok(secret) => {
            let secret = secret.trim().to_string();
            if secret.is_empty() {
                anyhow::bail!(
                    "the file {} is empty: restore it, or delete it to generate a new one",
                    path.display()
                );
            }
            Ok(secret)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let secret = crypto::generate_secret();
            write_secret_file(&path, &secret).await?;
            warn!(
                path = %path.display(),
                "instance secret generated — back up this file with the database, \
                 without it the device credentials cannot be recovered"
            );
            Ok(secret)
        }
        Err(error) => {
            Err(anyhow::Error::from(error).context(format!("reading {}", path.display())))
        }
    }
}

async fn write_secret_file(path: &std::path::Path, secret: &str) -> Result<()> {
    tokio::fs::write(path, secret).await.with_context(|| format!("writing {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .await
            .with_context(|| format!("restricting permissions on {}", path.display()))?;
    }
    Ok(())
}

/// Attend `SIGTERM` (arrêt de conteneur) ou `Ctrl+C`.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => warn!(%error, "cannot listen for SIGTERM"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("interrupt received"),
        _ = terminate => info!("SIGTERM received"),
    }
}
