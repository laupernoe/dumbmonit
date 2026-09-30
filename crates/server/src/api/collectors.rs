//! Types d'équipements que l'on peut ajouter, et comment les préparer.
//!
//! L'interface découvre cette liste au lieu de la coder en dur : elle ne propose
//! donc jamais un type que ce serveur n'embarque pas, et récupère automatiquement
//! ceux qu'une version ultérieure ajoutera.
//!
//! Chaque type porte sa propre notice de mise en route. C'est ce qui permet à
//! l'interface d'afficher, à côté du formulaire, les étapes à faire sur
//! l'équipement lui-même — la moitié du travail d'ajout se passe là-bas, pas ici.
//!
//! Chaque type décrit aussi ses options : les réglages qu'un collecteur lit dans
//! les étiquettes de la cible (`Target::tags`). Sans cette liste, l'utilisateur
//! devrait deviner le nom exact d'une étiquette et sa syntaxe ; avec elle,
//! l'interface propose un champ par option, avec son défaut et son aide. La liste
//! est un miroir des `options.rs` de chaque collecteur : une étiquette absente
//! ici reste lue, mais personne ne saura qu'elle existe.

use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct CollectorView {
    /// Valeur à placer dans le champ `kind` d'une cible.
    pub kind: &'static str,
    /// Libellé destiné à l'affichage.
    pub label: &'static str,
    /// Une phrase disant à quoi sert ce type, pour la liste de choix.
    pub summary: &'static str,
    /// Exemples d'équipements concernés, pour que l'utilisateur se reconnaisse.
    pub examples: &'static [&'static str],
    /// Formes de `credential` acceptées (leurs `kind`), dans l'ordre de préférence.
    /// Redondant avec `credentials`, conservé pour les interfaces plus anciennes.
    pub credential_types: &'static [&'static str],
    /// Les mêmes formes, décrites champ par champ : c'est ce que le formulaire
    /// affiche. Un jeton Proxmox se saisit ainsi en deux cases (identifiant et
    /// secret) plutôt qu'en une chaîne `user@pve!nom=secret` que personne ne devine.
    pub credentials: &'static [CredentialView],
    /// Adresse d'exemple, utilisée comme texte indicatif du champ.
    pub address_hint: &'static str,
    /// Port par défaut, affiché pour lever le doute.
    pub default_port: u16,
    /// Notice de mise en route affichée à côté du formulaire.
    pub setup: Setup,
    /// Réglages lus dans les étiquettes de la cible, dans l'ordre d'affichage.
    /// Vide pour un type qui n'en lit aucune.
    pub options: &'static [OptionView],
}

#[derive(Serialize)]
pub struct Setup {
    /// Titre de la notice.
    pub title: &'static str,
    /// Étapes à effectuer sur l'équipement, dans l'ordre.
    ///
    /// Une étape est une phrase. Si elle est suivie d'un saut de ligne, ce qui
    /// suit est une commande ou une valeur à copier telle quelle : l'interface
    /// l'affiche dans un bloc avec un bouton « copier », et la documentation la
    /// reprend dans un bloc de code. Les mêmes étapes figurent dans
    /// `docs/devices/<kind>.md` ; un test vérifie qu'elles n'ont pas divergé.
    pub steps: &'static [&'static str],
    /// Point d'attention fréquent, à mettre en évidence. Vide s'il n'y en a pas.
    pub warning: &'static str,
    /// Lien vers la documentation du constructeur. Vide s'il n'y en a pas.
    pub doc_url: &'static str,
}

/// Un réglage porté par une étiquette de la cible.
///
/// C'est le contrat avec l'interface : elle construit un champ de formulaire par
/// entrée, et enregistre la valeur saisie sous la clé `key` dans `Target::tags`.
#[derive(Serialize)]
pub struct OptionView {
    /// Clé du tag dans `Target::tags`.
    pub key: &'static str,
    pub label: &'static str,
    pub help: &'static str,
    pub placeholder: &'static str,
    /// Valeur par défaut affichée (vide si aucune).
    pub default: &'static str,
    pub required: bool,
    /// `text`, `number`, `boolean`, `select`.
    pub input: &'static str,
    /// Valeurs proposées quand `input == "select"`.
    pub choices: &'static [&'static str],
}

/// Une forme d'identifiant acceptée par un type, et les champs à remplir.
///
/// C'est le contrat avec le formulaire : un champ par entrée, envoyé sous la clé
/// `key` dans l'objet `credential` (`{"type": kind, key: valeur, …}`). Le serveur
/// sait recomposer ce qu'il attend — un jeton Proxmox à partir de `token_id` et
/// `secret`, un SNMP v3 à partir de ses protocoles et phrases de passe.
#[derive(Serialize)]
pub struct CredentialView {
    /// Valeur du champ `type` de `credential`.
    pub kind: &'static str,
    pub label: &'static str,
    /// Une phrase pour situer cette forme par rapport aux autres. Vide si inutile.
    pub help: &'static str,
    pub fields: &'static [CredentialField],
}

#[derive(Serialize)]
pub struct CredentialField {
    /// Clé du champ dans l'objet `credential`.
    pub key: &'static str,
    pub label: &'static str,
    pub help: &'static str,
    pub placeholder: &'static str,
    /// `text`, `password` ou `select`.
    pub input: &'static str,
    /// Valeurs proposées quand `input == "select"`, la première par défaut.
    pub choices: &'static [&'static str],
    pub required: bool,
}

/// Champ visible (nom d'utilisateur, identifiant de jeton).
const fn cred_text(
    key: &'static str,
    label: &'static str,
    help: &'static str,
    placeholder: &'static str,
) -> CredentialField {
    CredentialField { key, label, help, placeholder, input: "text", choices: &[], required: true }
}

/// Champ masqué avec un bouton « afficher » : mot de passe, secret, phrase de passe.
const fn cred_secret(
    key: &'static str,
    label: &'static str,
    help: &'static str,
    placeholder: &'static str,
    required: bool,
) -> CredentialField {
    CredentialField { key, label, help, placeholder, input: "password", choices: &[], required }
}

const fn cred_select(
    key: &'static str,
    label: &'static str,
    help: &'static str,
    choices: &'static [&'static str],
) -> CredentialField {
    CredentialField { key, label, help, placeholder: "", input: "select", choices, required: false }
}

const NO_AUTH: CredentialView = CredentialView {
    kind: "none",
    label: "No authentication",
    help: "Nothing is sent: the device answers without credentials.",
    fields: &[],
};

const SNMP_COMMUNITY: CredentialView = CredentialView {
    kind: "snmp_community",
    label: "SNMP v1 / v2c (community)",
    help: "The simplest form: one shared word, sent unencrypted.",
    fields: &[cred_secret(
        "community",
        "SNMP community",
        "Most devices ship with \"public\". A read-only community is enough.",
        "public",
        true,
    )],
};

const SNMP_V3: CredentialView = CredentialView {
    kind: "snmp_v3",
    label: "SNMP v3 (user, authentication, privacy)",
    help: "Authenticated and encrypted. Needs a USM user on the device.",
    fields: &[
        cred_text("username", "User name", "The SNMP v3 user configured on the device.", "monitor"),
        cred_select(
            "auth_protocol",
            "Authentication protocol",
            "Must match what the device was configured with.",
            &["sha256", "sha512", "sha384", "sha224", "sha1", "md5"],
        ),
        cred_secret(
            "auth_passphrase",
            "Authentication passphrase",
            "At least 8 characters, as set on the device.",
            "",
            true,
        ),
        cred_select(
            "privacy_protocol",
            "Privacy protocol",
            "Encryption of the SNMP traffic. Ignored when the privacy passphrase is empty.",
            &["aes128", "aes192", "aes256", "des"],
        ),
        cred_secret(
            "privacy_passphrase",
            "Privacy passphrase",
            "Leave blank for authentication without encryption (authNoPriv).",
            "",
            false,
        ),
    ],
};

/// Identifiant d'un jeton Proxmox (VE ou PBS) : `user@realm!nom`, tel que le
/// produit l'affiche à la création.
const fn token_id_field(placeholder: &'static str) -> CredentialField {
    CredentialField {
        key: "token_id",
        label: "Token ID",
        help: "User, realm and token name, exactly as shown when the token was created.",
        placeholder,
        input: "text",
        choices: &[],
        required: true,
    }
}

/// Secret d'un jeton Proxmox : l'UUID montré une seule fois.
const TOKEN_SECRET_FIELD: CredentialField = cred_secret(
    "secret",
    "Secret",
    "The UUID shown once when the token was created. Stored encrypted, never shown again.",
    "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
    true,
);

const PROXMOX_TOKEN_LABEL: &str = "API token (recommended)";
const PROXMOX_TOKEN_HELP: &str =
    "No expiry and no session opened: the right choice for monitoring.";

/// Jeton d'API Proxmox VE, en deux morceaux tels que le produit les affiche.
const PROXMOX_TOKEN: CredentialView = CredentialView {
    kind: "api_token",
    label: PROXMOX_TOKEN_LABEL,
    help: PROXMOX_TOKEN_HELP,
    fields: &[token_id_field("dumbmonit@pve!monitor"), TOKEN_SECRET_FIELD],
};

const PBS_TOKEN: CredentialView = CredentialView {
    kind: "api_token",
    label: PROXMOX_TOKEN_LABEL,
    help: PROXMOX_TOKEN_HELP,
    fields: &[token_id_field("dumbmonit@pbs!monitor"), TOKEN_SECRET_FIELD],
};

/// Nom d'utilisateur Proxmox, realm compris.
const fn proxmox_user_field(placeholder: &'static str) -> CredentialField {
    cred_text("username", "User name", "User and realm, as in \"dumbmonit@pve\".", placeholder)
}

const PROXMOX_LOGIN_LABEL: &str = "Username / password";
const PROXMOX_LOGIN_HELP: &str =
    "Opens a two-hour session, renewed automatically. Use it only if tokens are not an option.";
const PASSWORD_FIELD: CredentialField = cred_secret("password", "Password", "", "", true);

/// Connexion par mot de passe à l'API Proxmox VE : ouvre un ticket de deux heures.
const PROXMOX_LOGIN: CredentialView = CredentialView {
    kind: "username_password",
    label: PROXMOX_LOGIN_LABEL,
    help: PROXMOX_LOGIN_HELP,
    fields: &[proxmox_user_field("dumbmonit@pve"), PASSWORD_FIELD],
};

/// Jeton d'API Proxmox Datacenter Manager. La console ne rend son ticket de
/// session que dans un cookie `HttpOnly` : le jeton est la seule forme
/// d'identifiant utilisable par une sonde, et de toute façon la bonne.
const PDM_TOKEN: CredentialView = CredentialView {
    kind: "api_token",
    label: PROXMOX_TOKEN_LABEL,
    help: PROXMOX_TOKEN_HELP,
    fields: &[token_id_field("dumbmonit@pdm!monitor"), TOKEN_SECRET_FIELD],
};

/// Connexion à Proxmox Mail Gateway : un utilisateur et son mot de passe.
///
/// PMG 9 ne délivre pas de jeton d'API — c'est la seule des quatre consoles
/// Proxmox dans ce cas : le ticket est le mécanisme normal, et le seul que
/// l'interface propose de créer.
const PMG_LOGIN: CredentialView = CredentialView {
    kind: "username_password",
    label: "Username / password (recommended)",
    help: "Opens a two-hour session, renewed automatically. Proxmox Mail Gateway does not issue API tokens: this is its normal way in.",
    fields: &[
        cred_text(
            "username",
            "User name",
            "User and realm, as in \"dumbmonit@pmg\".",
            "dumbmonit@pmg",
        ),
        PASSWORD_FIELD,
    ],
};

/// Jeton d'API, pour une passerelle qui en accepterait un.
///
/// PMG 9 n'en crée pas ; l'option existe pour une version ultérieure ou une
/// passerelle placée derrière un frontal qui en attend un, et le collecteur
/// envoie alors `Authorization: PMGAPIToken=…` comme pour PVE et PBS.
const PMG_TOKEN: CredentialView = CredentialView {
    kind: "api_token",
    label: "API token",
    help: "Only if your gateway offers API tokens: Proxmox Mail Gateway 9 and earlier do not create any. Use the username and password instead.",
    fields: &[token_id_field("dumbmonit@pmg!monitor"), TOKEN_SECRET_FIELD],
};

const PBS_LOGIN: CredentialView = CredentialView {
    kind: "username_password",
    label: PROXMOX_LOGIN_LABEL,
    help: PROXMOX_LOGIN_HELP,
    fields: &[proxmox_user_field("dumbmonit@pbs"), PASSWORD_FIELD],
};

const SYNOLOGY_LOGIN: CredentialView = CredentialView {
    kind: "username_password",
    label: "DSM account",
    help: "",
    fields: &[
        cred_text("username", "User name", "The DSM account created for monitoring.", "dumbmonit"),
        cred_secret(
            "password",
            "Password",
            "Two-step verification must be off for this account: a monitor cannot type a one-time code.",
            "",
            true,
        ),
    ],
};

/// Compte du contrôleur de gestion : utilisateur et mot de passe, envoyés en
/// Basic ou échangés contre une session selon l'option `auth`.
const REDFISH_LOGIN: CredentialView = CredentialView {
    kind: "username_password",
    label: "Management controller account",
    help: "",
    fields: &[
        cred_text(
            "username",
            "User name",
            "The read-only account created on the controller for monitoring.",
            "dumbmonit",
        ),
        cred_secret("password", "Password", "", "", true),
    ],
};

/// Clé d'API OPNsense : le fichier téléchargé à la création contient `key=…`
/// et `secret=…`, et le pare-feu les attend en authentification HTTP « basic ».
const OPNSENSE_KEY: CredentialView = CredentialView {
    kind: "username_password",
    label: "API key and secret",
    help: "The two lines of the file OPNsense downloads when the key is created.",
    fields: &[
        cred_text(
            "username",
            "API key",
            "The \"key\" line of the file OPNsense downloaded when you created the key.",
            "nJ8kQ2vF...",
        ),
        cred_secret(
            "password",
            "API secret",
            "The \"secret\" line of the same file. Stored encrypted, never shown again.",
            "",
            true,
        ),
    ],
};

/// Clé d'API TrueNAS, envoyée en `Authorization: Bearer`.
const TRUENAS_KEY: CredentialView = CredentialView {
    kind: "api_token",
    label: "API key",
    help: "The key TrueNAS shows once when it is created, as \"3-AbCd…\".",
    fields: &[cred_secret(
        "token",
        "API key",
        "Starts with a number and a dash. Stored encrypted, never shown again.",
        "3-…",
        true,
    )],
};

/// Compte de l'API XML de MDaemon : adresse de messagerie complète et mot de
/// passe, envoyés en HTTP Basic. Facultatif : sans lui, seuls les services sont
/// vérifiés.
const MDAEMON_LOGIN: CredentialView = CredentialView {
    kind: "username_password",
    label: "XML API account",
    help: "Also reads the version through the XML API. Leave the credential empty to watch the mail services only.",
    fields: &[
        cred_text(
            "username",
            "Email address",
            "The full email address of the account: MDaemon refuses a bare user name.",
            "dumbmonit@example.com",
        ),
        PASSWORD_FIELD,
    ],
};

/// Clé d'API REST de SecurityGateway (12.5 et suivants), envoyée en
/// `Authorization: Bearer`. Facultative : sans elle, seuls les services sont
/// vérifiés.
const SECURITY_GATEWAY_KEY: CredentialView = CredentialView {
    kind: "api_token",
    label: "API key (12.5 and later)",
    help: "Also reads the version and the performance counters through the REST API. Leave the credential empty to watch the services only.",
    fields: &[cred_secret(
        "token",
        "API key",
        "Shown once when the key is created. Stored encrypted, never shown again.",
        "",
        true,
    )],
};

/// Pas d'identifiant : les services seuls.
const MAIL_SERVICES_ONLY: CredentialView = CredentialView {
    kind: "none",
    label: "Services only",
    help: "Nothing is sent: DumbMonit connects to the mail ports and reads their greetings.",
    fields: &[],
};

const HTTP_TOKEN: CredentialView = CredentialView {
    kind: "api_token",
    label: "Bearer token",
    help: "Sent as \"Authorization: Bearer …\" on every request.",
    fields: &[cred_secret("token", "Token", "Stored encrypted, never shown again.", "", true)],
};

const HTTP_LOGIN: CredentialView = CredentialView {
    kind: "username_password",
    label: "Username / password",
    help: "Sent as HTTP basic authentication.",
    fields: &[
        cred_text("username", "User name", "", ""),
        cred_secret("password", "Password", "", "", true),
    ],
};

/// Champ texte libre.
const fn text(
    key: &'static str,
    label: &'static str,
    help: &'static str,
    placeholder: &'static str,
    default: &'static str,
) -> OptionView {
    OptionView {
        key,
        label,
        help,
        placeholder,
        default,
        required: false,
        input: "text",
        choices: &[],
    }
}

/// Champ numérique. Le défaut est transmis tel qu'écrit dans le code du collecteur.
const fn number(
    key: &'static str,
    label: &'static str,
    help: &'static str,
    placeholder: &'static str,
    default: &'static str,
) -> OptionView {
    OptionView {
        key,
        label,
        help,
        placeholder,
        default,
        required: false,
        input: "number",
        choices: &[],
    }
}

/// Case à cocher. Les collecteurs lisent `true`/`false` mais aussi `1`/`0`,
/// `oui`/`non` : le défaut est donné sous la forme canonique `true`/`false`.
const fn boolean(
    key: &'static str,
    label: &'static str,
    help: &'static str,
    default: bool,
) -> OptionView {
    OptionView {
        key,
        label,
        help,
        placeholder: "",
        default: if default { "true" } else { "false" },
        required: false,
        input: "boolean",
        choices: &[],
    }
}

/// Liste de choix fermée.
const fn select(
    key: &'static str,
    label: &'static str,
    help: &'static str,
    default: &'static str,
    choices: &'static [&'static str],
) -> OptionView {
    OptionView {
        key,
        label,
        help,
        placeholder: "",
        default,
        required: false,
        input: "select",
        choices,
    }
}

/// Délai propre à une sonde de disponibilité, commun aux cinq (`uptime/tags.rs`).
///
/// Le plafond de 60 s est celui du collecteur : au-delà, le délai global du
/// planificateur interromprait la sonde avant qu'elle n'ait pu enregistrer
/// l'indisponibilité.
const PROBE_TIMEOUT: OptionView = number(
    "timeout_seconds",
    "Timeout (seconds)",
    "Time after which the service is reported down if it has not answered. Between 1 and 60.",
    "5",
    "5",
);

/// Accepter un certificat que l'on ne peut pas vérifier (auto-signé, autorité
/// privée). Même clé et même sens pour HTTP, TLS, Proxmox et Synology.
const fn insecure_tls(help: &'static str) -> OptionView {
    boolean("insecure_tls", "Accept an unverifiable certificate", help, false)
}

