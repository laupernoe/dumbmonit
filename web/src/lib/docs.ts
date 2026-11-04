import { getLocale } from '#lib/paraglide/runtime.js';

/** Codes de langue de l'interface dans les URL de la documentation (Starlight : l'anglais est à la racine). */
const DOCS_LANGUAGE: Record<string, string> = {};

const DOCS_ROOT = 'https://dumbmonit.selfhostr.io/docs/';

/** Racine de la documentation dans la langue de l'interface (`/docs/` en anglais, `/docs/fr/` sinon). */
function docsRoot(): string {
	const locale = getLocale();
	return locale === 'en' ? DOCS_ROOT : `${DOCS_ROOT}${DOCS_LANGUAGE[locale] ?? locale}/`;
}

/** Page "Website banner" of the documentation, in the language of the UI. */
export function websiteBannerDocsUrl(): string {
	return `${docsRoot()}integrations/website-banner/`;
}

/**
 * A link to the documentation on the website (the server sends the English one),
 * switched to the language of the UI. Any other link is returned unchanged.
 */
export function localizedDocsUrl(url: string): string {
	if (!url.startsWith(DOCS_ROOT)) return url;
	return `${docsRoot()}${url.slice(DOCS_ROOT.length)}`;
}

/** A page of the documentation (`devices/agent/`, `install/remote-site/#anchor`), in the language of the UI. */
export function docsUrl(page: string): string {
	return `${docsRoot()}${page}`;
}

/** Kinds without a page of their own: the closest page of the documentation. */
const KIND_DOCS: Record<string, string> = {
	http: 'website',
	snmp_oid: 'snmp',
	victorialogs: 'victoriametrics',
	agent_service: 'agent',
	dummy: ''
};
const SERVICE_KINDS = new Set(['dns', 'grpc', 'imap', 'mqtt', 'mysql', 'ntp', 'ping', 'pop3', 'postgres', 'radius', 'smtp', 'ssh', 'tcp', 'tls', 'udp', 'websocket']);

/** Page of the documentation that covers a monitor kind (`devices/proxmox/`). */
export function kindDocPage(kind: string): string {
	if (SERVICE_KINDS.has(kind)) return 'devices/services/';
	const slug = kind in KIND_DOCS ? KIND_DOCS[kind] : kind;
	return slug ? `devices/${slug}/` : 'devices/';
}

/** Page "Deploy with Active Directory" of the documentation, in the language of the UI. */
export function adDeployDocsUrl(): string {
	return `${docsRoot()}devices/agent-active-directory/`;
}
