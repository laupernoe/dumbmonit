//! L'API vSphere Web Services (SOAP, `/sdk`), réduite aux six appels utiles.
//!
//! `RetrieveServiceContent` (sans session) donne la version et les références
//! des gestionnaires ; `Login` ouvre une session, portée ensuite par le cookie
//! `vmware_soap_session` ; `CreateContainerView` + `RetrievePropertiesEx`
//! (et `ContinueRetrievePropertiesEx` tant qu'un jeton revient) lisent en un
//! seul passage les propriétés voulues de tous les hôtes, VM et banques de
//! données ; `DestroyView` rend la vue. Rien d'autre : aucune tâche, aucun
//! événement, aucune modification.
//!
//! Les références d'objets gérés ne sont jamais écrites en dur : un ESXi seul
//! nomme son collecteur `ha-property-collector` et son gestionnaire de
//! sessions `ha-sessionmgr`, un vCenter `propertyCollector` et
//! `SessionManager`. Elles viennent toutes de `ServiceContent`.
//!
//! Les enveloppes sont écrites à la main et les réponses lues avec
//! `roxmltree` : un client SOAP généré pour les milliers de types de vim25
//! coûterait bien plus que les quelques balises lues ici.

use std::collections::BTreeMap;

use dumbmonit_proto::ProbeError;
use roxmltree::{Document, Node};

/// Version d'API annoncée dans `SOAPAction`. 6.5 est comprise par tous les
/// ESXi et vCenter encore en service, et par le simulateur `vcsim`.
pub const SOAP_ACTION: &str = "urn:vim25/6.5";

/// Nombre d'objets par page de `RetrievePropertiesEx`.
pub const PAGE_SIZE: u32 = 500;

/// Propriétés lues pour chaque type d'objet. Toutes existent depuis vSphere 5.
pub const HOST_PATHS: &[&str] = &[
    "name",
    "runtime.connectionState",
    "runtime.powerState",
    "runtime.inMaintenanceMode",
    "overallStatus",
    "summary.quickStats.overallCpuUsage",
    "summary.quickStats.overallMemoryUsage",
    "summary.quickStats.uptime",
    "summary.hardware.cpuMhz",
    "summary.hardware.numCpuCores",
    "summary.hardware.memorySize",
];
pub const VM_PATHS: &[&str] =
    &["name", "runtime.powerState", "overallStatus", "guest.toolsStatus", "config.template"];
pub const DATASTORE_PATHS: &[&str] =
    &["name", "summary.capacity", "summary.freeSpace", "summary.accessible", "overallStatus"];

/// Référence d'objet géré : `<obj type="HostSystem">host-21</obj>`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MoRef {
    pub kind: String,
    pub id: String,
}

impl MoRef {
    fn from_node(node: Node) -> Option<Self> {
        Some(Self {
            kind: node.attribute("type")?.to_string(),
            id: node.text()?.trim().to_string(),
        })
    }

    fn xml(&self, tag: &str) -> String {
        format!("<{tag} type=\"{}\">{}</{tag}>", escape(&self.kind), escape(&self.id))
    }
}

/// Ce que `RetrieveServiceContent` apprend du serveur.
#[derive(Debug, Clone)]
pub struct ServiceContent {
    pub root_folder: MoRef,
    pub property_collector: MoRef,
    pub view_manager: MoRef,
    pub session_manager: MoRef,
    pub about: About,
}

#[derive(Debug, Clone, Default)]
pub struct About {
    pub full_name: String,
    pub version: String,
    pub build: String,
    /// `VirtualCenter` pour un vCenter, `HostAgent` pour un ESXi seul.
    pub api_type: String,
}

/// Un objet et ses propriétés, telles que `RetrievePropertiesEx` les rend.
#[derive(Debug, Clone)]
pub struct ObjectContent {
    pub obj: MoRef,
    /// Valeur textuelle des propriétés simples.
    pub props: BTreeMap<String, String>,
    /// Propriétés refusées ou introuvables (`missingSet`), avec la raison.
    pub missing: Vec<(String, String)>,
}

impl ObjectContent {
    pub fn text(&self, path: &str) -> Option<&str> {
        self.props.get(path).map(String::as_str)
    }

    pub fn number(&self, path: &str) -> Option<f64> {
        self.text(path)?.trim().parse().ok()
    }

    pub fn flag(&self, path: &str) -> Option<bool> {
        match self.text(path)?.trim() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        }
    }
}