/// Lever le garde-fou d'adresses des sondes (`collectors/uptime/guard.rs`) : la
/// boucle locale et le lien local sont refusés par défaut, parce qu'ils ne sont
/// joignables que depuis l'hôte de supervision lui-même.
const ALLOW_PRIVATE_TARGETS: OptionView = boolean(
    "allow_private_targets",
    "Allow loopback and link-local targets",
    "By default the check refuses addresses only the DumbMonit host itself can reach (127.0.0.1, ::1, 169.254.x.x). Enable this to monitor a service running on the DumbMonit host. Private LAN addresses (10.x, 192.168.x) are always allowed.",
    false,
);

/// Options lues par `collectors/uptime/http/options.rs`.
const HTTP_OPTIONS: &[OptionView] = &[
    select(
        "method",
        "HTTP method",
        "GET fits almost every case. HEAD avoids downloading the page when only the status code matters.",
        "GET",
        &["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"],
    ),
    text(
        "accepted_status",
        "Accepted status codes",
        "Codes or ranges considered normal, separated by commas. Any other code counts as down.",
        "200-299,301,404",
        "200-299",
    ),
    text(
        "keyword",
        "Expected keyword",
        "Text that must appear in the page. If it is missing, the service is reported down even though the page answers.",
        "Welcome",
        "",
    ),
    boolean(
        "keyword_absent",
        "Keyword must be absent",
        "Inverts the check: the presence of the keyword signals a failure. Useful for an error page that answers 200.",
        false,
    ),
    boolean(
        "keyword_case_sensitive",
        "Match keyword case",
        "By default, upper and lower case are treated the same.",
        false,
    ),
    text(
        "json_path",
        "JSON path to check",
        "For a JSON response: path of the value to check. Fill it in together with the expected value, never one without the other.",
        "$.status",
        "",
    ),
    text(
        "json_expect",
        "Expected JSON value",
        "Value the JSON path above must have, for example \"ok\" or \"true\".",
        "ok",
        "",
    ),
    text(
        "headers",
        "Extra headers",
        "One or more \"Name: value\" headers, separated by \"|\". Do not put secrets here: options are visible in the charts.",
        "Accept: application/json | X-Origin: dumbmonit",
        "",
    ),
    text(
        "body",
        "Request body",
        "Content sent with the request, for methods that expect one (POST, PUT…).",
        "",
        "",
    ),
    boolean(
        "follow_redirects",
        "Follow redirects",
        "Disable to monitor the redirect itself, for example a 301 to HTTPS.",
        true,
    ),
    number(
        "max_redirects",
        "Maximum redirects followed",
        "Between 1 and 20. No effect if redirects are not followed.",
        "10",
        "10",
    ),
    insecure_tls(
        "The check no longer fails on a self-signed certificate or one issued by a private authority.",
    ),
    ALLOW_PRIVATE_TARGETS,
    boolean(
        "check_certificate",
        "Read the certificate",
        "Over HTTPS, also records the certificate expiry date so you can be warned before it expires.",
        true,
    ),
    number(
        "max_body_bytes",
        "Maximum bytes read",
        "Beyond this, the rest of the page is not downloaded. The keyword and JSON path are only searched in this part.",
        "524288",
        "524288",
    ),
    text(
        "user_agent",
        "Announced identity (User-Agent)",
        "Name the check gives to the server. Change it if the server filters robots.",
        "DumbMonit/1.0",
        concat!("DumbMonit/", env!("CARGO_PKG_VERSION")),
    ),
    PROBE_TIMEOUT,
];

/// Options lues par `collectors/uptime/tcp.rs`.
const TCP_OPTIONS: &[OptionView] = &[
    number(
        "port",
        "Port",
        "Port to open, if the address does not already give it as \"host:port\". One of the two is required.",
        "22",
        "",
    ),
    ALLOW_PRIVATE_TARGETS,
    PROBE_TIMEOUT,
];

/// Options lues par `collectors/uptime/dns/options.rs`.
const DNS_OPTIONS: &[OptionView] = &[
    select(
        "record_type",
        "Record type",
        "A for an IPv4 address, AAAA for IPv6, MX for mail, CNAME for an alias…",
        "A",
        &["A", "AAAA", "CNAME", "MX", "TXT", "NS", "SOA", "SRV", "PTR", "CAA"],
    ),
    text(
        "resolver",
        "DNS server to query",
        "IP address of a resolver, port optional. Empty: the system resolver. Handy to check your own DNS server.",
        "1.1.1.1 or 10.0.0.1:5353",
        "",
    ),
    text(
        "expect",
        "Expected values",
        "Values that must all appear in the answer, separated by commas. Empty: only the resolution is checked.",
        "93.184.216.34",
        "",
    ),
    select(
        "expect_mode",
        "Comparison",
        "\"contains\": each expected value must appear somewhere in the answer. \"exact\": the answer must hold those values and nothing else, so a record added beside the right one fails the check.",
        "contains",
        &["contains", "exact"],
    ),
    text(
        "forbid",
        "Forbidden values",
        "Values that must never appear in the answer, separated by commas: a former host's address, an expired validation record. Checked before the expected ones.",
        "198.51.100.7",
        "",
    ),
    PROBE_TIMEOUT,
];

/// Options lues par `collectors/uptime/ping/options.rs`.
const PING_OPTIONS: &[OptionView] = &[
    number("count", "Number of echoes", "Packets sent on each poll, from 1 to 20.", "4", "4"),
    number(
        "packet_timeout_ms",
        "Wait per packet (ms)",
        "Time allowed for each reply, from 50 to 10,000 ms.",
        "1000",
        "1000",
    ),
    number(
        "interval_ms",
        "Gap between two echoes (ms)",
        "Pause between two echoes, from 0 to 5,000 ms.",
        "100",
        "100",
    ),
    number(
        "payload_bytes",
        "Payload size (bytes)",
        "Data carried by each echo, from 0 to 1,400 bytes.",
        "56",
        "56",
    ),
    select(
        "ip_version",
        "IP version",
        "\"auto\" takes the first resolved address; force 4 or 6 if the host has both and one does not answer.",
        "auto",
        &["auto", "4", "6"],
    ),
    number(
        "loss_threshold_percent",
        "Tolerated loss (%)",
        "Above this percentage of lost packets, the host is reported down. 100: only a total loss counts, partial loss stays visible in the charts.",
        "100",
        "100",
    ),
    number(
        "timeout_seconds",
        "Timeout (seconds)",
        "Total budget for the poll, from 1 to 60. Refused if it does not cover \"number of echoes × wait per packet\".",
        "5",
        "5",
    ),
];

/// Options lues par `collectors/uptime/tls/options.rs`.
const TLS_OPTIONS: &[OptionView] = &[
    text(
        "server_name",
        "Server name (SNI)",
        "Domain name announced to the server and checked in the certificate. Set it when the address is an IP behind a reverse proxy.",
        "www.example.com",
        "",
    ),
    insecure_tls(
        "An unverifiable chain no longer counts as a failure: the expiry date is still recorded.",
    ),
    ALLOW_PRIVATE_TARGETS,
    PROBE_TIMEOUT,
];

/// Nom annoncé en SNI, commun aux sondes applicatives chiffrées.
const fn server_name_option(help: &'static str) -> OptionView {
    text("server_name", "Server name (SNI)", help, "mail.example.com", "")
}

/// Options lues par `collectors/uptime/smtp/options.rs`.
const SMTP_OPTIONS: &[OptionView] = &[
    select(
        "security",
        "Encryption",
        "STARTTLS starts in clear text and upgrades (port 587), TLS encrypts from the first byte (port 465), None never encrypts (port 25) and only suits a relay on your own network.",
        "starttls",
        &["starttls", "tls", "none"],
    ),
    number(
        "port",
        "Port",
        "Used if the address does not give one. Empty: 587 with STARTTLS, 465 with TLS, 25 without encryption.",
        "587",
        "",
    ),
    text(
        "helo_name",
        "Name announced (EHLO)",
        "Name the check gives when it introduces itself. A strict relay refuses a name it cannot resolve.",
        "monit.example.com",
        "dumbmonit",
    ),
    text(
        "expect_capability",
        "Expected extension",
        "Extension the server must advertise in its EHLO answer, for example STARTTLS or AUTH. Empty: no expectation.",
        "STARTTLS",
        "",
    ),
    server_name_option(
        "Domain name announced to the server and checked in the certificate. Set it when the address is an IP.",
    ),
    insecure_tls(
        "An unverifiable chain no longer counts as a failure: the expiry date is still recorded.",
    ),
    ALLOW_PRIVATE_TARGETS,
    PROBE_TIMEOUT,
];

/// Options lues par `collectors/uptime/sql/options.rs`, moteur PostgreSQL.
const POSTGRES_OPTIONS: &[OptionView] = &[
    number("port", "Port", "Used if the address does not give one.", "5432", "5432"),
    text(
        "database",
        "Database",
        "Database opened on connection. The monitoring account must be allowed to connect to it.",
        "postgres",
        "postgres",
    ),
    text(
        "query",
        "Query",
        "Run on every check. The default reads no table, so the account needs no privilege beyond connecting. A query returning one number turns it into a chart.",
        "SELECT count(*) FROM jobs WHERE failed",
        "SELECT 1",
    ),
    text(
        "expect",
        "Expected value",
        "Value the first column of the first row must hold. Empty: only the query succeeding is checked.",
        "1",
        "",
    ),
    select(
        "sslmode",
        "Encryption",
        "\"prefer\" encrypts when the server offers it, \"require\" refuses to connect without it, \"verify-full\" also checks the certificate and the name, \"disable\" never encrypts.",
        "prefer",
        &["prefer", "require", "verify-full", "disable"],
    ),
    ALLOW_PRIVATE_TARGETS,
    PROBE_TIMEOUT,
];

/// Options lues par `collectors/uptime/sql/options.rs`, moteur MySQL/MariaDB.
const MYSQL_OPTIONS: &[OptionView] = &[
    number("port", "Port", "Used if the address does not give one.", "3306", "3306"),
    text(
        "database",
        "Database",
        "Database opened on connection. Empty: none, which is enough for the default query.",
        "monitoring",
        "",
    ),
    text(
        "query",
        "Query",
        "Run on every check. The default reads no table, so the account needs no privilege beyond connecting. A query returning one number turns it into a chart.",
        "SELECT count(*) FROM jobs WHERE failed",
        "SELECT 1",
    ),
    text(
        "expect",
        "Expected value",
        "Value the first column of the first row must hold. Empty: only the query succeeding is checked.",
        "1",
        "",
    ),
    select(
        "sslmode",
        "Encryption",
        "\"prefer\" encrypts when the server offers it, \"require\" refuses to connect without it, \"verify-full\" also checks the certificate and the name, \"disable\" never encrypts.",
        "prefer",
        &["prefer", "require", "verify-full", "disable"],
    ),
    ALLOW_PRIVATE_TARGETS,
    PROBE_TIMEOUT,
];

/// Options lues par `collectors/uptime/mqtt/options.rs`.
const MQTT_OPTIONS: &[OptionView] = &[
    boolean(
        "tls",
        "Encrypted connection",
        "Encrypts the session from the first byte, as brokers do on port 8883.",
        false,
    ),
    number(
        "port",
        "Port",
        "Used if the address does not give one. Empty: 1883 in clear text, 8883 encrypted.",
        "1883",
        "",
    ),
    text(
        "topic",
        "Topic",
        "Topic the check subscribes to. Empty: it only connects, which already tells you the broker is alive and accepts your account.",
        "home/living-room/temperature",
        "",
    ),
    boolean(
        "expect_message",
        "Expect a retained message",
        "A message must arrive on that topic before the timeout. Only retained messages arrive right away: a topic that is merely published to from time to time will look silent.",
        false,
    ),
    text(
        "expect",
        "Expected content",
        "Text that message must contain. Filling it in implies expecting a message.",
        "online",
        "",
    ),
    text(
        "client_id",
        "Client identifier",
        "Name announced to the broker. The default derives from the device, so two checks never disconnect each other.",
        "dumbmonit-3",
        "",
    ),
    server_name_option(
        "Domain name announced to the broker and checked in the certificate. Set it when the address is an IP.",
    ),
    insecure_tls(
        "An unverifiable chain no longer counts as a failure: the expiry date is still recorded.",
    ),
    ALLOW_PRIVATE_TARGETS,
    PROBE_TIMEOUT,
];

/// Options lues par `collectors/uptime/websocket/options.rs`.
const WEBSOCKET_OPTIONS: &[OptionView] = &[
    text(
        "path",
        "Path",
        "Path of the opening request. Taken from the address when it carries one.",
        "/api/websocket",
        "",
    ),
    number(
        "port",
        "Port",
        "Used if the address does not give one. Empty: 443 for wss, 80 for ws.",
        "8123",
        "",
    ),
    text(
        "send",
        "Frame to send",
        "Text frame sent once the connection is open. Empty: nothing is sent.",
        "{\"type\":\"ping\"}",
        "",
    ),
    text(
        "expect",
        "Expected content",
        "Text a received frame must contain. Empty and with nothing to send, only the opening handshake is checked.",
        "auth_required",
        "",
    ),
    text(
        "subprotocol",
        "Subprotocol",
        "Value of Sec-WebSocket-Protocol. The check fails if the server picks a different one.",
        "json",
        "",
    ),
    text(
        "origin",
        "Origin",
        "Value of the Origin header, which some servers require before upgrading.",
        "https://home.example.com",
        "",
    ),
    server_name_option(
        "Domain name announced to the server and checked in the certificate. Set it when the address is an IP.",
    ),
    insecure_tls(
        "An unverifiable chain no longer counts as a failure: the expiry date is still recorded.",
    ),
    ALLOW_PRIVATE_TARGETS,
    PROBE_TIMEOUT,
];

/// Identifiants d'un relais de messagerie ou d'un courtier MQTT.
const APP_LOGIN: CredentialView = CredentialView {
    kind: "username_password",
    label: "Username / password",
    help: "Sent once the connection is encrypted, never before.",
    fields: &[
        cred_text("username", "User name", "", "monitoring"),
        cred_secret("password", "Password", "Stored encrypted, never shown again.", "", true),
    ],
};

/// Identifiants d'un compte de base de données.
const SQL_LOGIN: CredentialView = CredentialView {
    kind: "username_password",
    label: "Username / password",
    help: "A read-only account created for monitoring, never an application one.",
    fields: &[
        cred_text("username", "User name", "", "dumbmonit"),
        cred_secret("password", "Password", "Stored encrypted, never shown again.", "", true),
    ],
};

/// Options lues par `collectors/push/mod.rs` (`Settings::from_target`).
const PUSH_OPTIONS: &[OptionView] = &[
    text(
        "expected_interval",
        "Expected interval",
        "How often the job is supposed to call in: 30m, 1h, 6h, 24h, 7d (or a number of seconds). A missed call is declared once this interval plus the grace period has passed.",
        "24h",
        "24h",
    ),
    text(
        "grace",
        "Grace period",
        "Extra time tolerated after the expected interval before the heartbeat counts as missed: a percentage of the interval (10%) or a fixed duration (15m). Never less than one minute.",
        "10%",
        "10%",
    ),
];

/// Options lues par `collectors/proxmox/options.rs`.
const PROXMOX_OPTIONS: &[OptionView] = &[
    number("port", "API port", "Used if the address does not give a port.", "8006", "8006"),
    insecure_tls(
        "Proxmox ships with a self-signed certificate by default: enable this if the connection is refused for that reason.",
    ),
    number(
        "request_timeout_seconds",
        "Timeout per request (seconds)",
        "Time allowed for each API call, from 1 to 120.",
        "10",
        "10",
    ),
    number(
        "backup_lookback_days",
        "Backup lookback (days)",
        "Older backup tasks are not examined, from 1 to 3650.",
        "31",
        "31",
    ),
    boolean(
        "scan_backup_storage",
        "Inventory backup archives",
        "Scans the backup storages to date the last backup of each machine. Disable if the storage is slow to answer.",
        true,
    ),
    text(
        "nodes",
        "Monitored nodes",
        "Names of the nodes to monitor, separated by commas. Empty: every node in the cluster.",
        "pve1, pve2",
        "",
    ),
    boolean(
        "ha",
        "Watch high availability",
        "Reads the HA manager state: quorum, master, LRMs and the state of each HA resource.",
        true,
    ),
    boolean(
        "backup_jobs",
        "Watch backup jobs",
        "Reads the scheduled backup jobs (next run, last result) and lists the guests no job covers.",
        true,
    ),
    boolean(
        "scan_snapshots",
        "Inventory snapshots",
        "Lists the snapshots of every VM and container to report their number and age. One API call per guest.",
        true,
    ),
    number(
        "max_snapshot_guests",
        "Snapshot inventory limit",
        "Maximum number of guests whose snapshots are listed per probe, from 1 to 10000. Beyond it, the remaining guests are counted as skipped.",
        "200",
        "200",
    ),
    boolean(
        "replication",
        "Watch replication jobs",
        "Reads the state of ZFS replication jobs on every node.",
        true,
    ),
    boolean(
        "ceph",
        "Watch Ceph",
        "Reads the Ceph cluster health, OSDs and usage. Silently skipped when Ceph is not set up.",
        true,
    ),
    boolean(
        "updates",
        "Count pending updates",
        "Lists the packages waiting for an update on each node. Needs Sys.Modify on \"/nodes\" (see the docs); silently skipped otherwise.",
        true,
    ),
    boolean(
        "certificates",
        "Watch node certificates",
        "Reports the days left before each node certificate expires.",
        true,
    ),
    boolean(
        "guest_agent",
        "Ask the QEMU guest agent",
        "For each running VM: balloon memory and, when the guest agent is enabled, the disk usage seen from inside (needs VM.GuestAgent.Audit, or VM.Monitor before Proxmox VE 9). Silently skipped when the agent is absent.",
        true,
    ),
    boolean(
        "disks",
        "Watch physical disks",
        "SMART health, wearout and temperature of every disk of each node.",
        true,
    ),
    boolean(
        "zfs",
        "Watch ZFS pools",
        "Health, capacity and fragmentation of the ZFS pools of each node.",
        true,
    ),
    boolean(
        "packages",
        "Detect package changes",
        "Compares the installed Proxmox packages with the previous probe and reports an upgrade for one hour.",
        true,
    ),
    boolean(
        "subscription",
        "Watch subscription and repositories",
        "Subscription status and APT repositories of each node (enterprise without subscription, unreadable sources).",
        true,
    ),
    boolean(
        "cluster_resources",
        "Read the cluster inventory",
        "One call lists every node, guest, storage and pool of the cluster. Keep it on: it is the only source that still sees the guests of a node that stopped answering, and it saves two calls per node.",
        true,
    ),
    boolean(
        "services",
        "Watch node services",
        "State of the Proxmox daemons of each node (pvestatd, pveproxy, pve-cluster, corosync…) and the version installed on it. A stopped pvestatd leaves the whole cluster showing frozen numbers.",
        true,
    ),
    boolean(
        "network",
        "Watch node networking",
        "Bridges, bonds and VLANs of each node with their link state, plus the traffic counters of each guest network card.",
        true,
    ),
    boolean(
        "lvm",
        "Watch LVM and thin pools",
        "Volume groups, LVM thin pools (data and metadata fill) and PVE-managed directory mounts. A full thin pool puts every guest on it read-only.",
        true,
    ),
    boolean(
        "ceph_detail",
        "Watch Ceph in detail",
        "Per-OSD state, usage and latency, per-pool usage, CephFS, OSD flags and muted health checks. Needs \"Watch Ceph\" to be on; silently skipped when Ceph is not set up.",
        true,
    ),
    boolean(
        "backup_volumes",
        "Check what backup jobs include",
        "For each scheduled job, which guests it covers and which of their disks it actually writes. Catches a job that succeeds every night while skipping a data disk.",
        true,
    ),
    boolean(
        "guest_os",
        "Read guest OS and addresses",
        "Operating system and IP addresses seen from inside each running guest (QEMU guest agent for VMs, the container namespace for containers). Refreshed once an hour, not at every probe.",
        true,
    ),
    boolean(
        "metrics_export",
        "Ingest the full RRD metric stream",
        "Reads /cluster/metrics/export, the stream Proxmox's own metric servers consume: everything pvestatd measures, including per-node pressure stall (PSI), and every point since the last probe rather than just the current one. Off by default: the series it adds (…_rrd_…) partly repeat the ones already collected. Needs Sys.Audit on \"/\".",
        false,
    ),
];

