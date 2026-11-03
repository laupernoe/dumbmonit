<script lang="ts">
	/**
	 * Deleting an agent's monitor does not remove the agent from the machine: it
	 * keeps running and asks to enrol again. When the agent is online and runs
	 * commands, it can remove itself (`agent.uninstall`): the monitor is deleted
	 * once it confirmed. Otherwise, or when that fails, this shows how to take it
	 * off by hand (commands matched to the machine's system), then deletes the
	 * monitor.
	 */
	import { onMount } from 'svelte';
	import { cancelAgentCommand, getAgentCommand, getAgentHost, uninstallAgent } from '#lib/api/index.js';
	import type { AgentHost, Target } from '#lib/api/index.js';
	import { parseServerDate } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Button, CopyBlock, ErrorNotice, Led, Panel } from '#lib/ui/index.js';

	interface Props {
		target: Target;
		deleting: boolean;
		ondelete: () => void;
		oncancel: () => void;
	}
	let { target, deleting, ondelete, oncancel }: Props = $props();

	/** The server lets a command wait 10 minutes; the page does not wait that long. */
	const WAIT_MS = 120_000;
	const POLL_MS = 2_000;

	let host = $state<AgentHost | null>(null);
	const os = $derived((host?.os ?? '').toLowerCase());

	type Phase = 'idle' | 'sending' | 'queued' | 'running' | 'done' | 'failed' | 'timeout';
	let phase = $state<Phase>('idle');
	let commandId = $state<number | null>(null);
	let reason = $state('');
	let sendError = $state<unknown>(null);

	let alive = true;
	let timer: ReturnType<typeof setTimeout> | undefined;
	let controller: AbortController | undefined;

	onMount(() => {
		const loading = new AbortController();
		getAgentHost(target.id, loading.signal)
			.then((found) => (host = found))
			.catch(() => {});
		return () => {
			alive = false;
			loading.abort();
			controller?.abort();
			clearTimeout(timer);
		};
	});

	/**
	 * Online: seen within three report intervals (at least 3 minutes, at most
	 * the 10 minutes a command waits for its agent).
	 */
	const online = $derived.by(() => {
		const seen = parseServerDate(host?.last_seen_at)?.getTime();
		if (seen === undefined) return false;
		const limit = Math.min(600, Math.max(180, 3 * target.interval_secs)) * 1000;
		return Date.now() - seen <= limit;
	});

	/**
	 * Offered when the agent runs commands and reports. An agent in a container,
	 * or not installed by the install scripts, refuses and says why: the manual
	 * commands then stay on screen.
	 */
	const automatic = $derived(host !== null && host.commands_supported && online);
	/** The request is on its way to the agent, or being carried out. */
	const waiting = $derived(phase === 'sending' || phase === 'queued' || phase === 'running');
	const gaveUp = $derived(phase === 'failed' || phase === 'timeout');
	/** Last status read, to pull a request no agent took back on timeout. */
	let lastStatus: string | undefined;

	async function removeAutomatically() {
		phase = 'sending';
		reason = '';
		sendError = null;
		try {
			const command = await uninstallAgent(target.id);
			if (!alive) return;
			commandId = command.id;
			phase = 'queued';
			follow(Date.now() + WAIT_MS);
		} catch (cause) {
			if (!alive) return;
			sendError = cause;
			phase = 'failed';
		}
	}

	function follow(deadline: number) {
		timer = setTimeout(async () => {
			if (!alive || commandId === null) return;
			controller = new AbortController();
			try {
				const command = await getAgentCommand(target.id, commandId, controller.signal);
				if (!alive) return;
				const status = command?.status;
				lastStatus = status;
				if (status === 'done') {
					phase = 'done';
					ondelete();
					return;
				}
				if (status === 'failed' || status === 'cancelled' || status === 'expired') {
					phase = 'failed';
					// The agent's log ends with the line that explains.
					const lines = (command?.result ?? '').trim().split('\n');
					reason = lines[lines.length - 1].trim();
					return;
				}
				if (status === 'running') phase = 'running';
			} catch {
				// A missed poll is not a verdict: the next one may answer.
				if (!alive) return;
			}
			if (Date.now() >= deadline) {
				phase = 'timeout';
				// Not picked up: withdrawn, so the agent does not leave later on
				// while the manual steps are on screen.
				if (lastStatus !== 'running' && commandId !== null) {
					cancelAgentCommand(target.id, commandId).catch(() => {});
				}
				return;
			}
			follow(deadline);
		}, POLL_MS);
	}

	/** Pulls a request the agent has not picked up yet back, then closes. */
	async function cancel() {
		clearTimeout(timer);
		controller?.abort();
		if (phase === 'queued' && commandId !== null) {
			await cancelAgentCommand(target.id, commandId).catch(() => {});
		}
		oncancel();
	}

	const commands = $derived.by(() => {
		if (os.includes('win')) {
			return {
				sudo: false,
				list: ['Stop-Service DumbMonitAgent -Force; sc.exe delete DumbMonitAgent; Remove-Item -Recurse -Force "$env:ProgramFiles\\DumbMonit", "$env:ProgramData\\DumbMonit"']
			};
		}
		if (os.includes('freebsd')) {
			return {
				sudo: true,
				list: ['service dumbmonit_agent onestop; sysrc -x dumbmonit_agent_enable; rm -rf /usr/local/etc/rc.d/dumbmonit_agent /usr/local/lib/dumbmonit-agent /usr/local/etc/dumbmonit']
			};
		}
		if (os.includes('darwin') || os.includes('mac')) {
			return {
				sudo: true,
				list: ['launchctl bootout system/com.dumbmonit.agent; rm -rf /Library/LaunchDaemons/com.dumbmonit.agent.plist /usr/local/lib/dumbmonit-agent /usr/local/etc/dumbmonit']
			};
		}
		return {
			sudo: true,
			list: [
				'systemctl disable --now dumbmonit-agent && rm -rf /etc/systemd/system/dumbmonit-agent.service /usr/local/bin/dumbmonit-agent /usr/local/lib/dumbmonit-agent /etc/dumbmonit && systemctl daemon-reload',
				'docker rm -f dumbmonit-agent && docker volume rm dumbmonit-agent'
			]
		};
	});

	const progress = $derived.by(() => {
		switch (phase) {
			case 'sending':
			case 'queued':
				return m.devices_agent_remove_waiting();
			case 'running':
				return m.devices_agent_remove_running();
			case 'done':
				return m.devices_agent_remove_done();
			default:
				return '';
		}
	});
