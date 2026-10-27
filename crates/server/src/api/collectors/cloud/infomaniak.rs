//! Notice de infomaniak (générée à partir de la spécification du produit).

use super::super::*;

pub(super) fn view() -> CollectorView {
    CollectorView {
        kind: "infomaniak",
        label: "Infomaniak",
        summary: "Expiry dates, locks and maintenance flags of the products of an Infomaniak account (domains, hosting, mail), read through the Infomaniak API.",
        examples: &["Infomaniak domains and hosting"],
        credential_types: &["api_token"],
        credentials: &[CredentialView {
            kind: "api_token",
            label: "API token",
            help: "An Infomaniak API token with the Products scope.",
            fields: &[CredentialField {
                key: "token",
                label: "API token",
                help: "The token created in the Infomaniak Manager.",
                placeholder: "",
                input: "password",
                choices: &[],
                required: true,
            }],
        }],
        address_hint: "api.infomaniak.com",
        default_port: 443,
        setup: Setup {
            title: "Create a read-only API token for DumbMonit",
            steps: &[
                "Open the Infomaniak Manager token page and sign in.\nhttps://manager.infomaniak.com/v3/ng/profile/user/token/list",
                "Click Create a token and name it DumbMonit.",
                "Choose the scopes listed below, and set the validity you want (no expiry avoids renewing it).",
                "Click Create the token, then copy it: it is shown once.",
                "In DumbMonit, enter api.infomaniak.com as address and paste the token. Fill the Account ID option if the token has no Accounts scope.",
            ],
            ports: &[to_device!(443, "tcp", "Infomaniak API (HTTPS, api.infomaniak.com)")],
            permissions: &[
                "Products (scope: product)\nLists the products of the account with their expiry date and flags; changes nothing.",
                "Accounts (scope: accounts)\nLists the account IDs of the token; optional when the Account ID option is filled.",
            ],
            warning: "The Infomaniak API gives no finer state than the expiry date, the lock and the maintenance flags of each product: it does not tell whether a site or a mailbox is up. Use an HTTP monitor for that.",
        },
        options: &[
            OptionView {
                key: "account_id",
                label: "Account ID",
                help: "The Infomaniak account to read, a number. Empty: every account the token can list (up to 10), which needs the Accounts scope.",
                placeholder: "123456",
                default: "",
                required: false,
                input: "text",
                choices: &[],
                group: "",
                items: no_items,
                columns: no_columns,
            },
            OptionView {
                key: "service_names",
                label: "Services",
                help: "Comma-separated service names to read, such as domain, hosting. Empty: every product of the account.",
                placeholder: "domain, hosting",
                default: "",
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
