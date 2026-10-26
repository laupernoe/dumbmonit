//! Notice de ovh (générée à partir de la spécification du produit).

use super::super::*;

pub(super) fn view() -> CollectorView {
    CollectorView {
        kind: "ovh",
        label: "OVHcloud",
        summary: "State and running tasks of the VPS and dedicated servers of an OVHcloud account, read through the OVHcloud API.",
        examples: &["OVHcloud VPS and dedicated servers"],
        credential_types: &["username_password"],
        credentials: &[CredentialView {
            kind: "username_password",
            label: "OVHcloud API keys",
            help: "The application key, and the application secret and consumer key joined by a colon.",
            fields: &[
                CredentialField {
                    key: "username",
                    label: "Application key",
                    help: "The Application Key shown when the token was created.",
                    placeholder: "a1b2c3d4e5f6a7b8",
                    input: "text",
                    choices: &[],
                    required: true,
                },
                CredentialField {
                    key: "password",
                    label: "Application secret:Consumer key",
                    help: "The Application Secret and the Consumer Key joined by a colon, in this order: secret:consumer.",
                    placeholder: "application_secret:consumer_key",
                    input: "password",
                    choices: &[],
                    required: true,
                },
            ],
        }],
        address_hint: "eu.api.ovh.com",
        default_port: 443,
        setup: Setup {
            title: "Create a read-only API token for DumbMonit",
            steps: &[
                "Open the OVHcloud token page of your region and sign in with your OVHcloud account. The link pre-selects the two read rights.\nhttps://eu.api.ovh.com/createToken/?GET=/vps*&GET=/dedicated/server*",
                "Name the script DumbMonit, set Validity to Unlimited, and keep only the two GET rights listed below, then click Create keys.",
                "Copy the Application Key, the Application Secret and the Consumer Key: they are shown once.",
                "In DumbMonit, enter eu.api.ovh.com as address (ca.api.ovh.com for OVHcloud Canada) and paste the Application Key as Application key.",
                "In the second field, type the Application Secret, a colon, then the Consumer Key.\napplication_secret:consumer_key",
            ],
            ports: &[to_device!(443, "tcp", "OVHcloud API (HTTPS, eu.api.ovh.com)")],
            permissions: &[
                "GET /vps*\nLists the VPS and reads their state and tasks; changes nothing.",
                "GET /dedicated/server*\nLists the dedicated servers and reads their state and tasks; changes nothing.",
            ],
            warning: "The OVHcloud API publishes no calendar of planned maintenance: DumbMonit sees an intervention only once it has started, as a running task. A VPS you stopped yourself counts as down. A dedicated server is reported in error only if OVHcloud monitoring is enabled on it. Either right may be omitted to watch only the other family.",
        },
        options: &[OptionView {
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
        }],
    }
}