</script>

<Panel title={m.devices_agent_remove_title()} description={automatic ? m.devices_agent_remove_auto_text() : m.devices_agent_remove_text()} class="mt-4">
	<div class="grid gap-3">
		{#if automatic && !gaveUp}
			<div class="flex flex-wrap items-center gap-3">
				<Button variant="primary" loading={waiting} disabled={waiting || phase === 'done'} onclick={removeAutomatically}>{m.devices_agent_remove_auto()}</Button>
				{#if waiting}
					<!-- Only a request still waiting for its agent can be pulled back. -->
					<Button variant="ghost" disabled={phase !== 'queued'} onclick={cancel}>{m.devices_agent_remove_cancel()}</Button>
				{/if}
			</div>
			{#if progress}
				<p class="flex items-center gap-2 text-sm text-ink-2" role="status" aria-live="polite">
					<Led tone={phase === 'done' ? 'signal' : 'info'} size="sm" />
					<span>{progress}</span>
				</p>
			{/if}
			{#if phase === 'idle'}
				<p class="text-sm text-ink-2">{m.devices_agent_remove_manual()}</p>
			{/if}
		{/if}
		{#if gaveUp}
			{#if sendError}
				<ErrorNotice error={sendError} title={m.devices_agent_remove_failed_title()} />
			{:else}
				<p class="flex items-start gap-2 text-sm text-ink" role="alert">
					<Led tone="warning" size="sm" class="mt-1.5" />
					<span class="min-w-0 break-words">
						{#if phase === 'timeout'}
							{m.devices_agent_remove_timeout()}
						{:else if reason}
							{m.devices_agent_remove_failed({ reason })}
						{:else}
							{m.devices_agent_remove_failed_title()}
						{/if}
					</span>
				</p>
			{/if}
		{/if}
		{#if !waiting && phase !== 'done'}
			{#each commands.list as cmd (cmd)}
				<CopyBlock value={cmd} sudo={commands.sudo} />
			{/each}
			<p class="text-sm text-ink-2">{m.devices_agent_remove_docker()}</p>
		{/if}
		{#if !waiting}
			<!-- After the agent left, this is also the retry if deleting the monitor failed. -->
			<div class="flex flex-wrap items-center gap-3">
				<Button variant="danger" loading={deleting} onclick={ondelete}>
					{gaveUp ? m.devices_agent_remove_delete_anyway() : m.devices_agent_remove_delete()}
				</Button>
				<Button variant="ghost" onclick={cancel}>{m.devices_agent_remove_cancel()}</Button>
			</div>
		{/if}
	</div>
</Panel>
