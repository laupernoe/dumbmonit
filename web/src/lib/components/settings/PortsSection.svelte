<script lang="ts">
	/**
	 * Settings → System → Network ports: which ports to open in the firewall,
	 * why, who reaches them, and whether they may face the Internet. The list is
	 * computed by the server from the running configuration
	 * (`GET /api/security/ports`, administrators only): a disabled listener is
	 * marked off, and the copyable docker-compose and ufw blocks only carry the
	 * ports that really listen. Exposure is always text plus icon.
	 */
	import { Globe, Lock, Network, ShieldOff } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getPorts } from '#lib/api/ports.js';
	import type { PortEntry, PortExposure, PortStatus, PortsView } from '#lib/api/types.js';
	import type { Tone } from '#lib/ui/index.js';
	import { CopyBlock, ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';

	let data = $state<PortsView | null>(null);
	let loadError = $state<unknown>(null);

	$effect(() => {
		const controller = new AbortController();
		getPorts(controller.signal)
			.then((view) => (data = view))
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				loadError = cause;
			});
		return () => controller.abort();
	});

	interface Text {
		name: string;
		purpose: string;
		who: string;
		/** What must hold before this port faces the Internet. */
		condition?: string;
	}

	/** The words of one port; an id this build does not know shows as it is. */
	function textOf(port: PortEntry): Text {
		switch (port.id) {
			case 'web':
				return { name: m.settings_ports_web_name(), purpose: m.settings_ports_web_purpose(), who: m.settings_ports_web_who(), condition: m.settings_ports_web_condition() };
			case 'agents':
				return { name: m.settings_ports_agents_name(), purpose: m.settings_ports_agents_purpose(), who: m.settings_ports_agents_who(), condition: m.settings_ports_agents_condition() };
			case 'syslog_udp':
				return { name: m.settings_ports_syslog_udp_name(), purpose: m.settings_ports_syslog_purpose(), who: m.settings_ports_syslog_who() };
			case 'syslog_tcp':
				return { name: m.settings_ports_syslog_tcp_name(), purpose: m.settings_ports_syslog_purpose(), who: m.settings_ports_syslog_who() };
			case 'traps':
				return { name: m.settings_ports_traps_name(), purpose: m.settings_ports_traps_purpose(), who: m.settings_ports_traps_who() };
			case 'metrics_store':
				return { name: m.settings_ports_store_metrics_name(), purpose: m.settings_ports_store_purpose(), who: m.settings_ports_store_who() };
			case 'long_store':
				return { name: m.settings_ports_store_long_name(), purpose: m.settings_ports_store_purpose(), who: m.settings_ports_store_who() };
			case 'logs_store':
				return { name: m.settings_ports_store_logs_name(), purpose: m.settings_ports_store_purpose(), who: m.settings_ports_store_who() };
			case 'relay_gateway':
				return { name: m.settings_ports_relay_name({ name: port.subject ?? '' }), purpose: m.settings_ports_relay_purpose(), who: m.settings_ports_relay_who() };
			default:
				return { name: port.id, purpose: '', who: '' };
		}
	}

	const EXPOSURE_TONE: Record<PortExposure, Tone> = { lan: 'info', wan_conditional: 'advisory', never: 'warning' };
	const STATUS_TONE: Record<PortStatus, Tone> = { listening: 'signal', off: 'ghost', local: 'ghost' };

	function exposureLabel(exposure: PortExposure): string {
		return exposure === 'lan' ? m.settings_ports_exposure_lan() : exposure === 'wan_conditional' ? m.settings_ports_exposure_wan() : m.settings_ports_exposure_never();
	}
	function statusLabel(status: PortStatus): string {
		return status === 'listening' ? m.settings_ports_status_listening() : status === 'off' ? m.settings_ports_status_off() : m.settings_ports_status_local();
	}
</script>

<Panel id="ports" title={m.settings_ports_title()} description={m.settings_ports_description()}>
	{#if loadError}
		<ErrorNotice error={loadError} title={m.settings_ports_load_error()} />
	{:else if !data}
		<Skeleton class="h-32" />
	{:else}
		<div class="grid gap-5">
			<p class="flex items-start gap-2 text-sm text-ink">
				<Lock class="mt-0.5 size-4 shrink-0 text-signal-ink" aria-hidden="true" />
				<span class="min-w-0 max-w-prose">{m.settings_ports_https_note()}</span>
			</p>

			<ul class="grid gap-3">
				{#each data.ports as port (`${port.id}-${port.protocol}-${port.port}-${port.subject ?? ''}`)}
					{@const text = textOf(port)}
					<li class={`grid gap-2 rounded-[var(--radius-card)] border border-line px-3 py-3 ${port.status === 'listening' ? 'bg-surface' : 'bg-surface-2'}`}>
						<div class="flex flex-wrap items-center gap-x-3 gap-y-2">
							<span class="tnum font-mono text-sm font-semibold text-ink">{port.host_port}/{port.protocol}</span>
							<span class="min-w-0 flex-1 text-sm font-semibold break-words text-ink">{text.name}</span>
						</div>
						<div class="flex flex-wrap items-center gap-2">
							<Plate tone={EXPOSURE_TONE[port.exposure]} label={exposureLabel(port.exposure)} size="md" bare>
								{#if port.exposure === 'lan'}
									<Network class="size-3.5 shrink-0" aria-hidden="true" />
								{:else if port.exposure === 'wan_conditional'}
									<Globe class="size-3.5 shrink-0" aria-hidden="true" />
								{:else}
									<ShieldOff class="size-3.5 shrink-0" aria-hidden="true" />
								{/if}
								{exposureLabel(port.exposure)}
							</Plate>
							<Plate tone={STATUS_TONE[port.status]} label={statusLabel(port.status)} size="md" />
						</div>
						{#if text.purpose}<p class="max-w-prose text-sm text-ink">{text.purpose}</p>{/if}
						{#if text.who || text.condition}
							<dl class="grid gap-x-4 gap-y-1 text-sm sm:grid-cols-[auto_1fr]">
								{#if text.who}
									<dt class="text-ink-2">{m.settings_ports_who_label()}</dt>
									<dd class="min-w-0 break-words text-ink">{text.who}</dd>
								{/if}
								{#if text.condition}
									<dt class="text-ink-2">{m.settings_ports_condition_label()}</dt>
									<dd class="min-w-0 break-words text-ink">{text.condition}</dd>
								{/if}
							</dl>
						{/if}
						{#if port.on_this_host && port.bind}
							<p class="text-xs text-ink-2">
								{m.settings_ports_listens_on({ bind: `${port.bind}:${port.port}` })}
								{#if port.host_port !== port.port}
									· {m.settings_ports_mapped({ host: String(port.host_port), port: String(port.port) })}
								{/if}
							</p>
						{/if}
					</li>
				{/each}
			</ul>

			<div class="grid gap-2">
				<h4 class="text-sm font-semibold text-ink">{m.settings_ports_compose_title()}</h4>
				<p class="max-w-prose text-sm text-ink-2">{m.settings_ports_compose_help()}</p>
				<CopyBlock value={data.compose} />
			</div>
			<div class="grid gap-2">
				<h4 class="text-sm font-semibold text-ink">{m.settings_ports_ufw_title()}</h4>
				<p class="max-w-prose text-sm text-ink-2">{m.settings_ports_ufw_help()}</p>
				<CopyBlock value={data.ufw} />
			</div>
		</div>
	{/if}
</Panel>
