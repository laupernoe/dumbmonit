/**
 * Kind-specific panels of the device page.
 *
 * A collector kind that deserves more than the generic charts (Proxmox VE with
 * its guests, PBS with its backup calendar, Synology with its disks…) registers
 * a panel here; the page mounts it between the alerts and the instruments,
 * with `{ target }` as its only prop. One line per kind, so several people can
 * add theirs without touching the page.
 */
import type { Component } from 'svelte';
import type { Target } from '$lib/api/types';
import AdguardPanel from './adguard/AdguardPanel.svelte';
import AgentPanel from './agent/AgentPanel.svelte';
import BackendPanel from './backends/BackendPanel.svelte';
import MailServicesPanel from './mdaemon/MailServicesPanel.svelte';
import MikrotikPanel from './mikrotik/MikrotikPanel.svelte';
import NutPanel from './nut/NutPanel.svelte';
import ObservabilityPanel from './observability/ObservabilityPanel.svelte';
import OpnsensePanel from './opnsense/OpnsensePanel.svelte';
import PbsPanel from './pbs/PbsPanel.svelte';
import PdmPanel from './pdm/PdmPanel.svelte';
import PiholePanel from './pihole/PiholePanel.svelte';
import PmgPanel from './pmg/PmgPanel.svelte';
import ProxmoxPanel from './proxmox/ProxmoxPanel.svelte';
import ProxyPanel from './proxies/ProxyPanel.svelte';
import PushPanel from './push/PushPanel.svelte';
import RedfishPanel from './redfish/RedfishPanel.svelte';
import SelfHostedPanel from './selfhosted/SelfHostedPanel.svelte';
import SynologyPanel from './synology/SynologyPanel.svelte';
import TruenasPanel from './truenas/TruenasPanel.svelte';
import UnifiPanel from './unifi/UnifiPanel.svelte';
import HomeAssistantPanel from './homeassistant/HomeAssistantPanel.svelte';
import KubernetesPanel from './kubernetes/KubernetesPanel.svelte';
import AppliancePanel from './appliance/AppliancePanel.svelte';
import VspherePanel from './vsphere/VspherePanel.svelte';
import WebchangePanel from './webchange/WebchangePanel.svelte';
import ActiveDirectoryPanel from './activedirectory/ActiveDirectoryPanel.svelte';

export type KindPanel = Component<{ target: Target }>;

export const kindPanels: Record<string, KindPanel> = { pbs: PbsPanel, pdm: PdmPanel, pmg: PmgPanel, agent: AgentPanel, proxmox: ProxmoxPanel, synology: SynologyPanel, push: PushPanel, opnsense: OpnsensePanel, truenas: TruenasPanel, redfish: RedfishPanel, victoriametrics: ObservabilityPanel, victorialogs: ObservabilityPanel, loki: ObservabilityPanel, graylog: ObservabilityPanel, mdaemon: MailServicesPanel, securitygateway: MailServicesPanel, pihole: PiholePanel, adguard: AdguardPanel, nut: NutPanel, mikrotik: MikrotikPanel, unifi: UnifiPanel, homeassistant: HomeAssistantPanel, vsphere: VspherePanel, redis: BackendPanel, mongodb: BackendPanel, rabbitmq: BackendPanel, crowdsec: BackendPanel, traefik: ProxyPanel, caddy: ProxyPanel, npm: ProxyPanel, domain: ProxyPanel, nextcloud: SelfHostedPanel, immich: SelfHostedPanel, paperless: SelfHostedPanel, jellyfin: SelfHostedPanel, plex: SelfHostedPanel, kubernetes: KubernetesPanel, activedirectory: ActiveDirectoryPanel, pfsense: AppliancePanel, unraid: AppliancePanel, veeam: AppliancePanel, tailscale: AppliancePanel, fortigate: AppliancePanel, sophos: AppliancePanel, webchange: WebchangePanel };

/** The panel for a kind, or `null` when the generic charts are all there is. */
export function kindPanel(kind: string): KindPanel | null {
	return kindPanels[kind] ?? null;
}