/// Une page de résultats et, s'il en reste, le jeton de la suivante.
#[derive(Debug, Default)]
pub struct Page {
    pub objects: Vec<ObjectContent>,
    pub token: Option<String>,
}

/// Une alarme déclenchée (`AlarmState`).
#[derive(Debug, Clone, PartialEq)]
pub struct AlarmState {
    pub entity: MoRef,
    pub alarm: MoRef,
    /// `red`, `yellow`, `green` ou `gray`.
    pub status: String,
    pub acknowledged: bool,
}

// ------------------------------------------------------------ enveloppes

/// Échappe un texte pour un nœud ou un attribut XML.
pub fn escape(text: &str) -> String {
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

pub fn envelope(body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <soapenv:Envelope xmlns:soapenv=\"http://schemas.xmlsoap.org/soap/envelope/\" \
         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns=\"urn:vim25\">\
         <soapenv:Body>{body}</soapenv:Body></soapenv:Envelope>"
    )
}

pub fn retrieve_service_content() -> String {
    envelope(
        "<RetrieveServiceContent><_this type=\"ServiceInstance\">ServiceInstance</_this>\
         </RetrieveServiceContent>",
    )
}

pub fn login(session_manager: &MoRef, username: &str, password: &str) -> String {
    envelope(&format!(
        "<Login>{}<userName>{}</userName><password>{}</password></Login>",
        session_manager.xml("_this"),
        escape(username),
        escape(password)
    ))
}

pub fn create_container_view(view_manager: &MoRef, root: &MoRef) -> String {
    envelope(&format!(
        "<CreateContainerView>{}{}<type>HostSystem</type><type>VirtualMachine</type>\
         <type>Datastore</type><recursive>true</recursive></CreateContainerView>",
        view_manager.xml("_this"),
        root.xml("container")
    ))
}

pub fn destroy_view(view: &MoRef) -> String {
    envelope(&format!("<DestroyView>{}</DestroyView>", view.xml("_this")))
}

fn prop_set(kind: &str, paths: &[&str]) -> String {
    let paths: String = paths.iter().map(|p| format!("<pathSet>{p}</pathSet>")).collect();
    format!("<propSet><type>{kind}</type>{paths}</propSet>")
}

/// Toutes les propriétés d'inventaire, lues à travers la vue.
pub fn retrieve_inventory(collector: &MoRef, view: &MoRef) -> String {
    envelope(&format!(
        "<RetrievePropertiesEx>{}<specSet>{}{}{}<objectSet>{}<skip>true</skip>\
         <selectSet xsi:type=\"TraversalSpec\"><name>view</name><type>ContainerView</type>\
         <path>view</path><skip>false</skip></selectSet></objectSet></specSet>\
         <options><maxObjects>{PAGE_SIZE}</maxObjects></options></RetrievePropertiesEx>",
        collector.xml("_this"),
        prop_set("HostSystem", HOST_PATHS),
        prop_set("VirtualMachine", VM_PATHS),
        prop_set("Datastore", DATASTORE_PATHS),
        view.xml("obj"),
    ))
}

pub fn continue_retrieve(collector: &MoRef, token: &str) -> String {
    envelope(&format!(
        "<ContinueRetrievePropertiesEx>{}<token>{}</token></ContinueRetrievePropertiesEx>",
        collector.xml("_this"),
        escape(token)
    ))
}

/// Les alarmes déclenchées, telles que le dossier racine les agrège.
pub fn retrieve_alarm_states(collector: &MoRef, root: &MoRef) -> String {
    envelope(&format!(
        "<RetrievePropertiesEx>{}<specSet>{}<objectSet>{}</objectSet></specSet><options/>\
         </RetrievePropertiesEx>",
        collector.xml("_this"),
        prop_set(&root.kind, &["triggeredAlarmState"]),
        root.xml("obj"),
    ))
}

/// Le nom de chaque définition d'alarme citée.
pub fn retrieve_alarm_names(collector: &MoRef, alarms: &[&MoRef]) -> String {
    let objects: String =
        alarms.iter().map(|alarm| format!("<objectSet>{}</objectSet>", alarm.xml("obj"))).collect();
    envelope(&format!(
        "<RetrievePropertiesEx>{}<specSet>{}{objects}</specSet><options/></RetrievePropertiesEx>",
        collector.xml("_this"),
        prop_set("Alarm", &["info.name"]),
    ))
}

// ------------------------------------------------------------ réponses

