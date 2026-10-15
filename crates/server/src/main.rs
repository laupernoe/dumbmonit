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

    init_tracing();

    let config = Config::from_env().context("invalid configuration")?;
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
    runtime.block_on(run(config))
}

async fn run(mut config: Config) -> Result<()> {
    tokio::fs::create_dir_all(&config.data_dir)
        .await
        .with_context(|| format!("creating directory {}", config.data_dir.display()))?;
    ensure_data_dir_writable(&config.data_dir).await?;

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

    // VictoriaMetrics s'arrête après nous : les derniers lots du tampon d'écriture
    // ont ainsi une chance d'être acceptés.
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

/// Détermine le secret d'instance : variable d'environnement si fournie, sinon
/// fichier persistant, sinon génération au premier démarrage.
///
/// Le fichier permet à `docker compose up` de fonctionner sans configuration, tout
/// en gardant les identifiants déchiffrables après un redémarrage.
/// Vérifie tout de suite que le répertoire de données est accessible, avec un
/// message qui dit quoi faire : depuis 0.1.0-alpha.2 le conteneur ne tourne plus
/// en root, et un volume créé par une version antérieure (ou un bind mount)
/// appartient encore à root. Sans ce contrôle, l'erreur arrive plus loin, sur
/// `secret.key` ou sur la base, sous une forme qui n'explique rien.
async fn ensure_data_dir_writable(data_dir: &std::path::Path) -> Result<()> {
    let probe = data_dir.join(".write-test");
    let outcome = match tokio::fs::write(&probe, b"").await {
        Ok(()) => tokio::fs::remove_file(&probe).await,
        Err(error) => Err(error),
    };
    match outcome {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            #[cfg(unix)]
            let uid = unsafe { libc::geteuid() };
            #[cfg(not(unix))]
            let uid = 0;
            anyhow::bail!(
                "{} is not writable by the server (running as uid {uid}). The container \
                 runs as user 65532 since 0.1.0-alpha.2: a volume created by an older \
                 version or a bind mount must be handed over once with\n  docker run --rm \
                 -v dumbmonit-data:/data alpine chown -R 65532:65532 /data\nor start the \
                 container as the owner of the files (`user: \"1000:1000\"` in \
                 docker-compose.yml).",
                data_dir.display()
            );
        }
        Err(error) => {
            Err(anyhow::Error::from(error).context(format!("writing to {}", data_dir.display())))
        }
    }
}

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