/// Options lues par `collectors/pbs/options.rs`.
const PBS_OPTIONS: &[OptionView] = &[
    number("port", "API port", "Used if the address does not give a port.", "8007", "8007"),
    insecure_tls(
        "Proxmox Backup Server ships with a self-signed certificate by default: enable this if the connection is refused for that reason.",
    ),
    number(
        "request_timeout_seconds",
        "Timeout per request (seconds)",
        "Time allowed for each API call, from 1 to 120. Listing the snapshots of a large datastore can take several seconds.",
        "15",
        "15",
    ),
    number(
        "task_lookback_hours",
        "Task window (hours)",
        "Older tasks are not counted among the failures, from 1 to 8760.",
        "24",
        "24",
    ),
    text(
        "datastores",
        "Monitored datastores",
        "Names of the datastores to monitor, separated by commas. Empty: every datastore.",
        "main, archive",
        "",
    ),
    number(
        "max_groups",
        "Backup group limit",
        "Maximum number of backed-up machines producing series; beyond it, the oldest are ignored. From 1 to 100000.",
        "500",
        "500",
    ),
    boolean(
        "jobs",
        "Watch sync, verify and prune jobs",
        "Reads the job lists to report each job's last result and next run. Needs Datastore.Audit on the datastores, plus Remote.Audit for sync jobs.",
        true,
    ),
    boolean(
        "updates",
        "Count pending updates",
        "Lists the packages waiting for an update on the backup server. Needs Sys.Audit on \"/\"; silently skipped otherwise.",
        true,
    ),
    boolean(
        "disks",
        "Watch disks and ZFS pools",
        "Reads the physical disks (SMART verdict, SSD wear) and the ZFS pools of the backup server (/nodes/localhost/disks). Needs Sys.Audit on \"/system\"; silently skipped otherwise.",
        true,
    ),
    boolean(
        "services",
        "Watch the server's services",
        "Reads the systemd units of the backup server and reports whether the ones it cannot do without are running. Needs Sys.Audit on \"/system\"; silently skipped otherwise.",
        true,
    ),
    boolean(
        "datastore_details",
        "Read datastore detail",
        "Two more calls per datastore: how many machines and snapshots it holds, what is reading or writing it right now, and whether it is held for maintenance. Cheap, and it explains a garbage collection that will not finish.",
        true,
    ),
    boolean(
        "traffic_control",
        "Watch traffic limits",
        "Reads the rate limits configured on the backup server and what they are carrying right now. Needs Sys.Audit; silently skipped otherwise.",
        true,
    ),
    boolean(
        "certificates",
        "Watch certificate expiry",
        "Reads the certificate the web interface serves and warns before it expires. Off by default: Proxmox Backup Server guards this one call behind Sys.Modify, a write privilege a monitoring token should not be given. Turn it on only if you granted it.",
        false,
    ),
    boolean(
        "tape",
        "Watch tape backups",
        "Reads the tape tier: backup jobs and their last run, drives, changers, media pools and the tapes themselves. Off by default, since most installations have no tape hardware. Needs Tape.Audit on \"/tape\".",
        false,
    ),
];

/// Options lues par `collectors/pdm/options.rs`.
const PDM_OPTIONS: &[OptionView] = &[
    number("port", "API port", "Used if the address does not give a port.", "8443", "8443"),
    insecure_tls(
        "Proxmox Datacenter Manager ships with a self-signed certificate by default: enable this if the connection is refused for that reason.",
    ),
    number(
        "request_timeout_seconds",
        "Timeout per request (seconds)",
        "Time allowed for each API call, from 1 to 120. The console relays calls to every federated instance, so one slow site holds the whole answer.",
        "20",
        "20",
    ),
    text(
        "node",
        "Console node name",
        "Name of the node that runs the console. A Datacenter Manager has a single node and calls it \"localhost\".",
        "localhost",
        "localhost",
    ),
    number(
        "task_lookback_hours",
        "Task window (hours)",
        "Older tasks are not counted among the failures, from 1 to 8760.",
        "24",
        "24",
    ),
    number(
        "max_age_seconds",
        "Accepted cache age (seconds)",
        "How stale the console's own inventory may be before it queries the federated instances again, from 0 to 3600. Zero asks for fresh data on every probe, which puts every cluster back on the line.",
        "60",
        "60",
    ),
    text(
        "remotes",
        "Monitored instances",
        "Names of the federated instances to monitor, separated by commas. Empty: every instance.",
        "site-a, site-b",
        "",
    ),
    number(
        "max_remotes",
        "Instance version limit",
        "Maximum number of instances asked for their version on each probe, from 1 to 1000.",
        "100",
        "100",
    ),
    boolean(
        "versions",
        "Read each instance version",
        "One call per federated instance, to report its version and to flag the ones left behind their peers. An instance that does not answer counts as unreachable.",
        true,
    ),
    boolean(
        "tasks",
        "Watch tasks across instances",
        "Reads the task list of every federated instance to report the failures. Needs Resource.Audit; silently skipped otherwise.",
        true,
    ),
    boolean(
        "node_status",
        "Watch the console host",
        "Reads the CPU, memory, root filesystem, uptime, certificates and subscription of the machine running the console. Needs Sys.Audit on \"/system\"; silently skipped otherwise.",
        true,
    ),
    boolean(
        "updates",
        "Count pending updates",
        "Lists the packages waiting for an update on the console itself. Needs Sys.Audit; silently skipped otherwise.",
        true,
    ),
    boolean(
        "remote_updates",
        "Count updates on each instance",
        "Reads the console's update summary for the federated instances. Off by default: the console guards that path with Resource.Modify, a write privilege a monitoring account has no reason to hold.",
        false,
    ),
];

/// Options lues par `collectors/pmg/options.rs`.
const PMG_OPTIONS: &[OptionView] = &[
    number("port", "API port", "Used if the address does not give a port.", "8006", "8006"),
    insecure_tls(
        "Proxmox Mail Gateway ships with a self-signed certificate by default: enable this if the connection is refused for that reason.",
    ),
    number(
        "request_timeout_seconds",
        "Timeout per request (seconds)",
        "Time allowed for each API call, from 1 to 120. Reading a queue runs a Postfix command on the gateway and can take a few seconds.",
        "15",
        "15",
    ),
    text(
        "node",
        "Monitored node",
        "Name of the cluster node to monitor. Empty: every node the gateway lists.",
        "mail1",
        "",
    ),
    number(
        "recent_hours",
        "Traffic window (hours)",
        "How far back the traffic curve goes, from 1 to 24. The daily totals always cover the current day.",
        "12",
        "12",
    ),
    boolean(
        "queues",
        "Watch the mail queues",
        "Reads the incoming, active, deferred and hold queues and the age of the oldest message in each. A growing deferred queue is the first sign that mail is stuck.",
        true,
    ),
    boolean(
        "quarantine",
        "Count the quarantines",
        "Reads how many messages the spam and virus quarantines hold, and how much disk they use. Counts only: no subject, sender or message content is ever read.",
        true,
    ),
    boolean(
        "attachment_quarantine",
        "Also count the attachment quarantine",
        "That quarantine has no count call: it has to be listed to be counted. Off by default; only the number of entries is kept.",
        false,
    ),
    boolean(
        "signatures",
        "Watch the signature databases",
        "Reads the age of the ClamAV virus databases and of the SpamAssassin rule channels. Out-of-date signatures fail silently: the gateway keeps filtering, badly.",
        true,
    ),
    boolean(
        "services",
        "Watch the services",
        "Reads the state of postfix, pmg-smtp-filter, pmgpolicy and the other units of each node.",
        true,
    ),
    boolean(
        "certificates",
        "Watch the certificates",
        "Reads the certificates the web interface serves and warns before they expire.",
        true,
    ),
    boolean(
        "updates",
        "Count pending updates",
        "Lists the packages waiting for an update on each node. Needs the Audit role; silently skipped otherwise.",
        true,
    ),
    boolean(
        "subscription",
        "Read the subscription",
        "Reads the subscription status of each node. An installation without a key reports \"notfound\", which is not an error.",
        true,
    ),
];

/// Options lues par `collectors/redfish/options.rs`.
const REDFISH_OPTIONS: &[OptionView] = &[
    number("port", "HTTPS port", "Used if the address does not give a port.", "443", "443"),
    insecure_tls(
        "Management controllers ship with a self-signed certificate: enable this if the connection is refused for that reason.",
    ),
    number(
        "request_timeout_seconds",
        "Timeout per request (seconds)",
        "Time allowed for each Redfish call, from 1 to 120. A management controller is a small processor: a few seconds per answer is normal.",
        "8",
        "8",
    ),
    select(
        "auth",
        "Authentication",
        "basic sends the user name and password with every request and leaves nothing open on the controller. session logs in once and reuses the session token; use it only if the controller refuses basic authentication, since controllers have few session slots.",
        "basic",
        &["basic", "session"],
    ),
    boolean(
        "storage",
        "Watch the drives",
        "Reads the storage controllers and every drive behind them: health, predicted failure, remaining SSD life. One request per drive.",
        true,
    ),
    boolean(
        "logs",
        "Count log entries",
        "Counts the entries of the system event log and of the controller's own log, by severity. The messages themselves are never read.",
        true,
    ),
];

/// Options lues par `collectors/opnsense/options.rs`.
const OPNSENSE_OPTIONS: &[OptionView] = &[
    select(
        "scheme",
        "Protocol",
        "HTTPS fits a firewall out of the box. HTTP only if the web interface is served in clear text.",
        "https",
        &["https", "http"],
    ),
    number("port", "Web interface port", "Used if the address does not give a port.", "443", "443"),
    insecure_tls(
        "OPNsense ships with a self-signed certificate by default: enable this if the connection is refused for that reason.",
    ),
    number(
        "request_timeout_seconds",
        "Timeout per request (seconds)",
        "Time allowed for each API call, from 1 to 120. Most answer instantly; the firmware status is the slow one.",
        "15",
        "15",
    ),
    boolean(
        "gateways",
        "Watch the gateways",
        "Reads what dpinger says about each gateway: up or down, round-trip delay and packet loss. On a multi-WAN firewall this is the one thing that fails without anyone noticing.",
        true,
    ),
    boolean(
        "interfaces",
        "Watch the interfaces",
        "Reads each interface's link state, addresses and byte, packet, error and drop counters. The addresses are where the current public address shows up.",
        true,
    ),
    boolean(
        "firewall",
        "Watch the state table",
        "Reads how many connections pf is tracking and the configured limit. A firewall that fills its state table refuses connections with no other symptom.",
        true,
    ),
    boolean(
        "dhcp",
        "Count the DHCP leases",
        "Counts the leases the firewall is handing out, whichever server it runs. Counts only: no address, host name or hardware address is ever read.",
        true,
    ),
    boolean(
        "vpn",
        "Watch the VPN tunnels",
        "Reads WireGuard, OpenVPN and IPsec: which tunnels exist and which ones are up. A plugin that is not installed is skipped in silence.",
        true,
    ),
    boolean(
        "unbound",
        "Watch the resolver",
        "Reads whether Unbound is answering. A firewall that routes but no longer resolves looks healthy to everyone except the machines behind it.",
        true,
    ),
    boolean(
        "services",
        "Watch the services",
        "Reads the list of services the firewall manages and which of them are running.",
        true,
    ),
    boolean(
        "carp",
        "Watch CARP",
        "Reads the virtual addresses of a high-availability pair and whether this firewall holds them. Also reports persistent maintenance mode, which is the state everyone forgets to switch off.",
        true,
    ),
    boolean(
        "firmware",
        "Watch for updates",
        "Reads the result of the firewall's last update check and whether a reboot is pending. The check itself is never triggered: that would send the firewall to the mirror on every measurement.",
        true,
    ),
    boolean(
        "temperature",
        "Read the temperature sensors",
        "Reads the CPU and board sensors the firewall exposes. A machine without sensors reports none, which is not an error.",
        true,
    ),
];

/// Options lues par `collectors/truenas/options.rs`.
const TRUENAS_OPTIONS: &[OptionView] = &[
    select(
        "scheme",
        "Protocol",
        "Keep HTTPS: TrueNAS revokes an API key it receives over plain HTTP from another machine. HTTP only behind a reverse proxy on the NAS itself.",
        "https",
        &["https", "http"],
    ),
    number("port", "Web interface port", "Used if the address does not give a port.", "443", "443"),
    insecure_tls(
        "TrueNAS ships with a self-signed certificate by default: enable this if the connection is refused for that reason.",
    ),
    number(
        "request_timeout_seconds",
        "Timeout per request (seconds)",
        "Time allowed for each API call, from 1 to 120. The service list probes every daemon and can take a while on a busy NAS.",
        "20",
        "20",
    ),
    boolean(
        "datasets",
        "Watch the datasets",
        "Reads each dataset's usage against its quota, its snapshot count and when its snapshots last changed. Counts only, from ZFS's own cache: no snapshot is listed.",
        true,
    ),
    boolean(
        "disks",
        "Watch the disks",
        "Reads the disk inventory and the temperatures TrueNAS already has in cache. A sleeping disk is never woken up for a measurement.",
        true,
    ),
    boolean(
        "smart",
        "Read the SMART test results",
        "Reads the last self-test of each disk. No test is ever started. TrueNAS 25.10 no longer offers this call; it is then skipped in silence.",
        true,
    ),
    boolean(
        "alerts",
        "Read TrueNAS's own alerts",
        "Reads the alert list TrueNAS keeps itself — pool state, SMART, capacity, certificates, failed replications. Dismissed alerts are left out.",
        true,
    ),
    boolean(
        "tasks",
        "Watch replication and snapshot tasks",
        "Reads the state of every replication and periodic snapshot task, its last run and the last snapshot it handled.",
        true,
    ),
    boolean(
        "services",
        "Watch the services",
        "Reads the services set to start with the NAS and whether they are running. A service you switched off is not watched.",
        true,
    ),
];

/// Options lues par `collectors/synology/options.rs`.
const SYNOLOGY_OPTIONS: &[OptionView] = &[
    select(
        "scheme",
        "Protocol",
        "HTTPS fits a NAS fresh out of the box. HTTP only if DSM listens in clear text only.",
        "https",
        &["https", "http"],
    ),
    number(
        "port",
        "DSM port",
        "Used if the address does not give a port. Empty: 5001 over HTTPS, 5000 over HTTP.",
        "5001",
        "",
    ),
    insecure_tls(
        "A NAS ships with a self-signed certificate by default: enable this if the connection is refused for that reason.",
    ),
    number(
        "request_timeout_seconds",
        "Timeout per request (seconds)",
        "Time allowed for each API call, from 1 to 120. The storage inventory can wake up sleeping disks.",
        "15",
        "15",
    ),
    boolean(
        "abb",
        "Watch Active Backup for Business",
        "Reads the Active Backup for Business tasks (PCs, servers, virtual machines, file servers): last result, age of the last successful backup, schedule. Needs the package installed and an account allowed to use it; a NAS without the package is simply skipped.",
        true,
    ),
];

/// Options lues par `collectors/mdaemon/email_server.rs`.
const MDAEMON_OPTIONS: &[OptionView] = &[
    text(
        "services",
        "Services",
        "Comma-separated list of what to check: smtp, msa, smtps, pop3, pop3s, imap, imaps, webmail, remote_admin, remote_admin_https, xmpp. Write name:port for a port other than the default, and any other name with its port for a service of your own. none checks no port.",
        "smtp,imap,webmail",
        "smtp,imap,webmail",
    ),
    number(
        "request_timeout_seconds",
        "Timeout per check (seconds)",
        "Time allowed for each connection, greeting and API call, from 1 to 60.",
        "10",
        "10",
    ),
    number(
        "api_port",
        "XML API port",
        "The Remote Administration port that serves /MdMgmtWS/. Used only with an account.",
        "444",
        "444",
    ),
    boolean(
        "api_tls",
        "XML API over HTTPS",
        "Untick only for Remote Administration over plain HTTP (port 1000 by default): the password then crosses the network in clear.",
        true,
    ),
    insecure_tls(
        "Remote Administration often uses a self-signed certificate: enable this if the connection is refused for that reason.",
    ),
];

/// Options lues par `collectors/mdaemon/security_gateway.rs`.
const SECURITY_GATEWAY_OPTIONS: &[OptionView] = &[
    text(
        "services",
        "Services",
        "Comma-separated list of what to check: smtp, smtps, web, web_https. Write name:port for a port other than the default, and any other name with its port for a service of your own. none checks no port.",
        "smtp,web",
        "smtp,web",
    ),
    number(
        "request_timeout_seconds",
        "Timeout per check (seconds)",
        "Time allowed for each connection, greeting and API call, from 1 to 60.",
        "10",
        "10",
    ),
    number(
        "api_port",
        "REST API port",
        "The web interface port that serves /api/v1. Used only with an API key.",
        "4443",
        "4443",
    ),
    boolean(
        "api_tls",
        "REST API over HTTPS",
        "Untick only for the web interface over plain HTTP (port 4000 by default): the key then crosses the network in clear.",
        true,
    ),
    insecure_tls(
        "The web interface often uses a self-signed certificate on 4443: enable this if the connection is refused for that reason.",
    ),
    boolean(
        "counters",
        "Read the performance counters",
        "Reads the read-only performance counters the REST API publishes (queues, quarantine, sessions) and keeps every numeric value as is.",
        true,
    ),
];

