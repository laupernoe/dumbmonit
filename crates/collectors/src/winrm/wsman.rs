//! WS-Management : l'enveloppe SOAP d'une requête WQL (`Enumerate`, puis `Pull`
//! tant que la séquence n'est pas finie) et la lecture de la réponse.
//!
//! Seul l'espace de noms `root/cimv2` est interrogé, et seulement par WQL : rien
//! n'exécute de commande ni de script sur la machine.

use std::collections::BTreeMap;

use dumbmonit_proto::ProbeError;

pub const NS_ENUMERATION: &str = "http://schemas.xmlsoap.org/ws/2004/09/enumeration";
const RESOURCE_URI: &str = "http://schemas.microsoft.com/wbem/wsman/1/wmi/root/cimv2/*";
const WQL_DIALECT: &str = "http://schemas.microsoft.com/wbem/wsman/1/WQL";

/// Une instance WMI : propriété → valeur textuelle.
pub type Row = BTreeMap<String, String>;

/// Ce qu'une réponse `Enumerate` ou `Pull` apporte.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Page {
    pub rows: Vec<Row>,
    /// Contexte à rendre au `Pull` suivant ; `None` quand la séquence est finie.
    pub context: Option<String>,
}

pub fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

fn envelope(
    endpoint: &str,
    action: &str,
    message_id: &str,
    timeout_secs: u64,
    body: &str,
) -> String {
    format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8"?>"#,
            r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" "#,
            r#"xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing" "#,
            r#"xmlns:w="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd" "#,
            r#"xmlns:n="{ns}">"#,
            r#"<s:Header>"#,
            r#"<a:To>{to}</a:To>"#,
            r#"<w:ResourceURI s:mustUnderstand="true">{resource}</w:ResourceURI>"#,
            r#"<a:ReplyTo><a:Address s:mustUnderstand="true">"#,
            r#"http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous"#,
            r#"</a:Address></a:ReplyTo>"#,
            r#"<a:Action s:mustUnderstand="true">{action}</a:Action>"#,
            r#"<w:MaxEnvelopeSize s:mustUnderstand="true">512000</w:MaxEnvelopeSize>"#,
            r#"<a:MessageID>uuid:{id}</a:MessageID>"#,
            r#"<w:OperationTimeout>PT{timeout}S</w:OperationTimeout>"#,
            r#"</s:Header><s:Body>{body}</s:Body></s:Envelope>"#,
        ),
        ns = NS_ENUMERATION,
        to = xml_escape(endpoint),
        resource = RESOURCE_URI,
        action = action,
        id = message_id,
        timeout = timeout_secs.max(1),
        body = body,
    )
}

/// `Enumerate` d'une requête WQL.
pub fn enumerate_request(
    endpoint: &str,
    message_id: &str,
    timeout_secs: u64,
    wql: &str,
    max_elements: u32,
) -> String {
    let body = format!(
        concat!(
            "<n:Enumerate><w:OptimizeEnumeration/><w:MaxElements>{max}</w:MaxElements>",
            r#"<w:Filter Dialect="{dialect}">{wql}</w:Filter></n:Enumerate>"#
        ),
        max = max_elements,
        dialect = WQL_DIALECT,
        wql = xml_escape(wql),
    );
    envelope(endpoint, &format!("{NS_ENUMERATION}/Enumerate"), message_id, timeout_secs, &body)
}

/// `Pull` de la suite d'une énumération.
pub fn pull_request(
    endpoint: &str,
    message_id: &str,
    timeout_secs: u64,
    context: &str,
    max_elements: u32,
) -> String {
    let body = format!(
        "<n:Pull><n:EnumerationContext>{}</n:EnumerationContext>\
         <n:MaxElements>{max_elements}</n:MaxElements></n:Pull>",
        xml_escape(context)
    );
    envelope(endpoint, &format!("{NS_ENUMERATION}/Pull"), message_id, timeout_secs, &body)
}

/// Nom local d'un élément, sans préfixe d'espace de noms.
fn local<'a>(node: roxmltree::Node<'a, '_>) -> &'a str {
    node.tag_name().name()
}

fn find<'a, 'b>(root: roxmltree::Node<'a, 'b>, name: &str) -> Option<roxmltree::Node<'a, 'b>> {
    root.descendants().find(|n| n.is_element() && local(*n) == name)
}

/// Texte d'une propriété : le texte direct, ou celui du premier descendant
/// (WMI enveloppe les dates dans `<cim:Datetime>`).
fn property_text(node: roxmltree::Node<'_, '_>) -> Option<String> {
    let direct: String =
        node.children().filter(|n| n.is_text()).filter_map(|n| n.text()).collect();
    let direct = direct.trim();
    if !direct.is_empty() {
        return Some(direct.to_string());
    }
    node.descendants()
        .skip(1)
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .map(str::trim)
        .find(|t| !t.is_empty())
        .map(str::to_string)
}

