//! Notice de entra (générée à partir de la spécification du produit).

use super::super::*;

pub(super) fn view() -> CollectorView {
    CollectorView {
        kind: "entra",
        label: "Microsoft Entra ID",
        summary: "Entra ID application secret and certificate expiry, on-premises directory sync age and administrators without MFA, read through Microsoft Graph.",
        examples: &["Contoso Entra ID"],
        credential_types: &["username_password"],
        credentials: &[CredentialView {
            kind: "username_password",
            label: "Entra application (client credentials)",
            help: "The application (client) ID and a client secret of an Entra app registration with read-only Graph application permissions.",
            fields: &[
                CredentialField {
                    key: "username",
                    label: "Application (client) ID",
                    help: "Shown on the Overview page of the app registration.",
                    placeholder: "00000000-0000-0000-0000-000000000000",
                    input: "text",
                    choices: &[],
                    required: true,
                },
                CredentialField {
                    key: "password",
                    label: "Client secret",
                    help: "The secret Value (not the Secret ID), shown once when the secret is created.",
                    placeholder: "",
                    input: "password",
                    choices: &[],
                    required: true,
                },
            ],
        }],
        address_hint: "graph.microsoft.com",
        default_port: 443,
        setup: Setup {
            title: "Create a read-only Entra application for DumbMonit",
            steps: &[
                "In the Microsoft Entra admin center (entra.microsoft.com), open Entra ID → App registrations → New registration, name it DumbMonit, keep a single tenant and register it.",
                "On the Overview page, copy the Application (client) ID and the Directory (tenant) ID.",
                "Open API permissions → Add a permission → Microsoft Graph → Application permissions, add the permissions listed below, then click Grant admin consent.",
                "Open Certificates & secrets → New client secret and copy its Value (not the Secret ID): it is shown once and it expires, so note the date.",
                "In DumbMonit, enter graph.microsoft.com as address, paste the client ID and the secret value, and fill in the Tenant ID option.",
            ],
            ports: &[
                to_device!(443, "tcp", "Microsoft Graph API (HTTPS, graph.microsoft.com)"),
                to_device!(
                    443,
                    "tcp",
                    "Microsoft sign-in, token request (HTTPS, login.microsoftonline.com)"
                ),
            ],
            permissions: &[
                "Application.Read.All\nReads app registrations: the expiry date of each secret and certificate, never their values.",
                "Organization.Read.All\nReads the tenant and its on-premises directory sync status.",
                "Reports.Read.All\nReads admin MFA registration; needs Entra ID P1 or P2 (optional).",
            ],
            warning: "Admin consent gives the application tenant-wide read access to the data above. Use a dedicated app registration with only these permissions, and renew its client secret before it expires: DumbMonit watches its own secret like any other.",
        },
        options: &[
            OptionView {
                key: "tenant_id",
                label: "Tenant ID",
                help: "Directory (tenant) ID of the app registration: a GUID, or a verified domain name.",
                placeholder: "00000000-0000-0000-0000-000000000000",
                default: "",
                required: true,
                input: "text",
                choices: &[],
                group: "",
                items: no_items,
                columns: no_columns,
            },
            OptionView {
                key: "login_url",
                label: "Sign-in URL",
                help: "Microsoft sign-in service. Change it only for a sovereign cloud, for example https://login.microsoftonline.us.",
                placeholder: "https://login.microsoftonline.com",
                default: "https://login.microsoftonline.com",
                required: false,
                input: "text",
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