/// Une faute SOAP, classée pour l'alerting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// `InvalidLogin` : identifiant ou mot de passe refusé.
    InvalidLogin,
    /// `NotAuthenticated` : la session a expiré ou n'existe pas.
    NotAuthenticated,
    /// `NoPermission` : le compte n'a pas le droit de lire.
    NoPermission(String),
    Other(String),
}

impl Fault {
    pub fn into_probe_error(self) -> ProbeError {
        match self {
            Self::InvalidLogin => ProbeError::Auth(
                "vSphere refused the user name or password (InvalidLogin).".to_string(),
            ),
            Self::NotAuthenticated => ProbeError::Auth(
                "vSphere did not accept the session (NotAuthenticated).".to_string(),
            ),
            Self::NoPermission(detail) => ProbeError::Auth(format!(
                "The vSphere account may not read the inventory (NoPermission): {detail}. Give \
                 it the Read-only role on the vCenter or host root, with propagation."
            )),
            Self::Other(detail) => {
                ProbeError::Protocol(format!("vSphere answered a fault: {detail}"))
            }
        }
    }
}

fn parse(body: &str) -> Result<Document<'_>, ProbeError> {
    Document::parse(body).map_err(|error| {
        let excerpt: String = body.chars().take(120).collect();
        ProbeError::Protocol(format!(
            "The answer is not XML ({error}): is this a vCenter or ESXi /sdk endpoint? \
             It starts with: {excerpt}"
        ))
    })
}

fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children().find(|n| n.is_element() && n.tag_name().name() == name)
}

fn children<'a, 'input>(
    node: Node<'a, 'input>,
    name: &'a str,
) -> impl Iterator<Item = Node<'a, 'input>> + 'a {
    node.children().filter(move |n| n.is_element() && n.tag_name().name() == name)
}

fn text_of(node: Option<Node>) -> String {
    node.and_then(|n| n.text()).map(|t| t.trim().to_string()).unwrap_or_default()
}

/// Le type xsi d'un nœud, quelle que soit la façon dont l'espace de noms est
/// écrit (`xsi:type`, ou l'alias qu'emploie `vcsim`).
fn xsi_type<'a>(node: Node<'a, '_>) -> Option<&'a str> {
    node.attributes().find(|attr| attr.name() == "type" && attr.namespace().is_some()).map(|attr| {
        let value = attr.value();
        value.rsplit(':').next().unwrap_or(value)
    })
}

/// Le corps de la réponse, ou la faute qu'elle porte.
fn body_or_fault<'a, 'input>(doc: &'a Document<'input>) -> Result<Node<'a, 'input>, Fault> {
    let body = doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "Body")
        .ok_or_else(|| Fault::Other("no SOAP body".to_string()))?;
    let Some(fault) = child(body, "Fault") else {
        return body
            .children()
            .find(|n| n.is_element())
            .ok_or_else(|| Fault::Other("empty SOAP body".to_string()));
    };
    let message = text_of(child(fault, "faultstring"));
    let detail = child(fault, "detail").and_then(|d| d.children().find(|n| n.is_element()));
    let kind = detail
        .map(|d| xsi_type(d).unwrap_or(d.tag_name().name()).trim_end_matches("Fault").to_string())
        .unwrap_or_default();
    Err(match kind.as_str() {
        "InvalidLogin" => Fault::InvalidLogin,
        "NotAuthenticated" => Fault::NotAuthenticated,
        "NoPermission" => Fault::NoPermission(message),
        _ if message.contains("not authenticated") => Fault::NotAuthenticated,
        _ => Fault::Other(if kind.is_empty() { message } else { format!("{kind}: {message}") }),
    })
}

/// Lit une réponse et en rend le corps, ou l'erreur de sonde correspondante.
pub fn read<T>(
    body: &str,
    decode: impl FnOnce(Node) -> Result<T, ProbeError>,
) -> Result<Result<T, Fault>, ProbeError> {
    let doc = parse(body)?;
    match body_or_fault(&doc) {
        Ok(node) => Ok(Ok(decode(node)?)),
        Err(fault) => Ok(Err(fault)),
    }
}

fn moref(node: Node, name: &str) -> Result<MoRef, ProbeError> {
    child(node, name)
        .and_then(MoRef::from_node)
        .ok_or_else(|| ProbeError::Protocol(format!("ServiceContent without {name}")))
}

