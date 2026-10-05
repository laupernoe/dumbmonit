/**
 * Kind-specific panels of the device page.
 *
 * A collector kind that deserves more than the generic charts (Proxmox VE with
 * its guests, PBS with its backup calendar, Synology with its disks…) registers
 * a panel here; the page mounts it between the alerts and the instruments,
 * with `{ target }` as its only prop. One line per kind, so several people can
 * add theirs without touching the page.
 *
 * Each entry is a dynamic import rather than a static one: a device page shows
 * exactly one panel, so there is no reason to ship and parse the other forty-odd
 * ones. `loadKindPanel` resolves the component for a kind; the page shows a
 * skeleton until it settles.
 */
import type { Component } from 'svelte';
import type { Target } from '$lib/api/types';

export type KindPanel = Component<{ target: Target }>;
type PanelLoader = () => Promise<{ default: KindPanel }>;

const kindPanelLoaders: Record<string, PanelLoader> = {
	pbs: () => import('./pbs/PbsPanel.svelte'),
	pdm: () => import('./pdm/PdmPanel.svelte'),
	pmg: () => import('./pmg/PmgPanel.svelte'),
	agent: () => import('./agent/AgentPanel.svelte'),
	proxmox: () => import('./proxmox/ProxmoxPanel.svelte'),
	synology: () => import('./synology/SynologyPanel.svelte'),
	push: () => import('./push/PushPanel.svelte'),
	opnsense: () => import('./opnsense/OpnsensePanel.svelte'),
	truenas: () => import('./truenas/TruenasPanel.svelte'),
	redfish: () => import('./redfish/RedfishPanel.svelte'),
	victoriametrics: () => import('./observability/ObservabilityPanel.svelte'),
	victorialogs: () => import('./observability/ObservabilityPanel.svelte'),
	loki: () => import('./observability/ObservabilityPanel.svelte'),
	graylog: () => import('./observability/ObservabilityPanel.svelte'),
	mdaemon: () => import('./mdaemon/MailServicesPanel.svelte'),
	securitygateway: () => import('./mdaemon/MailServicesPanel.svelte'),
	pihole: () => import('./pihole/PiholePanel.svelte'),
	adguard: () => import('./adguard/AdguardPanel.svelte'),
	nut: () => import('./nut/NutPanel.svelte'),
	mikrotik: () => import('./mikrotik/MikrotikPanel.svelte'),
	unifi: () => import('./unifi/UnifiPanel.svelte'),
	homeassistant: () => import('./homeassistant/HomeAssistantPanel.svelte'),
	vsphere: () => import('./vsphere/VspherePanel.svelte'),
	redis: () => import('./backends/BackendPanel.svelte'),
	mongodb: () => import('./backends/BackendPanel.svelte'),
	rabbitmq: () => import('./backends/BackendPanel.svelte'),
	crowdsec: () => import('./backends/BackendPanel.svelte'),
	traefik: () => import('./proxies/ProxyPanel.svelte'),
	caddy: () => import('./proxies/ProxyPanel.svelte'),
	npm: () => import('./proxies/ProxyPanel.svelte'),
	domain: () => import('./proxies/ProxyPanel.svelte'),
	nextcloud: () => import('./selfhosted/SelfHostedPanel.svelte'),
	immich: () => import('./selfhosted/SelfHostedPanel.svelte'),
	paperless: () => import('./selfhosted/SelfHostedPanel.svelte'),
	jellyfin: () => import('./selfhosted/SelfHostedPanel.svelte'),
	plex: () => import('./selfhosted/SelfHostedPanel.svelte'),
	gitlab: () => import('./selfhosted/SelfHostedPanel.svelte'),
	forgejo: () => import('./selfhosted/SelfHostedPanel.svelte'),
	kubernetes: () => import('./kubernetes/KubernetesPanel.svelte'),
	activedirectory: () => import('./activedirectory/ActiveDirectoryPanel.svelte'),
	pfsense: () => import('./appliance/AppliancePanel.svelte'),
	unraid: () => import('./appliance/AppliancePanel.svelte'),
	veeam: () => import('./appliance/AppliancePanel.svelte'),
	tailscale: () => import('./appliance/AppliancePanel.svelte'),
	fortigate: () => import('./appliance/AppliancePanel.svelte'),
	sophos: () => import('./appliance/AppliancePanel.svelte'),
	nginx: () => import('./appliance/AppliancePanel.svelte'),
	apache: () => import('./appliance/AppliancePanel.svelte'),
	webchange: () => import('./webchange/WebchangePanel.svelte')
};

/** Whether a kind has a dedicated panel, without loading it. */
export function hasKindPanel(kind: string): boolean {
	return kind in kindPanelLoaders;
}

/** Loads the panel component for a kind; `null` when there is none (unknown kind). */
export function loadKindPanel(kind: string): Promise<KindPanel> | null {
	const loader = kindPanelLoaders[kind];
	return loader ? loader().then((mod) => mod.default) : null;
}