/// Jeton d'accès Graylog : envoyé en « basic », le jeton comme nom
/// d'utilisateur et le mot `token` comme mot de passe
/// (`collectors/observability/client.rs`).
const GRAYLOG_TOKEN: CredentialView = CredentialView {
    kind: "api_token",
    label: "Access token (recommended)",
    help: "A token of the dedicated Graylog user, created from its Edit tokens page.",
    fields: &[cred_secret(
        "token",
        "Access token",
        "Graylog shows it once. Stored encrypted, never shown again.",
        "",
        true,
    )],
};

const GRAYLOG_LOGIN: CredentialView = CredentialView {
    kind: "username_password",
    label: "User name / password",
    help: "The dedicated Graylog user itself, sent as HTTP basic authentication.",
    fields: &[
        cred_text("username", "User name", "", "dumbmonit"),
        cred_secret("password", "Password", "", "", true),
    ],
};

/// Protocole des serveurs de journaux et de métriques : en clair par défaut,
/// comme ils écoutent tant qu'aucun proxy inverse n'est placé devant.
const OBSERVABILITY_SCHEME: OptionView = select(
    "scheme",
    "Protocol",
    "These servers listen over plain HTTP unless you put them behind a reverse proxy with TLS. HTTPS then.",
    "http",
    &["http", "https"],
);

const OBSERVABILITY_TIMEOUT: OptionView = number(
    "request_timeout_seconds",
    "Timeout per request (seconds)",
    "Time allowed for each call, from 1 to 120.",
    "10",
    "10",
);

const fn observability_port(default: &'static str) -> OptionView {
    number("port", "Port", "Used if the address does not give a port.", default, default)
}

const OBSERVABILITY_TLS: OptionView = insecure_tls(
    "For a reverse proxy with a self-signed or private certificate: enable this if the connection is refused for that reason.",
);

/// Options lues par `collectors/observability/options.rs`, une liste par port.
const VICTORIAMETRICS_OPTIONS: &[OptionView] =
    &[OBSERVABILITY_SCHEME, observability_port("8428"), OBSERVABILITY_TLS, OBSERVABILITY_TIMEOUT];
const VICTORIALOGS_OPTIONS: &[OptionView] =
    &[OBSERVABILITY_SCHEME, observability_port("9428"), OBSERVABILITY_TLS, OBSERVABILITY_TIMEOUT];
const LOKI_OPTIONS: &[OptionView] =
    &[OBSERVABILITY_SCHEME, observability_port("3100"), OBSERVABILITY_TLS, OBSERVABILITY_TIMEOUT];
const GRAYLOG_OPTIONS: &[OptionView] =
    &[OBSERVABILITY_SCHEME, observability_port("9000"), OBSERVABILITY_TLS, OBSERVABILITY_TIMEOUT];

/// Utilisateur ACL de Redis 6 et suivants, ou de Valkey (`collectors/redis`).
const REDIS_LOGIN: CredentialView = CredentialView {
    kind: "username_password",
    label: "ACL user (recommended)",
    help: "A Redis user allowed to run INFO and PING only. Sent as AUTH user password.",
    fields: &[
        cred_text("username", "User name", "", "dumbmonit"),
        cred_secret("password", "Password", "Stored encrypted, never shown again.", "", true),
    ],
};

/// Mot de passe seul (`requirepass`), porté par un jeton : `AUTH motdepasse`.
const REDIS_PASSWORD: CredentialView = CredentialView {
    kind: "api_token",
    label: "Password only",
    help: "For Redis 5 or older, or a server with only requirepass set. Sent as AUTH password.",
    fields: &[cred_secret(
        "token",
        "Password",
        "The requirepass value. Stored encrypted, never shown again.",
        "",
        true,
    )],
};

const MONGODB_LOGIN: CredentialView = CredentialView {
    kind: "username_password",
    label: "Username / password",
    help: "An account with the clusterMonitor role only, logged in with SCRAM-SHA-256.",
    fields: &[
        cred_text("username", "User name", "", "dumbmonit"),
        cred_secret("password", "Password", "Stored encrypted, never shown again.", "", true),
    ],
};

const RABBITMQ_LOGIN: CredentialView = CredentialView {
    kind: "username_password",
    label: "Username / password",
    help: "A user tagged monitoring, sent as HTTP basic authentication.",
    fields: &[
        cred_text("username", "User name", "", "dumbmonit"),
        cred_secret("password", "Password", "Stored encrypted, never shown again.", "", true),
    ],
};

/// Métriques seules : `/metrics` de CrowdSec n'a pas d'authentification.
const CROWDSEC_METRICS_ONLY: CredentialView = CredentialView {
    kind: "none",
    label: "Metrics only",
    help: "Reads /metrics, and /health of the Local API. Nothing to create.",
    fields: &[],
};

/// Clé de bouncer, envoyée en `X-Api-Key` à la LAPI (`collectors/crowdsec`).
const CROWDSEC_BOUNCER_KEY: CredentialView = CredentialView {
    kind: "api_token",
    label: "Bouncer API key",
    help: "Also proves the Local API answers decision queries. A bouncer key can only read decisions.",
    fields: &[cred_secret(
        "token",
        "Bouncer API key",
        "Shown once by cscli bouncers add. Stored encrypted, never shown again.",
        "",
        true,
    )],
};

/// Délai commun aux intégrations Redis et MongoDB (`collectors/socket.rs`).
const SOCKET_TIMEOUT: OptionView = number(
    "request_timeout_seconds",
    "Timeout (seconds)",
    "Time allowed for the whole check: connection, login and commands, from 1 to 120.",
    "10",
    "10",
);

const fn socket_port(default: &'static str) -> OptionView {
    number("port", "Port", "Used if the address does not give a port.", default, default)
}

const SOCKET_TLS: OptionView =
    boolean("tls", "TLS", "Tick when the server only accepts TLS connections on that port.", false);

const SOCKET_INSECURE_TLS: OptionView = insecure_tls(
    "For a certificate from a private authority or a self-signed one: enable this if the connection is refused for that reason.",
);

/// Options lues par `collectors/redis/mod.rs` et `collectors/socket.rs`.
const REDIS_OPTIONS: &[OptionView] =
    &[socket_port("6379"), SOCKET_TLS, SOCKET_INSECURE_TLS, SOCKET_TIMEOUT];

/// Options lues par `collectors/mongodb/mod.rs` et `collectors/socket.rs`.
const MONGODB_OPTIONS: &[OptionView] = &[
    socket_port("27017"),
    text(
        "auth_source",
        "Authentication database",
        "The database the account was created in.",
        "admin",
        "admin",
    ),
    SOCKET_TLS,
    SOCKET_INSECURE_TLS,
    SOCKET_TIMEOUT,
];

/// Options lues par `collectors/rabbitmq/mod.rs` et `observability/options.rs`.
const RABBITMQ_OPTIONS: &[OptionView] = &[
    select(
        "scheme",
        "Protocol",
        "The management API listens over plain HTTP unless you configured TLS for it (usually on port 15671).",
        "http",
        &["http", "https"],
    ),
    observability_port("15672"),
    OBSERVABILITY_TLS,
    OBSERVABILITY_TIMEOUT,
    number(
        "max_queues",
        "Queues watched one by one",
        "The fullest queues are charted one by one, up to this number (0 to 500). Totals always cover every queue.",
        "100",
        "100",
    ),
];

/// Options lues par `collectors/crowdsec/mod.rs` et `observability/options.rs`.
const CROWDSEC_OPTIONS: &[OptionView] = &[
    OBSERVABILITY_SCHEME,
    number("port", "Metrics port", "The prometheus listen_port of CrowdSec.", "6060", "6060"),
    boolean(
        "lapi",
        "Check the Local API",
        "Checks /health of the Local API on this host. Untick for a host that only runs an agent.",
        true,
    ),
    number(
        "lapi_port",
        "Local API port",
        "The port of listen_uri in CrowdSec's api.server section.",
        "8080",
        "8080",
    ),
    OBSERVABILITY_TLS,
    OBSERVABILITY_TIMEOUT,
];

pub async fn list(State(state): State<AppState>) -> Json<Vec<CollectorView>> {
    Json(state.collectors.kinds().into_iter().map(describe).collect())
}

