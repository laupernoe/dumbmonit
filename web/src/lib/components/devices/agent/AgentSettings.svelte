<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * Everything that changes an agent, gathered in the monitor's settings: its
	 * relay mode, log collection, the services to watch on its machine, and the
	 * re-enrolment window that binds it again to this registration.
	 *
	 * The data page only keeps a read-only summary (`AgentPanel`).
	 */
	import { ShieldCheck, ShieldAlert } from 'lucide-svelte';
	import { allowAgentRebind, getAgentHost, type AgentHost, type Target } from '#lib/api/index.js';
	import { formatDateTime } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, CopyBlock, ErrorNotice, Panel, Plate } from '#lib/ui/index.js';
	import RelayPanel from '#lib/components/devices/relay/RelayPanel.svelte';
	import LogsPanel from '#lib/components/devices/agent/LogsPanel.svelte';
	import SentinelPanel from '#lib/components/devices/agent/SentinelPanel.svelte';
	import AgentModulesPanel from './AgentModulesPanel.svelte';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let host = $state<AgentHost | null>(null);
	let rebinding = $state(false);
	let rebindError = $state<unknown>(null);
	let reloads = $state(0);

	$effect(() => {
		const id = target.id;
		// Read so the effect re-runs after a re-enrolment window is opened.
		reloads;
		const controller = new AbortController();
		getAgentHost(id, controller.signal)
			.then((agent) => (host = agent))
			.catch(() => (host = null));
		return () => controller.abort();
	});

	const unbound = $derived(!!host && host.binding !== 'bound');
	/** A re-enrolment window that has not run out yet. */
	const window_ = $derived.by(() => {
		if (!host?.rebind_until) return null;
		return new Date(host.rebind_until + 'Z') > new Date() ? host.rebind_until : null;
	});

	/** Commands to run on the machine itself when the agent does not report. */
	const help = $derived.by(() => {
		const os = (host?.os ?? '').toLowerCase();
		if (os.includes('win')) {
			return {
				sudo: false,
				items: [
					{ label: m.devices_agent_help_status(), cmd: 'Get-Service DumbMonitAgent' },
					{ label: m.devices_agent_help_restart(), cmd: 'Restart-Service DumbMonitAgent' },
					{ label: m.devices_agent_help_logs(), cmd: "Get-WinEvent -FilterHashtable @{ ProviderName = 'DumbMonitAgent' } -MaxEvents 30" }
				]
			};
		}
		if (os.includes('freebsd')) {
			return {
				sudo: true,
				items: [
					{ label: m.devices_agent_help_status(), cmd: 'service dumbmonit_agent status' },
					{ label: m.devices_agent_help_restart(), cmd: 'service dumbmonit_agent restart' },
					{ label: m.devices_agent_help_logs(), cmd: 'tail -n 50 /var/log/dumbmonit-agent.log' }
				]
			};
		}
		return {
			sudo: true,
			items: [
				{ label: m.devices_agent_help_status(), cmd: 'systemctl status dumbmonit-agent' },
				{ label: m.devices_agent_help_restart(), cmd: 'systemctl restart dumbmonit-agent' },
				{ label: m.devices_agent_help_logs(), cmd: 'journalctl -u dumbmonit-agent -n 50 --no-pager' }
			]
		};
	});

	async function rebind(): Promise<boolean> {
		rebinding = true;
		rebindError = null;
		try {
			await allowAgentRebind(target.id);
			reloads += 1;
			return true;
		} catch (cause) {
			rebindError = cause;
			return false;
		} finally {
			rebinding = false;
		}
	}
</script>

{#if auth.isAdmin}
	<div class="flex flex-col gap-6">
		<h2 class="text-base font-semibold tracking-tight text-ink">{m.devices_agent_settings_title()}</h2>

		<RelayPanel {target} />

		<!-- Log collection, off until an administrator turns it on. -->
		<LogsPanel {target} />

		<!-- Sentinel mode: this machine watches DumbMonit itself, off until set up. -->
		<SentinelPanel {target} />

		<!-- Services the agent found (Docker, ZFS, tunnels…): which ones to watch. Each is a device of its own under this machine. -->
		<AgentModulesPanel {target} />

		{#if host}
			<Panel title={m.devices_agent_binding()}>
				{#snippet aside()}
					<Plate tone={unbound ? 'warning' : 'signal'} label={unbound ? m.devices_agent_unbound() : m.devices_agent_bound()} />
				{/snippet}
				<p class="flex items-start gap-2 text-sm text-ink-2">
					{#if unbound}
						<ShieldAlert class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
						<span>{m.devices_agent_unbound_detail()}</span>
					{:else}
						<ShieldCheck class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
						<span>{m.devices_agent_bound_detail()}</span>
					{/if}
				</p>
				{#if unbound && !host.binding_supported}
					<p class="mt-2 text-sm text-ink-2">{m.devices_agent_too_old({ version: host.agent_version })}</p>
				{/if}
				{#if window_}
					<p class="mt-3 rounded-[var(--radius-card)] border border-advisory/40 bg-surface px-3 py-2 text-sm text-ink">
						{m.devices_agent_window_open({ until: formatDateTime(window_) })}
					</p>
				{:else}
					<div class="mt-3 flex flex-wrap items-center gap-3">
						<Button size="sm" variant="secondary" loading={rebinding} onclick={rebind}>{m.devices_agent_allow_rebind()}</Button>
						<p class="min-w-0 flex-1 text-sm text-ink-2">
							{#if unbound}
								{m.devices_agent_rebind_hint_unbound()}
							{:else}
								{m.devices_agent_rebind_hint_bound()}
							{/if}
						</p>
					</div>
				{/if}
				{#if rebindError}
					<ErrorNotice error={rebindError} title={m.devices_agent_err_rebind()} class="mt-3" />
				{/if}
				<div class="mt-4 grid gap-2 border-t border-line pt-3">
					<p class="text-sm font-semibold text-ink">{m.devices_agent_help_title()}</p>
					<p class="text-sm text-ink-2">{m.devices_agent_help_intro()}</p>
					{#each help.items as item (item.cmd)}
						<div class="grid gap-1">
							<span class="text-[0.8125rem] text-ink-2">{item.label}</span>
							<CopyBlock value={item.cmd} sudo={help.sudo} />
						</div>
					{/each}
				</div>
			</Panel>
		{/if}
	</div>
{/if}
