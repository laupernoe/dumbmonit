//! Notice de hetzner (générée à partir de la spécification du produit).

use super::super::*;

pub(super) fn view() -> CollectorView {
    CollectorView {
        kind: "hetzner",
        label: "Hetzner Cloud",
        summary: "Servers, volumes and backups of a Hetzner Cloud project, read through the Hetzner Cloud API.",
        examples: &["Hetzner Cloud project"],
        credential_types: &["api_token"],
        credentials: &[CredentialView {
            kind: "api_token",
            label: "API token",
            help: "A read-only API token of the Hetzner Cloud project.",
            fields: &[CredentialField {
                key: "token",
                label: "API token",
                help: "The token created in the dashboard, shown once.",
                placeholder: "",
                input: "password",
                choices: &[],
                required: true,
            }],
        }],
        address_hint: "api.hetzner.cloud",
        default_port: 443,
        setup: Setup {
            title: "Create a read-only API token for DumbMonit",
            steps: &[
                "In the Hetzner Cloud Console, open the project to monitor, then Security > API tokens > Generate API token.",
                "Name it DumbMonit and choose the Read permission (not Read & Write).",
                "Click Generate API token, then copy the token: it is shown once.",
                "In DumbMonit, use this address and paste the token as API token. One monitor reads one project.\napi.hetzner.cloud",
            ],
            ports: &[to_device!(443, "tcp", "Hetzner Cloud API (HTTPS, api.hetzner.cloud)")],
            permissions: &[
                "Read (project token)\nLists servers, volumes and backups and reads server metrics; changes nothing.",
            ],
            warning: "A server that is powered off is reported as not running, on purpose or not: mute the rule for it if that is expected. The API allows 3600 requests per hour and project, so server metrics are read for 10 servers by default.",
        },
        options: &[
            OptionView {
                key: "metrics_servers",
                label: "Servers with metrics",
                help: "How many running servers get their CPU, disk and network read each minute, from 0 (none) to 100. Each one costs one API call.",
                placeholder: "10",
                default: "10",
                required: false,
                input: "number",
                choices: &[],
                group: "",
                items: no_items,
                columns: no_columns,
            },
            OptionView {
                key: "request_timeout_seconds",
                label: "Timeout per request (seconds)",
                help: "Time allowed for each call, from 1 to 120.",
                placeholder: "15",
                default: "15",
                required: false,
                input: "number",
                choices: &[],
                group: "",
                items: no_items,
                columns: no_columns,
            },
        ],
    }
}
