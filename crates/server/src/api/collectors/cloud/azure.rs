//! Notice de azure (générée à partir de la spécification du produit).

use super::super::*;

pub(super) fn view() -> CollectorView {
    CollectorView {
        kind: "azure",
        label: "Microsoft Azure",
        summary: "Resource Health and active Azure Monitor alerts of an Azure subscription, read through a Reader service principal.",
        examples: &["Microsoft Azure subscription"],
        credential_types: &["username_password"],
        credentials: &[CredentialView {
            kind: "username_password",
            label: "Entra application (service principal)",
            help: "The client ID and a client secret of an Entra application that has the Reader role on the subscription.",
            fields: &[
                CredentialField {
                    key: "username",
                    label: "Application (client) ID",
                    help: "The Application (client) ID of the app registration, a GUID.",
                    placeholder: "00000000-0000-0000-0000-000000000000",
                    input: "text",
                    choices: &[],
                    required: true,
                },
                CredentialField {
                    key: "password",
                    label: "Client secret",
                    help: "The Value of a client secret of the app registration (shown once when created).",
                    placeholder: "",
                    input: "password",
                    choices: &[],
                    required: true,
                },
            ],
        }],
        address_hint: "management.azure.com",
        default_port: 443,
        setup: Setup {
            title: "Create a read-only application for DumbMonit",
            steps: &[
                "In the Azure portal, open Microsoft Entra ID > App registrations > New registration, name it DumbMonit, keep single tenant, then Register.",
                "On the Overview page, copy the Application (client) ID and the Directory (tenant) ID.",
                "Open Certificates & secrets > New client secret, choose a lifetime, then copy the secret Value: it is shown once.",
                "Open Subscriptions > your subscription > Access control (IAM) > Add > Add role assignment, pick the Reader role, select the DumbMonit application, then Review + assign.",
                "Copy the Subscription ID from the subscription Overview page.",
                "In DumbMonit, enter management.azure.com as address, paste the client ID and secret, and fill the Tenant ID and Subscription ID options.",
            ],
            ports: &[
                to_device!(443, "tcp", "Azure Resource Manager API (HTTPS, management.azure.com)"),
                to_device!(
                    443,
                    "tcp",
                    "Microsoft sign-in, access token (HTTPS, login.microsoftonline.com)"
                ),
            ],
            permissions: &[
                "Reader (role on the subscription)\nReads Resource Health and the active Azure Monitor alerts; changes nothing.",
            ],
            warning: "A stopped or deallocated virtual machine is reported by Azure as unavailable on the user's request: it is counted apart and raises no alert. Azure Monitor alerts need the Microsoft.AlertsManagement resource provider registered on the subscription; without it only Resource Health is monitored.",
        },
        options: &[
            OptionView {
                key: "tenant_id",
                label: "Tenant ID",
                help: "The Directory (tenant) ID of the application, a GUID or a domain such as contoso.onmicrosoft.com.",
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
                key: "subscription_id",
                label: "Subscription ID",
                help: "The subscription to watch, as shown on its Overview page in the Azure portal.",
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
                help: "Where the access token is requested. Change it only for a sovereign cloud, such as https://login.microsoftonline.us.",
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