fn describe(kind: &'static str) -> CollectorView {
    match kind {
        "snmp" => CollectorView {
            kind,
            label: "SNMP device",
            summary: "Switches, routers, UPS, printers, access points: anything that answers SNMP.",
            examples: &["Switch", "Router", "NAS", "UPS", "Printer"],
            credential_types: &["snmp_community", "snmp_v3"],
            credentials: &[SNMP_COMMUNITY, SNMP_V3],
            address_hint: "192.168.1.10",
            default_port: 161,
            setup: Setup {
                title: "Enable SNMP on the device",
                steps: &[
                    "Open the device's administration interface.",
                    "Look for the SNMP section, often under \"Network\", \"Services\" or \"Administration\".",
                    "Enable SNMP v2c and note the read-only community (\"public\" by default on many devices).",
                    "If the device filters by address, allow this server's address.",
                    "Come back here, enter the address and the community: the rest is detected automatically.",
                ],
                warning: "A community is not encrypted on the network. On a shared network, prefer SNMP v3, which authenticates and encrypts.",
                doc_url: "",
            },
            options: &[],
        },
        "proxmox" => CollectorView {
            kind,
            label: "Proxmox VE",
            summary: "Nodes, virtual machines, containers, storage and backups of a Proxmox server or cluster.",
            examples: &["Proxmox server", "Proxmox cluster"],
            credential_types: &["api_token", "username_password"],
            credentials: &[PROXMOX_TOKEN, PROXMOX_LOGIN],
            address_hint: "192.168.1.20",
            default_port: 8006,
            setup: Setup {
                title: "Create a read-only user and token in Proxmox VE",
                steps: &[
                    "Open a shell on any node (in the web UI: select the node, then Shell; or SSH) and create a user reserved for monitoring. It needs no password: the token is what logs in.\npveum user add dumbmonit@pve --comment \"DumbMonit monitoring\"",
                    "Create a role with only the privileges the collector uses: Sys.Audit (nodes, cluster, HA, disks and SMART, ZFS, certificates, package versions, subscription), Datastore.Audit (storages and backup archives), VM.Audit (VMs, containers, snapshots) and VM.GuestAgent.Audit (what the QEMU guest agent reads inside a VM: disk usage, operating system, IP addresses). None of them can change anything.\npveum role add DumbMonit --privs \"Datastore.Audit Sys.Audit VM.Audit VM.GuestAgent.Audit\"",
                    "Give the user that role on the whole cluster.\npveum aclmod / -user dumbmonit@pve -role DumbMonit",
                    "Create the user's API token. Privilege separation is off, so the token simply inherits the user's rights.\npveum user token add dumbmonit@pve monitor --privsep 0",
                    "The command prints a table with full-tokenid and value. Copy full-tokenid into DumbMonit's Token ID field and value (the UUID) into its Secret field. The secret is shown once: if it is lost, remove the token and create a new one.\ndumbmonit@pve!monitor",
                    "Optional, to count pending updates and pending security fixes: Proxmox guards that list with Sys.Modify. Grant it on /nodes only; without it the collector skips the list silently.\npveum role add DumbMonitUpdates --privs Sys.Modify\npveum aclmod /nodes -user dumbmonit@pve -role DumbMonitUpdates",
                    "On an older Proxmox, VM.GuestAgent.Audit may not exist yet and the command above is refused with invalid privilege: the guest-agent calls are then covered by VM.Monitor, which Proxmox VE 9 removed in turn. Use whichever your version knows.\npveum role add DumbMonit --privs \"Datastore.Audit Sys.Audit VM.Audit VM.Monitor\"",
                    "Prefer the web UI? The same steps live under Datacenter → Permissions: Users, Roles, Add → User Permission, then API Tokens with \"Privilege Separation\" unticked.",
                    "In DumbMonit, enter the address of any node (port 8006 by default).",
                ],
                warning: "Do not reuse the account you log in with: a leaked token would then control the whole cluster. The DumbMonit role above can only read. Proxmox also uses a self-signed certificate by default: if the connection is refused for that reason, tick \"Accept an unverifiable certificate\" in the options.",
                doc_url: "https://pve.proxmox.com/wiki/User_Management",
            },
            options: PROXMOX_OPTIONS,
        },
        "pbs" => CollectorView {
            kind,
            label: "Proxmox Backup Server",
            summary: "Backup freshness per machine, failed tasks, sync and verify jobs, datastore space.",
            examples: &["Proxmox Backup Server"],
            credential_types: &["api_token", "username_password"],
            credentials: &[PBS_TOKEN, PBS_LOGIN],
            address_hint: "pbs.lan",
            default_port: 8007,
            setup: Setup {
                title: "Create a read-only user and token in Proxmox Backup Server",
                steps: &[
                    "Open a shell on the backup server (in the web UI: Administration → Shell; or SSH) and create a user reserved for monitoring. It needs no password: the token is what logs in.\nproxmox-backup-manager user create dumbmonit@pbs --comment \"DumbMonit monitoring\"",
                    "Create the user's API token. The command prints the token id and its secret: copy both now, PBS never shows the secret again.\nproxmox-backup-manager user generate-token dumbmonit@pbs monitor",
                    "Give the read-only minimum, per path — to the token and to the user that owns it. A token's effective privileges are the intersection of its own ACL and its user's: granted to the token alone, they amount to nothing at all, and PBS reports that nowhere. Every line is therefore written twice. DatastoreAudit on /datastore (Datastore.Audit) reads the datastores, snapshots, verify and prune jobs and GC; Audit on /system (Sys.Audit) reads the node status, the task list and task logs, the services, the traffic-control rules, the disks and ZFS pools.\nproxmox-backup-manager acl update /datastore DatastoreAudit --auth-id 'dumbmonit@pbs'\nproxmox-backup-manager acl update /datastore DatastoreAudit --auth-id 'dumbmonit@pbs!monitor'\nproxmox-backup-manager acl update /system Audit --auth-id 'dumbmonit@pbs'\nproxmox-backup-manager acl update /system Audit --auth-id 'dumbmonit@pbs!monitor'",
                    "Optional, and twice as well. RemoteAudit on /remote (Remote.Audit) shows the sync jobs that pull from a remote: the built-in Audit role covers Sys.Audit and Datastore.Audit only, and without Remote.Audit such a job is not refused, it is simply absent from the job list. TapeAudit on /tape (Tape.Audit) reads the tape tier. Audit on / (Sys.Audit at the top level) lists pending package updates, and also covers /datastore and /system if you prefer fewer lines. Without them those items are silently skipped, nothing else changes. Nothing here can write: the collector only performs GETs.\nproxmox-backup-manager acl update /remote RemoteAudit --auth-id 'dumbmonit@pbs'\nproxmox-backup-manager acl update /remote RemoteAudit --auth-id 'dumbmonit@pbs!monitor'\nproxmox-backup-manager acl update /tape TapeAudit --auth-id 'dumbmonit@pbs'\nproxmox-backup-manager acl update /tape TapeAudit --auth-id 'dumbmonit@pbs!monitor'\nproxmox-backup-manager acl update / Audit --auth-id 'dumbmonit@pbs'\nproxmox-backup-manager acl update / Audit --auth-id 'dumbmonit@pbs!monitor'",
                    "Copy the token id into DumbMonit's Token ID field and the secret into its Secret field.\ndumbmonit@pbs!monitor",
                    "Before leaving the shell, check what the token can actually read. The command prints its effective privileges, path by path; an empty result means the user is missing the ACL the token has — the case where the device looks perfectly alive and reports nothing.\nproxmox-backup-manager user permissions 'dumbmonit@pbs!monitor'",
                    "Prefer the web UI? Configuration → Access Control: Users → Add, then API Tokens → Add, then Permissions → Add. Add each permission twice, once as User Permission for dumbmonit@pbs and once as API Token Permission for dumbmonit@pbs!monitor: with path /datastore and role DatastoreAudit, then again with path /system and role Audit. A token permission on its own grants nothing.",
                    "In DumbMonit, enter the server address, for example \"pbs.lan\" or \"pbs.lan:8007\".",
                ],
                warning: "Do not reuse the account you log in with: a leaked token would then manage every backup. The Audit role can only read. PBS also uses a self-signed certificate by default: if the connection is refused for that reason, tick \"Accept an unverifiable certificate\" in the options.",
                doc_url: "https://pbs.proxmox.com/docs/user-management.html#api-tokens",
            },
            options: PBS_OPTIONS,
        },
        "pdm" => CollectorView {
            kind,
            label: "Proxmox Datacenter Manager",
            summary: "The console over several clusters: which instances it reaches, their failed tasks, its health.",
            examples: &["Proxmox Datacenter Manager"],
            credential_types: &["api_token"],
            credentials: &[PDM_TOKEN],
            address_hint: "dc.lan",
            default_port: 8443,
            setup: Setup {
                title: "Create a read-only user and token in Proxmox Datacenter Manager",
                steps: &[
                    "In the console, open Configuration → Access Control → Users and click Add. Name the account as follows and leave the password empty: the token is what logs in.\ndumbmonit@pdm",
                    "Still under Access Control, open API Tokens → Add, pick that user and name the token \"monitor\". The console shows the secret once: copy it now.\ndumbmonit@pdm!monitor",
                    "Give both the user and the token the Auditor role on / (the top of the tree, so the whole estate). Auditor carries System.Audit, Resource.Audit and Access.Audit, and can change nothing. A token never has more rights than its user, so the permission is granted twice: Permissions → Add → User Permission, then again with API Token Permission.",
                    "Copy the token id into DumbMonit's Token ID field and the secret into its Secret field.",
                    "In DumbMonit, enter the console address, for example \"dc.lan\" or \"dc.lan:8443\".",
                ],
                warning: "Do not reuse the account you log in with: a leaked token would then control every cluster the console federates at once. The Auditor role can only read. The console also uses a self-signed certificate by default: if the connection is refused for that reason, tick \"Accept an unverifiable certificate\" in the options. Two items need more than Auditor and are simply skipped without it: the per-instance metric collection status, and the update summary of the federated instances.",
                doc_url: "https://pve.proxmox.com/wiki/Proxmox_Datacenter_Manager_Roadmap",
            },
            options: PDM_OPTIONS,
        },
        "pmg" => CollectorView {
            kind,
            label: "Proxmox Mail Gateway",
            summary: "Mail queues and stuck mail, spam and virus traffic, quarantine, signature age.",
            examples: &["Proxmox Mail Gateway"],
            credential_types: &["username_password", "api_token"],
            credentials: &[PMG_LOGIN, PMG_TOKEN],
            address_hint: "mail.lan",
            default_port: 8006,
            setup: Setup {
                title: "Create a read-only user in Proxmox Mail Gateway",
                steps: &[
                    "In the web interface: Configuration → User Management → Users → Add. Name the account as follows, give it a long password used nowhere else, and tick Enabled.\ndumbmonit@pmg",
                    "Set its Role to Audit. That is the exact read-only minimum for everything DumbMonit reads: node status, services, postfix queues, mail statistics, quarantine counts, ClamAV and SpamAssassin database age, cluster status, certificates, subscription and pending updates. Audit can change nothing, release nothing from quarantine and read no message.",
                    "Prefer a shell? The same account in one command, then set the password.\npmgsh create /access/users --userid dumbmonit@pmg --role audit --enable 1 --comment \"DumbMonit monitoring\"",
                    "In DumbMonit, enter the gateway address, for example \"mail.lan\" or \"mail.lan:8006\", then this account's user name (realm included) and password.",
                    "Proxmox Mail Gateway does not issue API tokens, unlike Proxmox VE and Proxmox Backup Server: the username and password are the way in. DumbMonit opens one two-hour session and renews it, rather than logging in on every measurement.",
                ],
                warning: "Do not reuse the account you log in with: the Audit role above can only read, and cannot release a quarantined message or change a rule. Proxmox Mail Gateway also uses a self-signed certificate by default: if the connection is refused for that reason, tick \"Accept an unverifiable certificate\" in the options.",
                doc_url: "https://pmg.proxmox.com/pmg-docs/pmg-admin-guide.html#pmgconfig_userman",
            },
            options: PMG_OPTIONS,
        },
        "redfish" => CollectorView {
            kind,
            label: "Server hardware (Redfish)",
            summary: "Fans, temperatures, power supplies, drives and memory, read from the BMC (iDRAC, iLO…).",
            examples: &[
                "Supermicro BMC",
                "Dell iDRAC",
                "HPE iLO",
                "Lenovo XClarity Controller",
                "ASRock Rack BMC",
            ],
            credential_types: &["username_password"],
            credentials: &[REDFISH_LOGIN],
            address_hint: "bmc.lan",
            default_port: 443,
            setup: Setup {
                title: "Create a read-only account on the management controller",
                steps: &[
                    "Open the web interface of the management controller (BMC), not the operating system of the server. Create a new local user named as follows, with a long password used nowhere else.\ndumbmonit",
                    "Give it the lowest role that can read, and nothing more. Supermicro: Configuration → Users → Add User, privilege User, and tick Redfish in the account type. Dell iDRAC: iDRAC Settings → Users → Local Users → Add, role Read Only. HPE iLO: Administration → User Administration → New, untick every privilege except Login. Lenovo XClarity Controller: BMC Configuration → User/LDAP → Create, role Read-only. ASRock Rack: Settings → User Management, privilege User.",
                    "Check from any machine on the management network that the account can read Redfish. The command asks for the password and prints the server's name and health.\ncurl -k -u dumbmonit https://bmc.lan/redfish/v1/Systems",
                    "In DumbMonit, enter the controller address, for example \"bmc.lan\" or \"10.0.0.50\", then the user name and password of that account.",
                    "DumbMonit only reads: fans, temperatures, voltages, power supplies, drives, memory and processor summaries, and how many log entries there are by severity. It never powers the server on or off, never changes a setting and never reads the text of the logs.",
                ],
                warning: "Do not reuse the factory account of the controller: it can power the server off, mount media and reflash the firmware. Management controllers also ship with a self-signed certificate: if the connection is refused for that reason, tick \"Accept an unverifiable certificate\" in the options. A controller answers slowly and a full read takes several requests: if the device reports timeouts, raise DUMBMONIT_PROBE_TIMEOUT_SECS on the server.",
                doc_url: "https://www.dmtf.org/standards/redfish",
            },
            options: REDFISH_OPTIONS,
        },
        "mdaemon" => CollectorView {
            kind,
            label: "MDaemon Email Server",
            summary: "SMTP, IMAP, POP3 and webmail answering, version, and the XML API.",
            examples: &["MDaemon Email Server"],
            credential_types: &["none", "username_password"],
            credentials: &[MAIL_SERVICES_ONLY, MDAEMON_LOGIN],
            address_hint: "mail.example.com",
            default_port: 25,
            setup: Setup {
                title: "Watch the mail services, and optionally the XML API",
                steps: &[
                    "Nothing to install on the server. With no credential, DumbMonit connects to the mail ports from the outside and reads each greeting: SMTP (25), IMAP (143) and Webmail (3000) by default. Add POP3, MSA, the TLS ports or Remote Administration in the Services option, as a list such as this one.\nsmtp,msa,imap,pop3,imaps,webmail",
                    "To also read the version through the XML API, create a dedicated account in MDaemon, named for example as follows, with a long password used nowhere else. Give it the lowest level your MDaemon accepts for the XML API, and nothing more: DumbMonit only calls GetVersionInfo, which reads no mailbox and changes nothing.\ndumbmonit@example.com",
                    "Since MDaemon 24, the XML API only answers the addresses it allows. Open Setup → XML API Service → Address Restrictions (Setup → XML API Management on MDaemon 26) and allow the address DumbMonit connects from.",
                    "Check from the DumbMonit host that the account reaches the API. The command asks for the password and prints an answer that starts with <MDaemon><API productversion=…>.\ncurl -k -u dumbmonit@example.com -H 'Content-Type: text/xml' --data '<MDaemon><API><Request version=\"20.0.0\" echo=\"0\" verbose=\"0\"><Operation>GetVersionInfo</Operation><Parameters/></Request></API></MDaemon>' https://mail.example.com:444/MdMgmtWS/",
                    "In DumbMonit, enter the server address, for example \"mail.example.com\", then the full email address of the account as user name, and its password. The XML API is reached on the Remote Administration HTTPS port, 444 by default.",
                ],
                warning: "MDaemon has no read-only role for the XML API: never reuse your own account, and keep this password out of any other tool. Dynamic Screening can block an address after repeated failed logins, so a wrong password here can get the DumbMonit host blocked: test with the command above first. Remote Administration often uses a self-signed certificate: if the connection is refused for that reason, tick \"Accept an unverifiable certificate\" in the options. Mail queue sizes are not read by this device: MDaemon publishes them only as Windows performance counters, which the DumbMonit agent reads when it is installed on the mail server.",
                doc_url: "https://help.mdaemon.com/mdaemon/en/xml-api-service.html",
            },
            options: MDAEMON_OPTIONS,
        },
        "securitygateway" => CollectorView {
            kind,
            label: "SecurityGateway for Email Servers",
            summary: "Mail gateway services answering, version, and the REST API performance counters (12.5 and later).",
            examples: &["SecurityGateway for Email Servers"],
            credential_types: &["none", "api_token"],
            credentials: &[MAIL_SERVICES_ONLY, SECURITY_GATEWAY_KEY],
            address_hint: "sg.example.com",
            default_port: 25,
            setup: Setup {
                title: "Watch the gateway services, and optionally the REST API",
                steps: &[
                    "Nothing to install on the server. With no credential, DumbMonit connects to SMTP (25) and the web interface (4000) from the outside and reads the SMTP greeting. Change the list in the Services option, for example to check HTTPS instead of HTTP.\nsmtp,web_https",
                    "To also read the performance counters, SecurityGateway 12.5 or later is needed. Create a dedicated account named as follows, with a long password used nowhere else, and give it the Domain Administrator role. If the counters stay empty on the device page, your version keeps them for the Global Administrator role, and the account needs that role instead.\ndumbmonit",
                    "Sign in as that account and create a key under Setup/Users → Accounts → API Keys, with an expiry date. A key carries the rights of the account that creates it, which is why it must be created from this account and not from yours.",
                    "Restrict the account to the address DumbMonit connects from in its IP restrictions: API keys obey them too.",
                    "Check from the DumbMonit host that the key works. The command prints the API description, which also lists the counters DumbMonit reads.\ncurl -k -H 'Authorization: Bearer YOUR_KEY' https://sg.example.com:4443/api/v1/openapi",
                    "In DumbMonit, enter the gateway address, for example \"sg.example.com\", and paste the key. The API is reached on the web interface HTTPS port, 4443 by default, under /api/v1.",
                ],
                warning: "SecurityGateway has no read-only role and no narrower key: the key can do everything its account can. Never create it from your own account, keep it only in DumbMonit and give it an expiry date. Versions before 12.5 have no REST API: leave the credential empty and DumbMonit checks the services only. The web interface often uses a self-signed certificate on 4443: if the connection is refused for that reason, tick \"Accept an unverifiable certificate\" in the options.",
                doc_url: "https://help.mdaemon.com/SecurityGateway/en/api_keys.html",
            },
            options: SECURITY_GATEWAY_OPTIONS,
        },
        "truenas" => CollectorView {
            kind,
            label: "TrueNAS",
            summary: "Pool health, the failing disk, scrubs, snapshots, replication and TrueNAS's own alerts.",
            examples: &["TrueNAS SCALE", "TrueNAS Community Edition"],
            credential_types: &["api_token"],
            credentials: &[TRUENAS_KEY],
            address_hint: "nas.lan",
            default_port: 443,
            setup: Setup {
                title: "Create an API key for DumbMonit in TrueNAS",
                steps: &[
                    "In the web interface: Credentials → Groups → Add. Name the group as follows and give it the Local Administrator privilege.\ndumbmonit",
                    "Why Local Administrator and not Read-Only Administrator: TrueNAS wired its read-only roles into its WebSocket API only. Over the REST API DumbMonit uses, a read-only key is refused on every single call. DumbMonit itself only ever reads: it never starts a scrub, a SMART test, an update or a replication.",
                    "Credentials → Users → Add. Name the account as follows, make dumbmonit its primary group, and leave shell access, sudo and SSH off: none of them is needed.\ndumbmonit",
                    "Create the key: Credentials → Users → API Keys → Add, pick the dumbmonit user and give the key a name. TrueNAS shows the key only once: copy it. On TrueNAS 24.10 the path is the user menu at the top right → API Keys → Add, and a key belongs to no user.",
                    "In DumbMonit, enter the NAS address, for example \"nas.lan\", and paste the key. Keep HTTPS: TrueNAS revokes a key it ever receives over plain HTTP from another machine.",
                ],
                warning: "This key can change anything on the NAS: TrueNAS offers no read-only key over its REST API. Treat it like the password of an account that can erase your pools, keep it for DumbMonit alone, and revoke it from the same page if it leaks. TrueNAS also ships with a self-signed certificate: if the connection is refused for that reason, tick \"Accept an unverifiable certificate\" in the options.",
                doc_url: "https://www.truenas.com/docs/scale/25.04/scaletutorials/toptoolbar/managingapikeys/",
            },
            options: TRUENAS_OPTIONS,
        },
        "opnsense" => CollectorView {
            kind,
            label: "OPNsense",
            summary: "Gateway latency and loss, VPN tunnels, DHCP leases, services and pending updates.",
            examples: &["OPNsense firewall", "Home router", "Multi-WAN edge"],
            credential_types: &["username_password"],
            credentials: &[OPNSENSE_KEY],
            address_hint: "192.168.1.1",
            default_port: 443,
            setup: Setup {
                title: "Create a read-only API key in OPNsense",
                steps: &[
                    "In the web interface: System → Access → Groups → Add. Name the group as follows.\ndumbmonit",
                    "Tick these privileges in the group, exactly as OPNsense names them: Lobby: Dashboard (system, memory, disks, temperatures, state table), System: Gateways, Status: Interfaces, Status: Services, System: Firmware, Interfaces: Virtual IPs: Status (CARP) and Services: Unbound (MVC). Then one per feature you run: Services: DHCP: Kea(v4) or Services: Dnsmasq DNS/DHCP: Settings for the leases, VPN: WireGuard: Status, Status: OpenVPN, Status: IPsec. A privilege you leave out costs exactly the metrics it carries, nothing else.",
                    "System → Access → Users → Add. Name the account as follows, let OPNsense generate a scrambled password (this account never logs in: it answers with its key), and make it a member of the dumbmonit group.\ndumbmonit",
                    "Edit that user again and, under API keys, click +. OPNsense downloads a small text file with two lines, key= and secret=, and shows the secret only this once. Copy the key into DumbMonit's API key field and the secret into its API secret field.",
                    "In DumbMonit, enter the firewall address, for example \"192.168.1.1\" or \"fw.lan:8443\". The firewall is never asked to check for updates: DumbMonit reads the result of the check OPNsense runs on its own schedule, or that you start from System → Firmware.",
                ],
                warning: "Do not reuse the account you log in with: an API key is a password that never expires. OPNsense has no read-only variant of some privileges above: Status: Services also allows starting and stopping a service, System: Firmware also allows starting an update, and the Unbound, Kea and Dnsmasq privileges also cover their settings. DumbMonit only ever reads, but to limit what a leaked key could do, restrict the group's source networks to the address of the DumbMonit host. Never grant All pages. OPNsense also ships with a self-signed certificate: if the connection is refused for that reason, tick \"Accept an unverifiable certificate\" in the options.",
                doc_url: "https://docs.opnsense.org/development/how-tos/api.html",
            },
            options: OPNSENSE_OPTIONS,
        },
        "synology" => CollectorView {
            kind,
            label: "Synology DSM",
            summary: "Volumes, pools, disk health, temperature, Hyper Backup and Active Backup jobs.",
            examples: &["DiskStation", "RackStation"],
            credential_types: &["username_password"],
            credentials: &[SYNOLOGY_LOGIN],
            address_hint: "192.168.1.30",
            default_port: 5001,
            setup: Setup {
                title: "Create a monitoring account in DSM",
                steps: &[
                    "In DSM, open Control Panel → User & Group → User and click Create. Name the account as follows and give it a long password that is used nowhere else.\ndumbmonit",
                    "Join groups: tick administrators. DSM only answers the storage, volume and disk SMART calls to that group; without it the NAS shows as alive but says nothing about its disks. The next two steps take back everything else.",
                    "Assign shared folder permissions: No access on every shared folder. Assign application permissions: Deny everything except DSM, plus Active Backup for Business if you want its tasks read. Skip the quota and speed limit pages.",
                    "Two-step verification must stay off for this account: no automated monitor can type a one-time code. If Control Panel → Security → Account enforces it, restrict the rule to groups this user is not in, or exempt it.",
                    "Backup packages are read only while they are installed and running: DSM does not advertise a stopped package's API at all, so Hyper Backup tasks then go quiet instead of failing, and a NAS that only receives backups (Hyper Backup Vault) has no tasks of its own to show. Active Backup for Business additionally answers only an account allowed to use it (Active Backup for Business → Settings → Privileges); otherwise untick DumbMonit's \"Watch Active Backup for Business\" option to stop asking.",
                    "In DumbMonit, enter the NAS address (HTTPS, port 5001 by default), then this account's user name and password.",
                ],
                warning: "The administrators group is required by DSM's storage API, not by DumbMonit. That is why this account gets no shared folder, no application and a password used nowhere else: it can read the NAS, not touch your files.",
                doc_url: "https://kb.synology.com/en-global/DSM/help/DSM/AdminCenter/file_user_create",
            },
            options: SYNOLOGY_OPTIONS,
        },
        "agent" => CollectorView {
            kind,
            label: "Server with agent",
            summary: "Linux, Windows, macOS or FreeBSD: CPU, memory, disks, network, Docker, services, backups.",
            examples: &["Linux server", "Windows server", "Raspberry Pi"],
            credential_types: &["none"],
            credentials: &[NO_AUTH],
            address_hint: "detected automatically",
            default_port: 0,
            setup: Setup {
                title: "Install the agent on the machine",
                steps: &[
                    "Save the device in DumbMonit: an enrollment token (dmon_…) is shown once, with the install command ready to copy for Linux and for Windows.",
                    "That token is the agent's key to push its measurements to DumbMonit. It is not an account on the machine: nothing to create there. A token created from the device form is single use: it enrols this one machine and nothing else. For a fleet, create a reusable token in Settings → Agents.",
                    "Run the install command on the machine to monitor, with elevated rights (sudo on Linux, an elevated PowerShell on Windows). It downloads the agent, writes the token into agent.yaml and starts the service.",
                    "The machine shows up on its own within a few seconds, named after its host name. On that first batch the server gives the agent a secret of its own and stores it in /etc/dumbmonit/agent-secret, readable by nobody else: from then on, that machine is the only one that can report under this device. Lost the token? Settings → Agents lets you revoke it and create another.",
                    "Docker: to see the containers and let DumbMonit restart or update them, the agent must reach the Docker socket. The service the installer registers already can; if you run the agent under a dedicated user instead, add that user to the \"docker\" group and restart it. Restart and auto-update policies are then set per container on the device page.\nusermod -aG docker dumbmonit",
                    "Plakar backups: detected automatically — the Backups panel appears when the plakar binary or a kloset (~/.config/plakar/stores.yml of every user, ~/.plakar, /var/lib/plakar) is found, and nothing is shown otherwise. Set \"plakar_klosets\" in agent.yaml to watch a fixed list, or \"plakar: false\" to opt out.",
                ],
                warning: "The agent contacts the server, never the other way round: no port needs to be opened on the monitored machine.",
                doc_url: "",
            },
            options: &[],
        },
        // Les cinq sondes de disponibilité surveillent des services, pas des
        // équipements : leur notice explique ce qu'on saisit dans l'adresse, et
        // rappelle que l'état se lit sur « le service répond » et non sur « la sonde
        // a tourné ».
        "http" => CollectorView {
            kind,
            label: "Website or web API (HTTP)",
            summary: "A page or API answers with the right status and content, over a valid certificate.",
            examples: &[
                "Website",
                "Application health page",
                "REST API",
                "Self-hosted service interface",
            ],
            credential_types: &["none", "username_password", "api_token"],
            credentials: &[NO_AUTH, HTTP_LOGIN, HTTP_TOKEN],
            address_hint: "https://example.com/health",
            default_port: 443,
            setup: Setup {
                title: "Monitor a web page",
                steps: &[
                    "In the address, paste the full URL of the page to monitor. Without \"http://\" or \"https://\", HTTPS is assumed.",
                    "Prefer a light page that needs no login, for example the application's health page (\"/health\", \"/status\"), rather than the home page.",
                    "If the page requires authentication, fill in the credential: a username and password give basic authentication, a token is sent as \"Bearer\".",
                    "By default, any status code between 200 and 299 is fine. To go further, require a keyword in the page or a specific value in a JSON response.",
                    "Over HTTPS, the certificate is read automatically: you will be warned before it expires.",
                ],
                warning: "Never write a password or a token in the options: they are copied in clear text on every measurement. Use the credential field, which is encrypted.",
                doc_url: "",
            },
            options: HTTP_OPTIONS,
        },
        "tcp" => CollectorView {
            kind,
            label: "Network port (TCP)",
            summary: "A port accepts connections: SSH, a file share, a game server, anything without its own probe.",
            examples: &["SSH", "SMB or NFS share", "Database", "Game server", "Network printer"],
            credential_types: &["none"],
            credentials: &[NO_AUTH],
            address_hint: "nas.home.lan:22",
            default_port: 0,
            setup: Setup {
                title: "Monitor a port",
                steps: &[
                    "In the address, write the host followed by the port, separated by a colon: \"nas.home.lan:22\" or \"192.168.1.5:445\".",
                    "For an IPv6 address, put it in brackets: \"[fd00::1]:445\".",
                    "You can also leave the address without a port and enter it in the \"Port\" option: one of the two is required.",
                    "Only the connection opening is tested: no data is sent to the service, so it has no effect on it.",
                ],
                warning: "An open port does not prove the application behind it works. For a web service, prefer the \"Website or web API\" type, which reads the response.",
                doc_url: "",
            },
            options: TCP_OPTIONS,
        },
        "dns" => CollectorView {
            kind,
            label: "Domain name (DNS)",
            summary: "A name resolves, and points to the address you expect.",
            examples: &[
                "Your domain name",
                "An internal name served by your Pi-hole or AdGuard",
                "An MX record",
            ],
            credential_types: &["none"],
            credentials: &[NO_AUTH],
            address_hint: "www.example.com",
            default_port: 53,
            setup: Setup {
                title: "Monitor a DNS resolution",
                steps: &[
                    "In the address, write the name to resolve, without \"http://\": \"www.example.com\".",
                    "By default, the check asks the system resolver for an IPv4 address (A record). Pick another record type in the options if needed.",
                    "To monitor your own DNS server, enter its IP address in \"DNS server to query\": the check will fail if it stops answering.",
                    "To detect hijacking or a misconfiguration, list in \"Expected values\" the addresses the answer must contain.",
                ],
                warning: "The DNS server is given by its IP address, not by a name: it would take a resolver to resolve the resolver.",
                doc_url: "",
            },
            options: DNS_OPTIONS,
        },
        "ping" => CollectorView {
            kind,
            label: "Reachable host (ping)",
            summary: "A host answers ICMP: response time and packet loss.",
            examples: &[
                "Home router or gateway",
                "Wi-Fi access point",
                "Printer",
                "Machine without agent or SNMP",
                "Remote host",
            ],
            credential_types: &["none"],
            credentials: &[NO_AUTH],
            address_hint: "192.168.1.1",
            default_port: 0,
            setup: Setup {
                title: "Monitor a host with ping",
                steps: &[
                    "In the address, write the host name or IP address: \"192.168.1.1\" or \"router.home.lan\".",
                    "On each poll, the check sends four echoes and measures the response time, along with the share of lost packets.",
                    "By default, only a total loss counts as down; partial loss stays visible in the charts. Lower \"Tolerated loss\" to be warned earlier.",
                    "If DumbMonit runs in Docker, add the NET_RAW capability to the container: in docker-compose.yml, uncomment the \"cap_add: - NET_RAW\" lines under the dumbmonit service, then restart it.",
                ],
                warning: "Without the NET_RAW capability, the check cannot open an ICMP socket: it reports this as a configuration error, not as a host failure. Some devices also ignore pings on purpose: check that before drawing conclusions.",
                doc_url: "",
            },
            options: PING_OPTIONS,
        },
        "tls" => CollectorView {
            kind,
            label: "TLS certificate",
            summary: "A certificate is valid on any TLS port, with a warning well before it expires.",
            examples: &[
                "Mail server (IMAPS, SMTPS)",
                "Reverse proxy",
                "LDAPS directory",
                "MQTT broker",
                "Administration interface",
            ],
            credential_types: &["none"],
            credentials: &[NO_AUTH],
            address_hint: "mail.example.com:993",
            default_port: 443,
            setup: Setup {
                title: "Monitor a certificate",
                steps: &[
                    "In the address, write the host and, if it is not 443, the port of the encrypted service: \"mail.example.com:993\", \"ldap.home.lan:636\".",
                    "The check opens an encrypted connection, reads the presented certificate and closes right away: no data is exchanged with the application.",
                    "It records the number of days before expiry, the issuer and the TLS version. The bundled alert rules warn at fourteen days, then at expiry.",
                    "If the service is reached by its IP address but the certificate carries a name, enter that name in \"Server name (SNI)\".",
                    "For a certificate issued by your own authority, tick \"Accept an unverifiable certificate\": the expiry date is still monitored.",
                ],
                warning: "For a website, the \"Website or web API\" type already reads the certificate: this type is meant for services that do not speak HTTP, or whose application you do not want to hit.",
                doc_url: "",
            },
            options: TLS_OPTIONS,
        },
        "smtp" => CollectorView {
            kind,
            label: "Mail relay (SMTP)",
            summary: "A mail server greets, offers STARTTLS and accepts a login, like a real client.",
            examples: &[
                "Your provider's SMTP relay",
                "A local Postfix or msmtp",
                "Proxmox Mail Gateway",
                "The submission port of a mail server",
            ],
            credential_types: &["none", "username_password"],
            credentials: &[NO_AUTH, APP_LOGIN],
            address_hint: "smtp.example.com",
            default_port: 587,
            setup: Setup {
                title: "Monitor a mail relay",
                steps: &[
                    "In the address, write the mail server, and the port if it is not the usual one: \"smtp.example.com\" or \"smtp.example.com:2525\".",
                    "Pick the encryption your relay uses: STARTTLS for the submission port 587, TLS for port 465, None for a relay on your own network listening on port 25.",
                    "To check that the relay still accepts your account, fill in the credential: the check then runs AUTH and reports a refused password as such, not as an outage.",
                    "Nothing is ever sent: the check stops after the greeting, the extensions and the optional login, then hangs up. No message enters the queue.",
                    "To be warned when an extension disappears, name it in \"Expected extension\": a relay that stops advertising STARTTLS is a relay that would send your mail in clear text.",
                ],
                warning: "An accepted session does not prove mail leaves. A relay that accepts everything and queues it forever answers perfectly here: watch the queues themselves for that.",
                doc_url: "",
            },
            options: SMTP_OPTIONS,
        },
        "postgres" => CollectorView {
            kind,
            label: "PostgreSQL database",
            summary: "Logs in and runs a query: catches a database that is up but no longer answering.",
            examples: &[
                "The database behind Nextcloud or Immich",
                "A Home Assistant recorder",
                "An application database",
                "A TimescaleDB instance",
            ],
            credential_types: &["username_password"],
            credentials: &[SQL_LOGIN],
            address_hint: "db.home.lan",
            default_port: 5432,
            setup: Setup {
                title: "Monitor a PostgreSQL database",
                steps: &[
                    "On the server, create an account for monitoring and give it nothing more than the right to connect: CREATE ROLE dumbmonit LOGIN PASSWORD 'a-long-password';",
                    "Make sure that account may reach the server from the DumbMonit machine, which usually means one more line in pg_hba.conf followed by a configuration reload.",
                    "In the address, write the server, and the port if it is not 5432: \"db.home.lan\" or \"db.home.lan:5433\".",
                    "The default query is SELECT 1, which reads no table: nothing else has to be granted. Connection time and query time are recorded separately.",
                    "To watch something of your own, replace the query with one that returns a single number, and it becomes a chart: a queue length, a row count, a replication lag.",
                ],
                warning: "The check opens a real connection on every poll. On an instance already close to its connection limit, give it a longer interval.",
                doc_url: "",
            },
            options: POSTGRES_OPTIONS,
        },
        "mysql" => CollectorView {
            kind,
            label: "MySQL or MariaDB database",
            summary: "Logs in and runs a query: catches a database that is up but no longer answering.",
            examples: &[
                "The database behind a WordPress",
                "A Nextcloud or a Kimai",
                "An application database",
                "A MariaDB in Docker",
            ],
            credential_types: &["username_password"],
            credentials: &[SQL_LOGIN],
            address_hint: "db.home.lan",
            default_port: 3306,
            setup: Setup {
                title: "Monitor a MySQL or MariaDB database",
                steps: &[
                    "On the server, create an account for monitoring with no privilege at all: CREATE USER 'dumbmonit'@'%' IDENTIFIED BY 'a-long-password';",
                    "Check that the account may connect from the DumbMonit machine: a host pattern of localhost would only accept connections made on the server itself.",
                    "In the address, write the server, and the port if it is not 3306: \"db.home.lan\" or \"db.home.lan:3307\".",
                    "The default query is SELECT 1, which reads no table: nothing else has to be granted. Connection time and query time are recorded separately.",
                    "To watch something of your own, replace the query with one that returns a single number, and it becomes a chart: a queue length, a row count, a replication lag.",
                ],
                warning: "The check opens a real connection on every poll. On an instance already close to its connection limit, give it a longer interval.",
                doc_url: "",
            },
            options: MYSQL_OPTIONS,
        },
        "mqtt" => CollectorView {
            kind,
            label: "MQTT broker",
            summary: "Connects, subscribes to a topic, and can wait for a retained message.",
            examples: &[
                "Mosquitto",
                "The broker behind Home Assistant",
                "Zigbee2MQTT",
                "ESPHome sensors",
            ],
            credential_types: &["none", "username_password"],
            credentials: &[NO_AUTH, APP_LOGIN],
            address_hint: "broker.home.lan",
            default_port: 1883,
            setup: Setup {
                title: "Monitor an MQTT broker",
                steps: &[
                    "In the address, write the broker, and the port if it is not the usual one: \"broker.home.lan\" or \"broker.home.lan:1884\".",
                    "Tick \"Encrypted connection\" for a broker listening on 8883, and fill in the credential if it requires an account.",
                    "Connecting is already worth monitoring: a broker that refuses your account, or that has stopped answering, is the reason the whole house went quiet.",
                    "To go further, name a topic: the check then subscribes to it, and a broker that refuses the subscription tells you the account lost its access rights.",
                    "Tick \"Expect a retained message\" only for a topic that carries one, typically an availability topic. A topic published to now and then looks silent to a client that has just connected.",
                ],
                warning: "The check never publishes anything. It also cannot tell how old a retained message is: MQTT does not date them, so a value frozen three weeks ago still counts as present.",
                doc_url: "",
            },
            options: MQTT_OPTIONS,
        },
        "websocket" => CollectorView {
            kind,
            label: "WebSocket endpoint",
            summary: "Completes the upgrade handshake, and can exchange a frame.",
            examples: &[
                "The Home Assistant API",
                "A live dashboard",
                "A log stream",
                "An endpoint behind a reverse proxy",
            ],
            credential_types: &["none", "username_password", "api_token"],
            credentials: &[NO_AUTH, HTTP_LOGIN, HTTP_TOKEN],
            address_hint: "wss://home.example.com/api/websocket",
            default_port: 443,
            setup: Setup {
                title: "Monitor a WebSocket endpoint",
                steps: &[
                    "In the address, paste the full endpoint: \"wss://home.example.com/api/websocket\". Without a scheme, wss is assumed.",
                    "The check runs the whole opening handshake and verifies the answer the server computes from the key it was sent: a reverse proxy that has lost the Upgrade header is caught here, where an HTTP check would still see a healthy 200.",
                    "Nothing else is needed for most endpoints. A status other than 101 is reported with its code, so a 401 from an expired token is not mistaken for an outage.",
                    "To go further, fill in \"Expected content\" with a fragment of the first frame the server sends by itself, for example the greeting of the Home Assistant API.",
                    "If the endpoint says nothing until it is spoken to, put a message in \"Frame to send\" and what its answer must contain in \"Expected content\".",
                ],
                warning: "The check opens and hangs up within a second. A connection that a firewall or a proxy timeout cuts after thirty seconds looks perfectly healthy here.",
                doc_url: "",
            },
            options: WEBSOCKET_OPTIONS,
        },
        // Moniteur en poussée : rien n'est interrogé, c'est le travail surveillé
        // qui appelle. L'adresse n'est qu'un libellé : elle doit rester unique
        // parmi les heartbeats, comme toute adresse pour un type donné.
        "push" => CollectorView {
            kind,
            label: "Heartbeat (push)",
            summary: "Your cron job or backup script calls DumbMonit; when it goes quiet, you are told.",
            examples: &[
                "Nightly backup script",
                "Cron job",
                "Home Assistant automation",
                "Certificate renewal",
                "Database dump",
            ],
            credential_types: &["none"],
            credentials: &[NO_AUTH],
            address_hint: "nightly-backup",
            default_port: 0,
            setup: Setup {
                title: "Make the job call in",
                steps: &[
                    "In the address, write a short label for the job (\"nightly-backup\", \"certbot-renew\"): nothing is contacted, the label only has to be unique among your heartbeats.",
                    "Set the expected interval to the job's schedule (\"24h\" for a nightly job, \"1h\" for an hourly one) and, if the schedule drifts, a wider grace period. Save the device.",
                    "The device page shows the URL to call. Add it at the end of the job, so it is called only when the job succeeded:\ncurl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token>",
                    "A job that can tell when it failed may say so instead of staying silent: append ?status=down&msg=… to the URL, and the alert fires at once.",
                    "Until the first call arrives the device shows \"Waiting\" and nothing is alerted. Lost or leaked URL? \"Regenerate\" on the device page issues a new one; the old one stops answering immediately.",
                ],
                warning: "Call the URL at the end of the job, after the part that matters. A call placed at the top would report a success even when the backup itself failed.",
                doc_url: "",
            },
            options: PUSH_OPTIONS,
        },
        "dummy" => CollectorView {
            kind,
            label: "Demo device",
            summary: "Made-up measurements, to explore DumbMonit without any hardware.",
            examples: &["No hardware required"],
            credential_types: &["none"],
            credentials: &[NO_AUTH],
            address_hint: "demo",
            default_port: 0,
            setup: Setup {
                title: "Nothing to prepare",
                steps: &[
                    "Give it any name and confirm.",
                    "Fake measurements will appear right away, enough to browse the interface.",
                ],
                warning: "",
                doc_url: "",
            },
            options: &[],
        },
        "victoriametrics" => CollectorView {
            kind,
            label: "VictoriaMetrics",
            summary: "The time series database itself: ingestion, samples it refuses, disk headroom before it turns read-only, slow inserts and active series.",
            examples: &["VictoriaMetrics single-node", "Prometheus long-term storage"],
            credential_types: &["none", "username_password", "api_token"],
            credentials: &[NO_AUTH, HTTP_LOGIN, HTTP_TOKEN],
            address_hint: "victoriametrics.lan",
            default_port: 8428,
            setup: Setup {
                title: "Let DumbMonit read VictoriaMetrics' own health",
                steps: &[
                    "Nothing to install: VictoriaMetrics publishes its own health on /health and /metrics, on the port of its API (8428 by default). Check from the DumbMonit host that both answer.\ncurl http://victoriametrics.lan:8428/health\ncurl -s http://victoriametrics.lan:8428/metrics | grep vm_app_version",
                    "If VictoriaMetrics runs with -httpAuth.username and -httpAuth.password, pick Username / password below and enter those. Behind vmauth or a reverse proxy that expects a bearer token, pick Bearer token. If -metricsAuthKey is set, /metrics only answers with that key in the URL, which DumbMonit does not send: protect /metrics with the basic authentication above instead.",
                    "In DumbMonit, enter the server address, for example \"victoriametrics.lan\" or \"http://10.0.0.5:8428\". Behind a reverse proxy, enter the full URL with its path prefix. This covers the single-node server; the components of a VictoriaMetrics cluster expose other metrics and are not covered.",
                    "DumbMonit only reads /health and /metrics. It never runs a query against your data and never writes anything.",
                ],
                warning: "Free disk space is what to watch: below -storage.minFreeDiskSpaceBytes (100 MB by default) VictoriaMetrics switches to read-only and refuses every new sample, while it keeps answering queries as if nothing happened. DumbMonit shows the headroom left above that limit and warns under 10%.",
                doc_url: "https://docs.victoriametrics.com/victoriametrics/single-server-victoriametrics/#monitoring",
            },
            options: VICTORIAMETRICS_OPTIONS,
        },
        "victorialogs" => CollectorView {
            kind,
            label: "VictoriaLogs",
            summary: "The log database itself: lines ingested, lines it refuses, disk headroom before it turns read-only, errors.",
            examples: &["VictoriaLogs single-node"],
            credential_types: &["none", "username_password", "api_token"],
            credentials: &[NO_AUTH, HTTP_LOGIN, HTTP_TOKEN],
            address_hint: "victorialogs.lan",
            default_port: 9428,
            setup: Setup {
                title: "Let DumbMonit read VictoriaLogs' own health",
                steps: &[
                    "Nothing to install: VictoriaLogs publishes its own health on /health and /metrics, on the port of its API (9428 by default). Check from the DumbMonit host that both answer.\ncurl http://victorialogs.lan:9428/health\ncurl -s http://victorialogs.lan:9428/metrics | grep vl_rows_ingested_total",
                    "If VictoriaLogs runs with -httpAuth.username and -httpAuth.password, pick Username / password below and enter those. Behind vmauth or a reverse proxy that expects a bearer token, pick Bearer token. If -metricsAuthKey is set, /metrics only answers with that key in the URL, which DumbMonit does not send: protect /metrics with the basic authentication above instead.",
                    "In DumbMonit, enter the server address, for example \"victorialogs.lan\" or \"http://10.0.0.6:9428\". Behind a reverse proxy, enter the full URL with its path prefix.",
                    "DumbMonit only reads /health and /metrics. It never runs a LogsQL query and never reads a log line.",
                ],
                warning: "Free disk space is what to watch: below -storage.minFreeDiskSpaceBytes (10 MB by default) VictoriaLogs switches to read-only and refuses every new log line. DumbMonit shows the headroom left above that limit and warns under 10%.",
                doc_url: "https://docs.victoriametrics.com/victorialogs/#monitoring",
            },
            options: VICTORIALOGS_OPTIONS,
        },
        "loki" => CollectorView {
            kind,
            label: "Grafana Loki",
            summary: "The log server itself: readiness, log lines it refuses and why, chunks that fail to reach storage, write-ahead log disk, request errors.",
            examples: &["Loki single binary", "Loki simple scalable"],
            credential_types: &["none", "username_password", "api_token"],
            credentials: &[NO_AUTH, HTTP_LOGIN, HTTP_TOKEN],
            address_hint: "loki.lan",
            default_port: 3100,
            setup: Setup {
                title: "Let DumbMonit read Loki's own health",
                steps: &[
                    "Nothing to install: Loki publishes /ready and /metrics on its HTTP port (3100 by default). Check from the DumbMonit host that both answer.\ncurl http://loki.lan:3100/ready\ncurl -s http://loki.lan:3100/metrics | grep loki_build_info",
                    "Loki has no login of its own. If it sits behind a reverse proxy that asks for a user name and password, pick Username / password below; for a bearer token, pick Bearer token. No tenant is needed: /ready and /metrics are not per tenant.",
                    "In DumbMonit, enter Loki's address, for example \"loki.lan\", or \"https://logs.lan/loki\" behind a reverse proxy. With Loki split into components (simple scalable or microservices), add at least the write path, where refused lines and flush failures are counted.",
                    "DumbMonit only reads /ready and /metrics. It never runs a LogQL query and never reads a log line.",
                ],
                warning: "Refused lines are the failure to watch: when Loki refuses a line (too old, over the ingestion rate or stream limit, line too long), the sender gets an error and usually drops the batch. Nothing shows in Grafana except a gap in the logs.",
                doc_url: "https://grafana.com/docs/loki/latest/operations/meta-monitoring/",
            },
            options: LOKI_OPTIONS,
        },
        "graylog" => CollectorView {
            kind,
            label: "Graylog",
            summary: "The log server and its search cluster: processing, journal backlog, buffers, throughput in and out, OpenSearch health, failed inputs and indexing failures.",
            examples: &["Graylog Open", "Graylog with OpenSearch"],
            credential_types: &["api_token", "username_password"],
            credentials: &[GRAYLOG_TOKEN, GRAYLOG_LOGIN],
            address_hint: "graylog.lan",
            default_port: 9000,
            setup: Setup {
                title: "Create a read-only user and token in Graylog",
                steps: &[
                    "In Graylog: System → Users and Teams → Create user. Name the user as follows, give it a long random password, and assign the Reader role only. Reader can read the node status, journal, buffers, throughput, inputs and search cluster health, and no message at all as long as no stream is shared with it.\ndumbmonit",
                    "Open that user's Edit tokens page, create a token named after DumbMonit and copy it now: Graylog shows it only once.",
                    "Optional: Graylog's own notifications (an input that failed to start, disk watermarks, a journal almost full) need one more permission, notifications:read, which no built-in role grants on its own. With an account allowed to manage roles, create a role holding only that permission and give it to the dumbmonit user. Without it, notifications are skipped and nothing else changes.\ncurl -u YOUR_USER -H 'X-Requested-By: cli' -H 'Content-Type: application/json' -X POST https://graylog.lan/api/roles -d '{\"name\":\"DumbMonit Notifications\",\"description\":\"Read system notifications\",\"permissions\":[\"notifications:read\"],\"read_only\":false}'\ncurl -u YOUR_USER -H 'X-Requested-By: cli' -X PUT 'https://graylog.lan/api/roles/DumbMonit%20Notifications/members/dumbmonit'",
                    "In DumbMonit, enter the address of the Graylog node, for example \"graylog.lan\" (port 9000) or \"https://graylog.lan\" behind a reverse proxy, and paste the token. In a Graylog cluster, add each node: journal, buffers and throughput are per node.",
                    "DumbMonit only reads. It never searches messages, never starts or stops an input and never acknowledges a notification.",
                ],
                warning: "Do not reuse the account you log in with: a leaked token carries every right of its user, while the Reader role above can change nothing. Graylog serves plain HTTP unless you configured TLS, and the token travels with every request: across an untrusted network, put Graylog behind HTTPS.",
                doc_url: "",
            },
            options: GRAYLOG_OPTIONS,
        },
        "redis" => CollectorView {
            kind,
            label: "Redis / Valkey",
            summary: "The in-memory store itself: memory against maxmemory, refused connections, replication link and lag, failed saves and keyspace size.",
            examples: &["Redis", "Valkey", "The cache behind Nextcloud, Immich or Authentik"],
            credential_types: &["username_password", "api_token", "none"],
            credentials: &[REDIS_LOGIN, REDIS_PASSWORD, NO_AUTH],
            address_hint: "cache.lan",
            default_port: 6379,
            setup: Setup {
                title: "Create a Redis user that may only run INFO",
                steps: &[
                    "On Redis 6 or later, or Valkey, create a user that may run INFO and PING and nothing else, with no access to any key. In redis-cli, logged in with a user allowed to manage ACLs:\nACL SETUSER dumbmonit on >a-long-password -@all +info +ping",
                    "Make it permanent: ACL SAVE if the server loads its users from an aclfile, CONFIG REWRITE otherwise.\nACL SAVE",
                    "On Redis 5 or older, or with only requirepass set, pick Password only below. That password carries every right: prefer an ACL user whenever the server supports one.",
                    "In DumbMonit, enter the server address, for example \"cache.lan\" or \"cache.lan:6380\". If the server only accepts TLS on that port (tls-port), tick TLS below.",
                    "DumbMonit sends AUTH, then INFO. It never reads, writes or lists a key.",
                ],
                warning: "Redis gives no warning before maxmemory: at the limit it either evicts keys silently or refuses every write, depending on maxmemory-policy. Without maxmemory it grows until the kernel kills it. DumbMonit warns above 90% of maxmemory.",
                doc_url: "https://redis.io/docs/latest/operate/oss_and_stack/management/security/acl/",
            },
            options: REDIS_OPTIONS,
        },
        "mongodb" => CollectorView {
            kind,
            label: "MongoDB",
            summary: "The document database itself: replica set members and lag, a missing primary, connections against the limit, WiredTiger cache and assertions.",
            examples: &[
                "MongoDB replica set",
                "Standalone mongod",
                "The database behind UniFi or Rocket.Chat",
            ],
            credential_types: &["username_password", "none"],
            credentials: &[MONGODB_LOGIN, NO_AUTH],
            address_hint: "db1.lan",
            default_port: 27017,
            setup: Setup {
                title: "Create a MongoDB account with the clusterMonitor role",
                steps: &[
                    "In mongosh, logged in with a user allowed to create users, create an account with the built-in clusterMonitor role only. It reads server and replication status, and no document of any database.\nuse admin\ndb.createUser({user: \"dumbmonit\", pwd: passwordPrompt(), roles: [{role: \"clusterMonitor\", db: \"admin\"}]})",
                    "DumbMonit logs in with SCRAM-SHA-256, the default for accounts created on MongoDB 4.0 and later. If you created the account in another database than admin, set Authentication database below.",
                    "In DumbMonit, enter the address of one mongod, for example \"db1.lan\" or \"db1.lan:27018\". In a replica set, add every member: each reports its own connections and cache, and any of them reports the state and lag of all members. If the server requires TLS, tick TLS below.",
                    "DumbMonit only runs serverStatus and replSetGetStatus. It never lists a collection and never reads a document.",
                ],
                warning: "A mongodb+srv:// address is not resolved: enter the host of each member instead. MongoDB older than 3.6 and mongos routers are not supported.",
                doc_url: "https://www.mongodb.com/docs/manual/reference/built-in-roles/#mongodb-authrole-clusterMonitor",
            },
            options: MONGODB_OPTIONS,
        },
        "rabbitmq" => CollectorView {
            kind,
            label: "RabbitMQ",
            summary: "The message broker: memory and disk alarms that block publishers, stopped nodes and partitions, queues filling up with no consumer.",
            examples: &["RabbitMQ single node", "RabbitMQ cluster"],
            credential_types: &["username_password"],
            credentials: &[RABBITMQ_LOGIN],
            address_hint: "rabbit.lan",
            default_port: 15672,
            setup: Setup {
                title: "Create a RabbitMQ user tagged monitoring",
                steps: &[
                    "Enable the management plugin if it is not already on (the management Docker images have it). Its API listens on port 15672.\nrabbitmq-plugins enable rabbitmq_management",
                    "Create a user with the monitoring tag only: it can read node, alarm and queue state, and cannot change anything.\nrabbitmqctl add_user dumbmonit 'a-long-password'\nrabbitmqctl set_user_tags dumbmonit monitoring",
                    "Give it empty permissions on each virtual host whose queues you want to see. It can then list those queues, and still cannot publish, consume or read a message. Repeat with -p for every other virtual host.\nrabbitmqctl set_permissions -p / dumbmonit '^$' '^$' '^$'",
                    "In DumbMonit, enter the address of one node, for example \"rabbit.lan\", or \"https://rabbit.lan:15671\" when the API is served over TLS. One node reports the whole cluster: the alarms and partitions of every node, and every queue.",
                ],
                warning: "A memory or disk alarm blocks every publisher in the cluster, and clients get no error: applications simply hang on publish. Queues are listed fullest first, up to Queues watched one by one (100 by default); totals always cover every queue.",
                doc_url: "https://www.rabbitmq.com/docs/management#permissions",
            },
            options: RABBITMQ_OPTIONS,
        },
        "crowdsec" => CollectorView {
            kind,
            label: "CrowdSec",
            summary: "The security engine: active decisions, alerts, bouncers that stopped pulling, log lines read and not parsed, Local API health.",
            examples: &[
                "CrowdSec on a Linux host",
                "CrowdSec in Docker",
                "CrowdSec on OPNsense or pfSense",
            ],
            credential_types: &["none", "api_token"],
            credentials: &[CROWDSEC_METRICS_ONLY, CROWDSEC_BOUNCER_KEY],
            address_hint: "crowdsec.lan",
            default_port: 6060,
            setup: Setup {
                title: "Let DumbMonit read CrowdSec's metrics",
                steps: &[
                    "CrowdSec publishes its metrics on 127.0.0.1:6060 by default. Set listen_addr in the prometheus section of /etc/crowdsec/config.yaml to an address DumbMonit can reach, then restart CrowdSec. The official Docker image already listens on every interface.\nsudo sed -i 's/listen_addr: 127.0.0.1/listen_addr: 0.0.0.0/' /etc/crowdsec/config.yaml\nsudo systemctl restart crowdsec",
                    "The metrics need no credential: they hold counts, bouncer and machine names and log file paths, never an IP address. Filter port 6060 so that only the DumbMonit host reaches it, and check that it answers.\ncurl -s http://crowdsec.lan:6060/metrics | grep cs_info",
                    "DumbMonit also checks /health of the Local API, on port 8080 of the same host: listen_uri in the api.server section must then be reachable too. Untick Check the Local API for a host that only runs an agent.",
                    "Optional: create a bouncer key for DumbMonit and paste it as Bouncer API key below. A bouncer key can only read decisions. DumbMonit asks for the decisions on 192.0.2.1, a documentation address, so the answer is always empty and the ban list is never downloaded.\nsudo cscli bouncers add dumbmonit",
                    "In DumbMonit, enter the CrowdSec host, for example \"crowdsec.lan\". Where the Local API and the agents run on different hosts, add each host: decisions, alerts and bouncers are counted where the Local API runs, log lines where each agent reads them.",
                ],
                warning: "A bouncer that stops pulling keeps enforcing a frozen list: new attackers get through and nothing complains. DumbMonit warns when a bouncer's requests stop for 30 minutes. It only knows bouncers that pulled at least once since CrowdSec last started.",
                doc_url: "https://docs.crowdsec.net/docs/next/observability/prometheus/",
            },
            options: CROWDSEC_OPTIONS,
        },
        other => CollectorView {
            kind,
            label: other,
            summary: "",
            examples: &[],
            credential_types: &["none"],
            credentials: &[NO_AUTH],
            address_hint: "",
            default_port: 0,
            setup: Setup { title: "", steps: &[], warning: "", doc_url: "" },
            options: &[],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les types enregistrés dans `main.rs`. Un type ajouté là-bas sans notice ici
    /// s'afficherait sous son nom brut, sans explication ni exemple d'adresse.
    const KINDS_ENREGISTRES: &[&str] = &[
        "snmp",
        "proxmox",
        "pbs",
        "pdm",
        "pmg",
        "synology",
        "opnsense",
        "truenas",
        "redfish",
        "victoriametrics",
        "victorialogs",
        "loki",
        "graylog",
        "mdaemon",
        "securitygateway",
        "redis",
        "mongodb",
        "rabbitmq",
        "crowdsec",
        "agent",
        "http",
        "tcp",
        "dns",
        "ping",
        "tls",
        "smtp",
        "postgres",
        "mysql",
        "mqtt",
        "websocket",
        "push",
        "dummy",
    ];

    #[test]
    fn chaque_type_enregistre_a_une_notice_qui_lui_est_propre() {
        for kind in KINDS_ENREGISTRES {
            let view = describe(kind);
            assert_eq!(view.kind, *kind);
            assert!(
                view.label != *kind || !view.summary.is_empty(),
                "« {kind} » n'a que la description générique"
            );
            assert!(!view.setup.title.is_empty(), "« {kind} » n'a pas de notice de mise en route");
            assert!(!view.setup.steps.is_empty(), "« {kind} » n'a aucune étape");
            assert!(!view.address_hint.is_empty(), "« {kind} » n'a pas d'adresse d'exemple");
            assert!(
                !view.credential_types.is_empty(),
                "« {kind} » n'annonce aucune forme d'identifiant"
            );
        }
    }

    #[test]
    fn un_type_inconnu_recoit_une_description_generique_mais_complete() {
        let view = describe("inconnu");
        assert_eq!(view.label, "inconnu");
        assert!(view.summary.is_empty());
        assert!(view.options.is_empty());
        assert_eq!(view.credential_types, &["none"]);
    }

    /// Les sondes lisent ces étiquettes dans leurs `options.rs` : l'interface ne
    /// doit pas proposer un champ que le collecteur ignorerait, ni en cacher un.
    #[test]
    fn les_options_des_sondes_reprennent_les_etiquettes_lues_par_les_collecteurs() {
        let attendues: &[(&str, &[&str])] = &[
            (
                "http",
                &[
                    "method",
                    "accepted_status",
                    "keyword",
                    "keyword_absent",
                    "keyword_case_sensitive",
                    "json_path",
                    "json_expect",
                    "headers",
                    "body",
                    "follow_redirects",
                    "max_redirects",
                    "insecure_tls",
                    "allow_private_targets",
                    "check_certificate",
                    "max_body_bytes",
                    "user_agent",
                    "timeout_seconds",
                ],
            ),
            ("tcp", &["port", "allow_private_targets", "timeout_seconds"]),
            (
                "dns",
                &["record_type", "resolver", "expect", "expect_mode", "forbid", "timeout_seconds"],
            ),
            (
                "ping",
                &[
                    "count",
                    "packet_timeout_ms",
                    "interval_ms",
                    "payload_bytes",
                    "ip_version",
                    "loss_threshold_percent",
                    "timeout_seconds",
                ],
            ),
            ("tls", &["server_name", "insecure_tls", "allow_private_targets", "timeout_seconds"]),
            (
                "smtp",
                &[
                    "security",
                    "port",
                    "helo_name",
                    "expect_capability",
                    "server_name",
                    "insecure_tls",
                    "allow_private_targets",
                    "timeout_seconds",
                ],
            ),
            (
                "postgres",
                &[
                    "port",
                    "database",
                    "query",
                    "expect",
                    "sslmode",
                    "allow_private_targets",
                    "timeout_seconds",
                ],
            ),
            (
                "mysql",
                &[
                    "port",
                    "database",
                    "query",
                    "expect",
                    "sslmode",
                    "allow_private_targets",
                    "timeout_seconds",
                ],
            ),
            (
                "mqtt",
                &[
                    "tls",
                    "port",
                    "topic",
                    "expect_message",
                    "expect",
                    "client_id",
                    "server_name",
                    "insecure_tls",
                    "allow_private_targets",
                    "timeout_seconds",
                ],
            ),
            (
                "websocket",
                &[
                    "path",
                    "port",
                    "send",
                    "expect",
                    "subprotocol",
                    "origin",
                    "server_name",
                    "insecure_tls",
                    "allow_private_targets",
                    "timeout_seconds",
                ],
            ),
            (
                "proxmox",
                &[
                    "port",
                    "insecure_tls",
                    "request_timeout_seconds",
                    "backup_lookback_days",
                    "scan_backup_storage",
                    "nodes",
                    "ha",
                    "backup_jobs",
                    "scan_snapshots",
                    "max_snapshot_guests",
                    "replication",
                    "ceph",
                    "updates",
                    "certificates",
                    "guest_agent",
                    "disks",
                    "zfs",
                    "packages",
                    "subscription",
                    "cluster_resources",
                    "services",
                    "network",
                    "lvm",
                    "ceph_detail",
                    "backup_volumes",
                    "guest_os",
                    "metrics_export",
                ],
            ),
            (
                "pbs",
                &[
                    "port",
                    "insecure_tls",
                    "request_timeout_seconds",
                    "task_lookback_hours",
                    "datastores",
                    "max_groups",
                    "jobs",
                    "updates",
                    "disks",
                    "services",
                    "datastore_details",
                    "traffic_control",
                    "certificates",
                    "tape",
                ],
            ),
            (
                "pdm",
                &[
                    "port",
                    "insecure_tls",
                    "request_timeout_seconds",
                    "node",
                    "task_lookback_hours",
                    "max_age_seconds",
                    "remotes",
                    "max_remotes",
                    "versions",
                    "tasks",
                    "node_status",
                    "updates",
                    "remote_updates",
                ],
            ),
            (
                "pmg",
                &[
                    "port",
                    "insecure_tls",
                    "request_timeout_seconds",
                    "node",
                    "recent_hours",
                    "queues",
                    "quarantine",
                    "attachment_quarantine",
                    "signatures",
                    "services",
                    "certificates",
                    "updates",
                    "subscription",
                ],
            ),
            ("synology", &["scheme", "port", "insecure_tls", "request_timeout_seconds", "abb"]),
            (
                "truenas",
                &[
                    "scheme",
                    "port",
                    "insecure_tls",
                    "request_timeout_seconds",
                    "datasets",
                    "disks",
                    "smart",
                    "alerts",
                    "tasks",
                    "services",
                ],
            ),
            (
                "opnsense",
                &[
                    "scheme",
                    "port",
                    "insecure_tls",
                    "request_timeout_seconds",
                    "gateways",
                    "interfaces",
                    "firewall",
                    "dhcp",
                    "vpn",
                    "unbound",
                    "services",
                    "carp",
                    "firmware",
                    "temperature",
                ],
            ),
            ("push", &["expected_interval", "grace"]),
            (
                "redfish",
                &["port", "insecure_tls", "request_timeout_seconds", "auth", "storage", "logs"],
            ),
            ("victoriametrics", &["scheme", "port", "insecure_tls", "request_timeout_seconds"]),
            ("victorialogs", &["scheme", "port", "insecure_tls", "request_timeout_seconds"]),
            ("loki", &["scheme", "port", "insecure_tls", "request_timeout_seconds"]),
            ("graylog", &["scheme", "port", "insecure_tls", "request_timeout_seconds"]),
            (
                "mdaemon",
                &["services", "request_timeout_seconds", "api_port", "api_tls", "insecure_tls"],
            ),
            (
                "securitygateway",
                &[
                    "services",
                    "request_timeout_seconds",
                    "api_port",
                    "api_tls",
                    "insecure_tls",
                    "counters",
                ],
            ),
            ("redis", &["port", "tls", "insecure_tls", "request_timeout_seconds"]),
            ("mongodb", &["port", "auth_source", "tls", "insecure_tls", "request_timeout_seconds"]),
            (
                "rabbitmq",
                &["scheme", "port", "insecure_tls", "request_timeout_seconds", "max_queues"],
            ),
            (
                "crowdsec",
                &["scheme", "port", "lapi", "lapi_port", "insecure_tls", "request_timeout_seconds"],
            ),
        ];
        for (kind, cles) in attendues {
            let obtenues: Vec<&str> = describe(kind).options.iter().map(|o| o.key).collect();
            assert_eq!(&obtenues, cles, "options de « {kind} »");
        }
        for kind in ["snmp", "agent", "dummy"] {
            assert!(describe(kind).options.is_empty(), "« {kind} » ne lit aucune étiquette");
        }
    }

    #[test]
    fn chaque_option_est_coherente_avec_son_type_de_champ() {
        for kind in KINDS_ENREGISTRES {
            for option in describe(kind).options {
                let contexte = format!("option « {} » de « {kind} »", option.key);
                assert!(!option.label.is_empty(), "{contexte} : sans libellé");
                assert!(!option.help.is_empty(), "{contexte} : sans aide");
                match option.input {
                    "select" => {
                        assert!(!option.choices.is_empty(), "{contexte} : liste sans choix");
                        assert!(
                            option.choices.contains(&option.default),
                            "{contexte} : le défaut « {} » n'est pas dans la liste",
                            option.default
                        );
                    }
                    "boolean" => assert!(
                        matches!(option.default, "true" | "false"),
                        "{contexte} : défaut booléen « {} »",
                        option.default
                    ),
                    "number" => assert!(
                        option.default.is_empty() || option.default.parse::<u64>().is_ok(),
                        "{contexte} : défaut numérique « {} »",
                        option.default
                    ),
                    "text" => assert!(option.choices.is_empty(), "{contexte} : choix sur un texte"),
                    other => panic!("{contexte} : type de champ inconnu « {other} »"),
                }
            }
        }
    }

    /// Les défauts affichés doivent être ceux que les collecteurs appliquent
    /// réellement : un défaut faux dans le formulaire est pire qu'aucun.
    #[test]
    fn les_defauts_affiches_sont_ceux_des_collecteurs() {
        let defaut = |kind: &'static str, cle: &str| {
            describe(kind).options.iter().find(|o| o.key == cle).map(|o| o.default).unwrap()
        };
        assert_eq!(defaut("http", "timeout_seconds"), "5");
        assert_eq!(defaut("http", "max_body_bytes"), "524288");
        assert_eq!(defaut("http", "max_redirects"), "10");
        assert_eq!(
            defaut("http", "user_agent"),
            format!("DumbMonit/{}", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(defaut("ping", "count"), "4");
        assert_eq!(defaut("ping", "packet_timeout_ms"), "1000");
        assert_eq!(defaut("proxmox", "port"), "8006");
        assert_eq!(defaut("proxmox", "backup_lookback_days"), "31");
        assert_eq!(defaut("pbs", "port"), "8007");
        assert_eq!(defaut("pbs", "request_timeout_seconds"), "15");
        assert_eq!(defaut("pbs", "task_lookback_hours"), "24");
        assert_eq!(defaut("pbs", "max_groups"), "500");
        assert_eq!(defaut("pdm", "port"), "8443");
        assert_eq!(defaut("pdm", "request_timeout_seconds"), "20");
        assert_eq!(defaut("pdm", "node"), "localhost");
        assert_eq!(defaut("pdm", "max_age_seconds"), "60");
        assert_eq!(defaut("pdm", "max_remotes"), "100");
        assert_eq!(defaut("pdm", "remote_updates"), "false");
        assert_eq!(defaut("pmg", "port"), "8006");
        assert_eq!(defaut("pmg", "request_timeout_seconds"), "15");
        assert_eq!(defaut("pmg", "recent_hours"), "12");
        assert_eq!(defaut("pmg", "attachment_quarantine"), "false");
        assert_eq!(defaut("synology", "request_timeout_seconds"), "15");
        assert_eq!(defaut("synology", "abb"), "true");
        assert_eq!(defaut("redfish", "port"), "443");
        assert_eq!(defaut("redfish", "request_timeout_seconds"), "8");
        assert_eq!(defaut("redfish", "auth"), "basic");
        // `collectors/observability/{options,victoria,loki,graylog}.rs`.
        assert_eq!(defaut("victoriametrics", "port"), "8428");
        assert_eq!(defaut("victorialogs", "port"), "9428");
        assert_eq!(defaut("loki", "port"), "3100");
        assert_eq!(defaut("graylog", "port"), "9000");
        for kind in ["victoriametrics", "victorialogs", "loki", "graylog"] {
            assert_eq!(defaut(kind, "scheme"), "http", "« {kind} »");
            assert_eq!(
                defaut(kind, "request_timeout_seconds"),
                dumbmonit_collectors::observability::DEFAULT_REQUEST_TIMEOUT.as_secs().to_string()
            );
        }
        // `collectors/{socket,redis,mongodb,rabbitmq,crowdsec}`.
        assert_eq!(defaut("redis", "port"), dumbmonit_collectors::redis::DEFAULT_PORT.to_string());
        assert_eq!(
            defaut("mongodb", "port"),
            dumbmonit_collectors::mongodb::DEFAULT_PORT.to_string()
        );
        assert_eq!(
            defaut("mongodb", "auth_source"),
            dumbmonit_collectors::mongodb::DEFAULT_AUTH_SOURCE
        );
        assert_eq!(
            defaut("rabbitmq", "port"),
            dumbmonit_collectors::rabbitmq::DEFAULT_PORT.to_string()
        );
        assert_eq!(
            defaut("rabbitmq", "max_queues"),
            dumbmonit_collectors::rabbitmq::DEFAULT_MAX_QUEUES.to_string()
        );
        assert_eq!(
            defaut("crowdsec", "port"),
            dumbmonit_collectors::crowdsec::DEFAULT_PORT.to_string()
        );
        assert_eq!(
            defaut("crowdsec", "lapi_port"),
            dumbmonit_collectors::crowdsec::DEFAULT_LAPI_PORT.to_string()
        );
        for kind in ["redis", "mongodb"] {
            assert_eq!(
                defaut(kind, "request_timeout_seconds"),
                dumbmonit_collectors::SOCKET_DEFAULT_TIMEOUT.as_secs().to_string()
            );
            assert_eq!(defaut(kind, "tls"), "false");
        }
        assert_eq!(
            defaut("push", "expected_interval"),
            crate::collectors::push::DEFAULT_EXPECTED_INTERVAL
        );
        assert_eq!(defaut("push", "grace"), crate::collectors::push::DEFAULT_GRACE);
    }

    /// `credential_types` et `credentials` décrivent la même liste : l'ancienne
    /// interface lit la première, la nouvelle la seconde.
    #[test]
    fn les_formes_didentifiant_sont_decrites_champ_par_champ() {
        for kind in KINDS_ENREGISTRES {
            let view = describe(kind);
            let kinds: Vec<&str> = view.credentials.iter().map(|c| c.kind).collect();
            assert_eq!(&kinds, view.credential_types, "« {kind} » : formes annoncées");
            for credential in view.credentials {
                assert!(!credential.label.is_empty(), "« {kind} » : forme sans libellé");
                let mut keys = std::collections::BTreeSet::new();
                for field in credential.fields {
                    let contexte = format!("champ « {} » de « {kind} »", field.key);
                    assert!(keys.insert(field.key), "{contexte} : déclaré deux fois");
                    assert!(!field.label.is_empty(), "{contexte} : sans libellé");
                    match field.input {
                        "select" => assert!(!field.choices.is_empty(), "{contexte} : sans choix"),
                        "text" | "password" => {
                            assert!(field.choices.is_empty(), "{contexte} : choix sur un texte")
                        }
                        other => panic!("{contexte} : type de champ inconnu « {other} »"),
                    }
                }
                if credential.kind == "none" {
                    assert!(credential.fields.is_empty(), "« {kind} » : « none » sans champ");
                } else {
                    assert!(!credential.fields.is_empty(), "« {kind} » : forme sans champ");
                }
            }
        }
    }

    /// Un jeton Proxmox se saisit en deux cases ; les clés sont celles que
    /// `dumbmonit_proto::Credential` sait recomposer.
    #[test]
    fn un_jeton_proxmox_se_saisit_en_deux_champs() {
        for kind in ["proxmox", "pbs", "pmg"] {
            let view = describe(kind);
            let token = view.credentials.iter().find(|c| c.kind == "api_token").unwrap();
            let keys: Vec<&str> = token.fields.iter().map(|f| f.key).collect();
            assert_eq!(keys, ["token_id", "secret"], "« {kind} »");
            assert!(token.fields[0].placeholder.contains('!'), "l'exemple montre user@realm!nom");
            assert_eq!(token.fields[1].input, "password");
            let credential: dumbmonit_proto::Credential =
                serde_json::from_value(serde_json::json!({
                    "type": "api_token",
                    "token_id": token.fields[0].placeholder,
                    "secret": "8f3a1c9e-0000-4444-8888-aaaabbbbcccc",
                }))
                .unwrap();
            assert_eq!(
                credential,
                dumbmonit_proto::Credential::ApiToken {
                    token: format!(
                        "{}=8f3a1c9e-0000-4444-8888-aaaabbbbcccc",
                        token.fields[0].placeholder
                    )
                }
            );
        }
    }

    /// Texte d'une notice, étapes et mise en garde comprises.
    fn notice(kind: &'static str) -> String {
        let view = describe(kind);
        let mut text = view.setup.steps.join("\n");
        text.push('\n');
        text.push_str(view.setup.warning);
        text
    }

    /// Les tutoriels font créer un compte réservé, jamais réutiliser le compte
    /// tout-puissant : le mot « root » ou « admin » n'a rien à y faire — sauf le
    /// nom du groupe `administrators` que l'API de stockage de DSM exige, et qui
    /// est justement expliqué.
    #[test]
    fn les_notices_font_creer_un_compte_dedie_et_ne_citent_jamais_le_compte_racine() {
        let attendus = [
            ("proxmox", "dumbmonit@pve"),
            ("pbs", "dumbmonit@pbs"),
            ("pdm", "dumbmonit@pdm"),
            ("pmg", "dumbmonit@pmg"),
            ("synology", "dumbmonit"),
            ("opnsense", "dumbmonit"),
            ("truenas", "dumbmonit"),
            ("redfish", "dumbmonit"),
            ("graylog", "dumbmonit"),
            ("mdaemon", "dumbmonit@example.com"),
            ("securitygateway", "dumbmonit"),
            ("redis", "dumbmonit"),
            ("mongodb", "dumbmonit"),
            ("rabbitmq", "dumbmonit"),
            ("crowdsec", "dumbmonit"),
            ("agent", "token"),
        ];
        for (kind, dedie) in attendus {
            let text = notice(kind);
            assert!(text.contains(dedie), "« {kind} » ne nomme pas le compte dédié « {dedie} »");
            // Exceptions nommées, qui sont des privilèges ou des bases et non
            // des comptes : le groupe `administrators` de DSM, le privilège
            // « Local Administrator » que l'API REST de TrueNAS exige, etc.
            let allowed = text
                .to_lowercase()
                .replace("administrators", "")
                .replace("local administrator", "")
                .replace("read-only administrator", "")
                .replace("global administrator", "")
                .replace("domain administrator", "")
                // MongoDB range ses comptes dans la base `admin` : c'est un nom
                // de base, pas un compte.
                .replace("use admin", "")
                .replace("db: \"admin\"", "")
                .replace("than admin", "");
            for word in allowed.split(|c: char| !c.is_alphanumeric()) {
                assert!(
                    !matches!(word, "root" | "admin" | "administrator"),
                    "la notice de « {kind} » cite « {word} » comme compte à utiliser"
                );
            }
        }
    }

    /// Les commandes des notices sont des lignes à copier telles quelles : pas
    /// d'espace autour, pas de guillemets typographiques qu'un shell refuserait.
    #[test]
    fn les_commandes_des_notices_sont_copiables_telles_quelles() {
        for kind in KINDS_ENREGISTRES {
            for step in describe(kind).setup.steps {
                let mut lines = step.lines();
                let text = lines.next().unwrap_or_default();
                assert!(!text.trim().is_empty(), "« {kind} » : étape sans texte");
                for command in lines {
                    assert_eq!(command, command.trim(), "« {kind} » : commande avec des espaces");
                    assert!(!command.is_empty(), "« {kind} » : ligne de commande vide");
                    assert!(
                        !command.contains(['“', '”', '‘', '’']),
                        "« {kind} » : guillemets typographiques dans « {command} »"
                    );
                }
            }
        }
    }

    /// Les pages `docs/devices/*.md` reprennent les notices mot pour mot : le
    /// tutoriel affiché dans l'application et celui de la documentation ne
    /// doivent pas diverger. La comparaison ignore les retours à la ligne et
    /// les accents de code Markdown.
    #[test]
    fn la_documentation_reprend_les_notices_mot_pour_mot() {
        let docs: &[(&str, &str)] = &[
            ("proxmox", include_str!("../../../../docs/devices/proxmox.md")),
            ("pbs", include_str!("../../../../docs/devices/pbs.md")),
            ("pdm", include_str!("../../../../docs/devices/pdm.md")),
            ("pmg", include_str!("../../../../docs/devices/pmg.md")),
            ("synology", include_str!("../../../../docs/devices/synology.md")),
            ("opnsense", include_str!("../../../../docs/devices/opnsense.md")),
            ("truenas", include_str!("../../../../docs/devices/truenas.md")),
            ("redfish", include_str!("../../../../docs/devices/redfish.md")),
            ("victoriametrics", include_str!("../../../../docs/devices/victoriametrics.md")),
            ("victorialogs", include_str!("../../../../docs/devices/victoriametrics.md")),
            ("loki", include_str!("../../../../docs/devices/loki.md")),
            ("graylog", include_str!("../../../../docs/devices/graylog.md")),
            ("mdaemon", include_str!("../../../../docs/devices/mdaemon.md")),
            ("securitygateway", include_str!("../../../../docs/devices/securitygateway.md")),
            ("redis", include_str!("../../../../docs/devices/redis.md")),
            ("mongodb", include_str!("../../../../docs/devices/mongodb.md")),
            ("rabbitmq", include_str!("../../../../docs/devices/rabbitmq.md")),
            ("crowdsec", include_str!("../../../../docs/devices/crowdsec.md")),
            ("agent", include_str!("../../../../docs/devices/agent.md")),
            ("push", include_str!("../../../../docs/devices/push.md")),
            ("smtp", include_str!("../../../../docs/devices/services.md")),
            ("postgres", include_str!("../../../../docs/devices/services.md")),
            ("mysql", include_str!("../../../../docs/devices/services.md")),
            ("mqtt", include_str!("../../../../docs/devices/services.md")),
            ("websocket", include_str!("../../../../docs/devices/services.md")),
        ];
        fn flatten(text: &str) -> String {
            text.replace('`', "").split_whitespace().collect::<Vec<_>>().join(" ")
        }
        for (kind, doc) in docs {
            let flat = flatten(doc);
            let view = describe(kind);
            for step in view.setup.steps {
                let mut lines = step.lines();
                let text = flatten(lines.next().unwrap_or_default());
                assert!(flat.contains(&text), "docs/devices/{kind}.md ne reprend pas : {text}");
                for command in lines {
                    assert!(
                        doc.contains(command),
                        "docs/devices/{kind}.md ne reprend pas : {command}"
                    );
                }
            }
            let warning = flatten(view.setup.warning);
            assert!(flat.contains(&warning), "docs/devices/{kind}.md ne reprend pas : {warning}");
        }
    }

    #[test]
    fn la_notice_du_ping_mentionne_la_capacite_reseau_necessaire() {
        let view = describe("ping");
        let texte = view.setup.steps.join(" ") + view.setup.warning;
        assert!(texte.contains("NET_RAW"), "l'utilisateur doit savoir quoi décommenter");
    }

    #[test]
    fn la_sonde_http_annonce_les_identifiants_quelle_sait_envoyer() {
        let view = describe("http");
        assert!(view.credential_types.contains(&"username_password"));
        assert!(view.credential_types.contains(&"api_token"));
        for kind in ["tcp", "dns", "ping", "tls", "push"] {
            assert_eq!(describe(kind).credential_types, &["none"], "« {kind} » n'envoie rien");
        }
    }

    /// Une base de données ne se surveille pas anonymement : le formulaire ne
    /// doit pas proposer « aucune authentification », qui échouerait à coup sûr.
    #[test]
    fn les_sondes_applicatives_annoncent_les_identifiants_quelles_savent_envoyer() {
        for kind in ["postgres", "mysql"] {
            assert_eq!(
                describe(kind).credential_types,
                &["username_password"],
                "« {kind} » exige un compte"
            );
        }
        for kind in ["smtp", "mqtt"] {
            assert_eq!(describe(kind).credential_types, &["none", "username_password"], "{kind}");
        }
        assert_eq!(
            describe("websocket").credential_types,
            &["none", "username_password", "api_token"]
        );
    }

    /// Les défauts des sondes applicatives sont ceux de leurs `options.rs` : un
    /// port faux dans le formulaire enverrait l'utilisateur chercher une panne
    /// qui n'existe pas.
    #[test]
    fn les_defauts_des_sondes_applicatives_sont_ceux_des_collecteurs() {
        let defaut = |kind: &'static str, cle: &str| {
            describe(kind).options.iter().find(|o| o.key == cle).map(|o| o.default).unwrap()
        };
        assert_eq!(defaut("smtp", "security"), "starttls");
        assert_eq!(defaut("smtp", "helo_name"), dumbmonit_collectors::uptime::SMTP_DEFAULT_HELO);
        assert_eq!(defaut("postgres", "port"), "5432");
        assert_eq!(defaut("postgres", "database"), "postgres");
        assert_eq!(defaut("mysql", "port"), "3306");
        assert_eq!(defaut("mysql", "database"), "", "MySQL n'exige pas qu'on nomme une base");
        for kind in ["postgres", "mysql"] {
            assert_eq!(
                defaut(kind, "query"),
                dumbmonit_collectors::uptime::SQL_DEFAULT_QUERY,
                "« {kind} » : la requête par défaut ne doit lire aucune table"
            );
            assert_eq!(defaut(kind, "sslmode"), "prefer");
        }
        assert_eq!(defaut("mqtt", "tls"), "false");
        assert_eq!(defaut("mqtt", "expect_message"), "false");
    }
}
