<script lang="ts">
	/**
	 * Log collection of an agent device, switched from here (administrators):
	 * the server sends the setting to the agent in its next acknowledgement and
	 * the agent applies it on the fly. The lines land in the embedded log store
	 * and are searched on the Logs page. Off until someone turns it on.
	 */
	import { ScrollText } from 'lucide-svelte';
	import { getAgentLogs, setAgentLogs } from '#lib/api/storage.js';
	import type { AgentLogs, Target } from '#lib/api/types.js';
	import { formatRelative } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, ErrorNotice, Field, Panel, Plate, Toggle } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	type Level = AgentLogs['min_level'];

	let current = $state<AgentLogs | null>(null);
	let enabled = $state(false);
	let level = $state<Level>('warn');
	let journald = $state(true);
	let files = $state('');
	let eventlog = $state('');
	let saving = $state(false);
	let saved = $state(false);
	let error = $state<unknown>(null);

	const LEVELS: { id: Level; label: () => string }[] = [
		{ id: 'error', label: () => m.logsearch_level_error() },
		{ id: 'warn', label: () => m.logsearch_level_warn() },
		{ id: 'info', label: () => m.logsearch_level_info() },
		{ id: 'debug', label: () => m.logsearch_level_debug() }
	];

	function adopt(value: AgentLogs) {
		current = value;
		enabled = value.enabled;
		level = value.min_level;
		journald = value.journald;
		files = value.files.join('\n');
		eventlog = value.eventlog.join('\n');
	}

	const lines = (text: string) =>
		text
			.split('\n')
			.map((line) => line.trim())
			.filter(Boolean);

	$effect(() => {
		if (!auth.isAdmin) return;
		const controller = new AbortController();
		getAgentLogs(target.id, controller.signal)
			.then(adopt)
			.catch(() => (current = null));
		return () => controller.abort();
	});

	const dirty = $derived(
		current !== null &&
			(enabled !== current.enabled ||
				level !== current.min_level ||
				journald !== current.journald ||
				JSON.stringify(lines(files)) !== JSON.stringify(current.files) ||
				JSON.stringify(lines(eventlog)) !== JSON.stringify(current.eventlog))
	);

	async function save(event: SubmitEvent) {
		event.preventDefault();
		if (saving) return;
		saving = true;
		saved = false;
		error = null;
		try {
			adopt(await setAgentLogs(target.id, { enabled, min_level: level, journald, files: lines(files), eventlog: lines(eventlog) }));
			saved = true;
		} catch (cause) {
			error = cause;
		} finally {
			saving = false;
		}
	}
</script>

{#if current && auth.isAdmin}
	<Panel title={m.agentlogs_title()}>
		{#snippet aside()}
			<Plate tone={current?.enabled ? 'signal' : 'ghost'} label={current?.enabled ? m.agentlogs_on() : m.logsearch_inputs_off()} />
			<Button variant="ghost" size="sm" href={`/logs?target=${target.id}&source=agent`}>
				<ScrollText class="size-4" aria-hidden="true" />
				{m.agentlogs_view()}
			</Button>
		{/snippet}

		<form class="grid gap-4" onsubmit={save}>
			<div class="flex items-start gap-3">
				<Toggle id="agent-logs-switch" bind:checked={enabled} label={m.agentlogs_switch()} disabled={saving} />
				<div class="min-w-0">
					<label for="agent-logs-switch" class="text-sm font-semibold text-ink">{m.agentlogs_switch()}</label>
					<p class="text-sm text-ink-2">{m.agentlogs_switch_help()}</p>
				</div>
			</div>

			{#if enabled}
				<div class="grid gap-4 sm:grid-cols-2">
					<Field label={m.logsearch_level()} for="agent-logs-level">
						<select id="agent-logs-level" class="input" bind:value={level} disabled={saving}>
							{#each LEVELS as option (option.id)}<option value={option.id}>{option.label()}</option>{/each}
						</select>
					</Field>
					<div class="self-end">
						<label class="inline-flex min-h-10 items-center gap-2 text-sm text-ink">
							<input type="checkbox" class="size-4 accent-[var(--c-signal)]" bind:checked={journald} disabled={saving} />
							{m.agentlogs_journald()}
						</label>
						<p class="text-[0.8125rem] text-ink-2">{m.agentlogs_journald_help()}</p>
					</div>
				</div>
				<Field label={m.agentlogs_files()} for="agent-logs-files" help={m.agentlogs_files_help()}>
					<textarea
						id="agent-logs-files"
						class="input min-h-24 font-mono text-[0.8125rem]"
						rows="3"
						spellcheck="false"
						autocomplete="off"
						placeholder="/var/log/nginx/*.log"
						disabled={saving}
						bind:value={files}
					></textarea>
				</Field>
				<Field label={m.agentlogs_eventlog()} for="agent-logs-eventlog" help={m.agentlogs_eventlog_help()}>
					<textarea
						id="agent-logs-eventlog"
						class="input min-h-16 font-mono text-[0.8125rem]"
						rows="2"
						spellcheck="false"
						autocomplete="off"
						placeholder="System"
						disabled={saving}
						bind:value={eventlog}
					></textarea>
				</Field>
			{/if}

			<div class="text-sm text-ink-2">
				{#if current.last_received_at}
					<p>{m.agentlogs_last({ when: formatRelative(current.last_received_at) })} · <span class="tnum">{m.agentlogs_total({ count: current.received_total })}</span></p>
				{:else if current.enabled}
					<p>{m.agentlogs_never()}</p>
				{/if}
				{#if current.enabled && !current.store_running}
					<p class="mt-1 text-warning-ink">{m.agentlogs_store_off()}</p>
				{/if}
				<p class="mt-1">{m.agentlogs_local_hint({ env: 'DUMBMONIT_AGENT_LOGS=off' })}</p>
			</div>

			{#if saved && !dirty}
				<p class="text-sm text-ink-2" role="status">{m.agentlogs_pending()}</p>
			{/if}
			{#if error}
				<ErrorNotice {error} title={m.agentlogs_save_error()} />
			{/if}
			<div>
				<Button type="submit" variant="secondary" size="sm" loading={saving} disabled={!dirty}>{m.agentlogs_save()}</Button>
			</div>
		</form>
	</Panel>
{/if}
