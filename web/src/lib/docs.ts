import { getLocale } from '#lib/paraglide/runtime.js';

/** Slug de chaque langue de l'interface dans les projets Read the Docs (minuscules). */
const DOCS_LANGUAGE: Record<string, string> = {
	'pt-BR': 'pt-br',
	'zh-Hans': 'zh-cn'
};

/** Page "Website banner" of the documentation, in the language of the UI. */
export function websiteBannerDocsUrl(): string {
	const locale = getLocale();
	return `https://dumbmonit.readthedocs.io/${DOCS_LANGUAGE[locale] ?? locale}/latest/integrations/website-banner/`;
}

/**
 * A link to our Read the Docs project (the server sends the English one),
 * switched to the language of the UI. Any other link is returned unchanged.
 */
export function localizedDocsUrl(url: string): string {
	const prefix = 'https://dumbmonit.readthedocs.io/en/';
	if (!url.startsWith(prefix)) return url;
	const locale = getLocale();
	return `https://dumbmonit.readthedocs.io/${DOCS_LANGUAGE[locale] ?? locale}/${url.slice(prefix.length)}`;
}

/** A page of the documentation (`devices/agent/`, `install/remote-site/#anchor`), in the language of the UI. */
export function docsUrl(page: string): string {
	return localizedDocsUrl(`https://dumbmonit.readthedocs.io/en/latest/${page}`);
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
	const locale = getLocale();
	return `https://dumbmonit.readthedocs.io/${DOCS_LANGUAGE[locale] ?? locale}/latest/devices/agent-active-directory/`;
}