pub fn decode_service_content(response: Node) -> Result<ServiceContent, ProbeError> {
    let content = child(response, "returnval")
        .ok_or_else(|| ProbeError::Protocol("RetrieveServiceContent without returnval".into()))?;
    let about = child(content, "about");
    let field = |name: &str| text_of(about.and_then(|a| child(a, name)));
    Ok(ServiceContent {
        root_folder: moref(content, "rootFolder")?,
        property_collector: moref(content, "propertyCollector")?,
        view_manager: moref(content, "viewManager")?,
        session_manager: moref(content, "sessionManager")?,
        about: About {
            full_name: field("fullName"),
            version: field("version"),
            build: field("build"),
            api_type: field("apiType"),
        },
    })
}

pub fn decode_moref(response: Node) -> Result<MoRef, ProbeError> {
    child(response, "returnval")
        .and_then(MoRef::from_node)
        .ok_or_else(|| ProbeError::Protocol("Answer without a managed object reference".into()))
}

pub fn decode_nothing(_: Node) -> Result<(), ProbeError> {
    Ok(())
}

/// Une page de `RetrievePropertiesEx` ou `ContinueRetrievePropertiesEx`.
///
/// Une réponse vide (aucun objet) n'a même pas de `returnval`.
pub fn decode_page(response: Node) -> Result<Page, ProbeError> {
    let Some(returnval) = child(response, "returnval") else {
        return Ok(Page::default());
    };
    let token = child(returnval, "token").and_then(|t| t.text()).map(|t| t.trim().to_string());
    let mut objects = Vec::new();
    for object in children(returnval, "objects") {
        let Some(obj) = child(object, "obj").and_then(MoRef::from_node) else { continue };
        let mut props = BTreeMap::new();
        for prop in children(object, "propSet") {
            let name = text_of(child(prop, "name"));
            // Les valeurs composées (tableaux) sont lues à part ; ici, le texte.
            if let Some(val) = child(prop, "val")
                && val.children().all(|n| !n.is_element())
            {
                props.insert(name, val.text().unwrap_or_default().trim().to_string());
            }
        }
        let missing = children(object, "missingSet")
            .map(|m| {
                let path = text_of(child(m, "path"));
                let reason = child(m, "fault")
                    .and_then(|f| child(f, "fault"))
                    .and_then(xsi_type)
                    .unwrap_or("unknown")
                    .to_string();
                (path, reason)
            })
            .collect();
        objects.push(ObjectContent { obj, props, missing });
    }
    Ok(Page { objects, token })
}

/// Les `AlarmState` du tableau `triggeredAlarmState`.
pub fn decode_alarm_states(response: Node) -> Result<Vec<AlarmState>, ProbeError> {
    let mut out = Vec::new();
    let Some(returnval) = child(response, "returnval") else { return Ok(out) };
    for object in children(returnval, "objects") {
        for prop in children(object, "propSet") {
            let Some(val) = child(prop, "val") else { continue };
            for state in children(val, "AlarmState") {
                let (Some(entity), Some(alarm)) = (
                    child(state, "entity").and_then(MoRef::from_node),
                    child(state, "alarm").and_then(MoRef::from_node),
                ) else {
                    continue;
                };
                out.push(AlarmState {
                    entity,
                    alarm,
                    status: text_of(child(state, "overallStatus")),
                    acknowledged: text_of(child(state, "acknowledged")) == "true",
                });
            }
        }
    }
    Ok(out)
}

