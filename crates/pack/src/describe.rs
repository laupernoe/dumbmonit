//! Description d'un paquet pour `GET /api/collectors`, au même format que les
//! types compilés : l'interface ne fait aucune différence entre les deux.

use dumbmonit_proto::{
    CredentialDescription, CredentialFieldDescription, KindDescription, OptionDescription,
    SetupDescription,
};

use crate::Pack;
use crate::manifest::{CredentialSpec, OptionSpec};

impl Pack {
    pub fn description(&self) -> KindDescription {
        let manifest = &self.manifest;
        let credentials: Vec<CredentialDescription> = if manifest.credentials.is_empty() {
            vec![credential("none", None, None)]
        } else {
            manifest
                .credentials
                .iter()
                .map(|spec| match spec {
                    CredentialSpec::Kind(kind) => credential(kind, None, None),
                    CredentialSpec::Detailed(detail) => {
                        credential(&detail.kind, detail.label.as_deref(), detail.help.as_deref())
                    }
                })
                .collect()
        };
        let mut options: Vec<OptionDescription> = manifest.options.iter().map(option).collect();
        if self.has_collector() {
            options.extend(self.connection_options());
        }
        KindDescription {
            kind: self.kind.clone().into(),
            label: manifest.label.clone().into(),
            summary: manifest.summary.clone().into(),
            examples: manifest.examples.iter().cloned().map(Into::into).collect(),
            credential_types: credentials.iter().map(|c| c.kind.clone()).collect(),
            credentials,
            address_hint: manifest.address_hint.clone().into(),
            default_port: manifest.default_port,
            setup: SetupDescription {
                title: manifest.setup.title.clone().into(),
                steps: manifest.setup.steps.iter().cloned().map(Into::into).collect(),
                warning: manifest.setup.warning.clone().into(),
                doc_url: manifest.setup.doc_url.clone().into(),
            },
            options,
        }
    }

    /// Réglages de connexion communs à tous les paquets HTTP, lus par
    /// `Connection::from_target` et le garde-fou d'adresses.
    fn connection_options(&self) -> Vec<OptionDescription> {
        let scheme = self.manifest.scheme.as_str();
        let port = match self.manifest.default_port {
            0 if scheme == "https" => "443".to_string(),
            0 => "80".to_string(),
            port => port.to_string(),
        };
        vec![
            OptionDescription {
                key: "scheme".into(),
                label: "Protocol".into(),
                help: "How DumbMonit talks to the device.".into(),
                default: scheme.into(),
                input: "select".into(),
                choices: vec!["http".into(), "https".into()],
                ..OptionDescription::default()
            },
            OptionDescription {
                key: "port".into(),
                label: "Port".into(),
                help: "Used if the address does not give a port.".into(),
                placeholder: port.clone().into(),
                default: port.into(),
                input: "number".into(),
                ..OptionDescription::default()
            },
            OptionDescription {
                key: "insecure_tls".into(),
                label: "Accept an unverifiable certificate".into(),
                help: "For a device with a self-signed certificate: enable this if the \
                       connection is refused for that reason."
                    .into(),
                default: "false".into(),
                input: "boolean".into(),
                ..OptionDescription::default()
            },
            OptionDescription {
                key: "request_timeout_seconds".into(),
                label: "Timeout per request (seconds)".into(),
                help: "Time allowed for each call, from 1 to 120.".into(),
                placeholder: "10".into(),
                default: "10".into(),
                input: "number".into(),
                ..OptionDescription::default()
            },
            OptionDescription {
                key: "allow_private_targets".into(),
                label: "Allow loopback and link-local targets".into(),
                help: "By default a pack refuses addresses only the DumbMonit host itself can \
                       reach (127.0.0.1, ::1, 169.254.x.x). Private LAN addresses (10.x, \
                       192.168.x) are always allowed."
                    .into(),
                default: "false".into(),
                input: "boolean".into(),
                ..OptionDescription::default()
            },
        ]
    }
}

fn option(spec: &OptionSpec) -> OptionDescription {
    OptionDescription {
        key: spec.key.clone().into(),
        label: spec.label.clone().into(),
        help: spec.help.clone().into(),
        placeholder: spec.placeholder.clone().into(),
        default: spec.default.0.clone().into(),
        required: spec.required,
        input: spec.input.clone().into(),
        choices: spec.choices.iter().cloned().map(Into::into).collect(),
    }
}

fn credential(kind: &str, label: Option<&str>, help: Option<&str>) -> CredentialDescription {
    let (default_label, default_help, fields) = match kind {
        "api_token" => (
            "API token",
            "Stored encrypted, sent only to this device.",
            vec![CredentialFieldDescription {
                key: "token".into(),
                label: "Token".into(),
                help: "Stored encrypted, never shown again.".into(),
                input: "password".into(),
                required: true,
                ..CredentialFieldDescription::default()
            }],
        ),
        "username_password" => (
            "Username / password",
            "Stored encrypted, sent only to this device.",
            vec![
                CredentialFieldDescription {
                    key: "username".into(),
                    label: "User name".into(),
                    input: "text".into(),
                    required: true,
                    ..CredentialFieldDescription::default()
                },
                CredentialFieldDescription {
                    key: "password".into(),
                    label: "Password".into(),
                    input: "password".into(),
                    required: true,
                    ..CredentialFieldDescription::default()
                },
            ],
        ),
        _ => (
            "No authentication",
            "Nothing is sent: the device answers without credentials.",
            vec![],
        ),
    };
    CredentialDescription {
        kind: kind.to_string().into(),
        label: label.unwrap_or(default_label).to_string().into(),
        help: help.unwrap_or(default_help).to_string().into(),
        fields,
    }
}