/// Lit une réponse `EnumerateResponse` / `PullResponse`.
pub fn parse_page(xml: &str) -> Result<Page, ProbeError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| ProbeError::Protocol(format!("WinRM answer is not valid XML: {e}")))?;
    if let Some(fault) = fault(&doc) {
        return Err(fault);
    }
    let mut page = Page::default();
    if let Some(items) = find(doc.root_element(), "Items") {
        for instance in items.children().filter(|n| n.is_element()) {
            let mut row = Row::new();
            for property in instance.children().filter(|n| n.is_element()) {
                // `xsi:nil="true"` : propriété sans valeur.
                if let Some(value) = property_text(property) {
                    row.insert(local(property).to_string(), value);
                }
            }
            page.rows.push(row);
        }
    }
    let finished = find(doc.root_element(), "EndOfSequence").is_some();
    if !finished {
        page.context = find(doc.root_element(), "EnumerationContext")
            .and_then(property_text)
            .filter(|c| !c.is_empty());
    }
    Ok(page)
}

/// Une faute SOAP, traduite en erreur typée : l'accès refusé est un réglage à
/// corriger (compte), le reste une réponse inattendue.
pub fn fault(doc: &roxmltree::Document<'_>) -> Option<ProbeError> {
    let fault = find(doc.root_element(), "Fault")?;
    let reason = find(fault, "Reason")
        .and_then(|r| find(r, "Text"))
        .and_then(property_text)
        .unwrap_or_else(|| "unknown SOAP fault".to_string());
    // Le message WMI détaillé est dans `MSFT_WmiError` ; le texte suffit pour trier.
    let detail = find(fault, "Message").and_then(property_text);
    let lowered = format!("{reason} {}", detail.as_deref().unwrap_or("")).to_lowercase();
    let message: String = match detail {
        Some(d) if !reason.contains(&d) => format!("{reason} ({d})"),
        _ => reason,
    };
    let message: String = message.chars().take(300).collect();
    if lowered.contains("access is denied") || lowered.contains("access denied") {
        return Some(ProbeError::Auth(format!(
            "Windows refused access to WMI: {message}. Add the account to Remote Management \
             Users and grant it the WMI namespace permissions (see the documentation)."
        )));
    }
    Some(ProbeError::Protocol(format!("WinRM fault: {message}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OS: &str = include_str!("testdata/enumerate_os.xml");
    const DISKS_FIRST: &str = include_str!("testdata/pull_first.xml");
    const DISKS_LAST: &str = include_str!("testdata/pull_last.xml");
    const DENIED: &str = include_str!("testdata/fault_access_denied.xml");
    const BAD_QUERY: &str = include_str!("testdata/fault_invalid_class.xml");

    #[test]
    fn une_enumeration_complete_donne_ses_proprietes() {
        let page = parse_page(OS).expect("réponse valide");
        assert_eq!(page.context, None);
        assert_eq!(page.rows.len(), 1);
        let os = &page.rows[0];
        assert_eq!(os["Caption"], "Microsoft Windows Server 2022 Standard");
        assert_eq!(os["TotalVisibleMemorySize"], "16776244");
        // La date est enveloppée dans <cim:Datetime>.
        assert_eq!(os["LastBootUpTime"], "2026-10-20T06:11:12.500000+00:00");
        // Une propriété `xsi:nil` n'apparaît pas.
        assert!(!os.contains_key("CSDVersion"));
    }

    #[test]
    fn une_sequence_inachevee_rend_son_contexte() {
        let first = parse_page(DISKS_FIRST).unwrap();
        assert_eq!(first.rows.len(), 1);
        assert_eq!(first.context.as_deref(), Some("uuid:11111111-2222-3333-4444-555555555555"));
        let last = parse_page(DISKS_LAST).unwrap();
        assert_eq!(last.rows[0]["DeviceID"], "D:");
        assert_eq!(last.context, None);
    }

    #[test]
    fn un_acces_refuse_est_une_erreur_de_compte() {
        match parse_page(DENIED) {
            Err(ProbeError::Auth(message)) => assert!(message.contains("Remote Management")),
            other => panic!("attendu Auth, obtenu {other:?}"),
        }
    }

    #[test]
    fn une_autre_faute_est_une_reponse_inattendue() {
        match parse_page(BAD_QUERY) {
            Err(ProbeError::Protocol(message)) => assert!(message.contains("Invalid class")),
            other => panic!("attendu Protocol, obtenu {other:?}"),
        }
        assert!(matches!(parse_page("pas du xml"), Err(ProbeError::Protocol(_))));
    }

    #[test]
    fn la_requete_echappe_le_wql_et_porte_l_action() {
        let xml = enumerate_request(
            "https://srv:5986/wsman",
            "abc",
            20,
            "SELECT * FROM Win32_Service WHERE Name = 'a&b' AND X < 3",
            100,
        );
        assert!(xml.contains("a&amp;b"));
        assert!(xml.contains("X &lt; 3"));
        assert!(xml.contains("/enumeration/Enumerate</a:Action>"));
        assert!(xml.contains("PT20S"));
        // Le document produit est lui-même du XML valide.
        roxmltree::Document::parse(&xml).expect("XML bien formé");
        let pull = pull_request("https://srv:5986/wsman", "abc", 20, "ctx<1>", 50);
        roxmltree::Document::parse(&pull).expect("XML bien formé");
        assert!(pull.contains("ctx&lt;1&gt;"));
    }
}