/// Vrai si une page contient une propriété refusée faute de session : `vcsim`
/// le dit ainsi plutôt que par une faute.
pub fn page_not_authenticated(page: &Page) -> bool {
    page.objects.iter().any(|o| o.missing.iter().any(|(_, reason)| reason == "NotAuthenticated"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Réponses réelles du simulateur `vcsim` 0.56.0 (govmomi), en mode vCenter
    // puis en mode ESXi seul, interrogé par un compte ordinaire.
    const SC_VCENTER: &str = include_str!("testdata/vcsim_0.56.0/vcenter/service_content.xml");
    const SC_ESX: &str = include_str!("testdata/vcsim_0.56.0/esx/service_content.xml");
    const LOGIN_REFUSED: &str = include_str!("testdata/vcsim_0.56.0/vcenter/login_refused.xml");
    const PAGE1: &str = include_str!("testdata/vcsim_0.56.0/vcenter/inventory_page1.xml");
    const AFTER_LOGOUT: &str = include_str!("testdata/vcsim_0.56.0/vcenter/after_logout.xml");
    const VIEW: &str = include_str!("testdata/vcsim_0.56.0/vcenter/create_view.xml");
    const ALARMS_EMPTY: &str = include_str!("testdata/vcsim_0.56.0/vcenter/alarms.xml");
    // Forme documentée d'un `triggeredAlarmState` non vide (vSphere Web
    // Services API, type AlarmState) : `vcsim` ne déclenche pas d'alarme.
    const ALARMS_DOC: &str = include_str!("testdata/documented/alarm_states.xml");

    #[test]
    fn le_service_content_donne_les_references_propres_a_chaque_produit() {
        let vc = read(SC_VCENTER, decode_service_content).unwrap().unwrap();
        assert_eq!(vc.about.api_type, "VirtualCenter");
        assert_eq!(vc.about.version, "6.5.0");
        assert_eq!(vc.property_collector.id, "propertyCollector");
        assert_eq!(vc.session_manager.id, "SessionManager");
        assert_eq!(vc.root_folder, MoRef { kind: "Folder".into(), id: "group-d1".into() });
        let esx = read(SC_ESX, decode_service_content).unwrap().unwrap();
        assert_eq!(esx.about.api_type, "HostAgent");
        assert_eq!(esx.property_collector.id, "ha-property-collector");
        assert_eq!(esx.session_manager.id, "ha-sessionmgr");
        assert_eq!(esx.root_folder.id, "ha-folder-root");
    }

    #[test]
    fn un_mot_de_passe_refuse_est_une_faute_invalid_login() {
        let fault = read(LOGIN_REFUSED, decode_nothing).unwrap().unwrap_err();
        assert_eq!(fault, Fault::InvalidLogin);
        assert!(matches!(fault.into_probe_error(), ProbeError::Auth(_)));
    }

    #[test]
    fn une_page_porte_ses_objets_et_le_jeton_de_la_suivante() {
        let page = read(PAGE1, decode_page).unwrap().unwrap();
        assert_eq!(page.objects.len(), 2);
        assert!(page.token.is_some());
        let vm = &page.objects[0];
        assert_eq!(vm.obj.kind, "VirtualMachine");
        assert_eq!(vm.text("name"), Some("DC0_H0_VM0"));
        assert_eq!(vm.text("runtime.powerState"), Some("poweredOn"));
        assert_eq!(vm.flag("config.template"), Some(false));
    }

    #[test]
    fn une_session_perdue_se_reconnait_meme_sans_faute() {
        let page = read(AFTER_LOGOUT, decode_page).unwrap().unwrap();
        assert!(page_not_authenticated(&page));
        let view = read(VIEW, decode_moref).unwrap().unwrap();
        assert_eq!(view.kind, "ContainerView");
    }

    #[test]
    fn les_alarmes_declenchees_sont_lues_avec_leur_entite() {
        assert!(read(ALARMS_EMPTY, decode_alarm_states).unwrap().unwrap().is_empty());
        let states = read(ALARMS_DOC, decode_alarm_states).unwrap().unwrap();
        assert_eq!(states.len(), 3);
        assert_eq!(states[0].entity, MoRef { kind: "HostSystem".into(), id: "host-37".into() });
        assert_eq!(states[0].alarm.id, "alarm-1");
        assert_eq!(states[0].status, "red");
        assert!(!states[0].acknowledged);
        assert!(states[2].acknowledged);
    }

    #[test]
    fn les_textes_sont_echappes_dans_les_enveloppes() {
        let manager = MoRef { kind: "SessionManager".into(), id: "SessionManager".into() };
        let body = login(&manager, "dumbmonit@vsphere.local", "a&b<c>\"d'");
        assert!(body.contains("<password>a&amp;b&lt;c&gt;&quot;d&apos;</password>"));
        assert!(Document::parse(&body).is_ok());
        let collector = MoRef { kind: "PropertyCollector".into(), id: "propertyCollector".into() };
        let view = MoRef { kind: "ContainerView".into(), id: "session[x]y".into() };
        assert!(Document::parse(&retrieve_inventory(&collector, &view)).is_ok());
        let alarm = MoRef { kind: "Alarm".into(), id: "alarm-1".into() };
        assert!(Document::parse(&retrieve_alarm_names(&collector, &[&alarm])).is_ok());
    }

    #[test]
    fn une_reponse_qui_n_est_pas_du_xml_est_une_erreur_de_protocole() {
        let error = read("<html><body>Login", decode_nothing).unwrap_err();
        assert!(matches!(error, ProbeError::Protocol(_)));
    }
}
