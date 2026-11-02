<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * Databases the agent found on its machine (PostgreSQL, MySQL/MariaDB,
	 * SQL Server, HFSQL, MongoDB), one line each: whether the service runs and
	 * how old the newest backup file is. The agent only reads services, ports,
	 * the registry and backup folders; no credentials, no SQL.
	 *
	 * Each line carries a plate AND a sentence ("Running · last backup 3 h ago"),
	 * so the state never rests on colour alone. A database whose backup folder
	 * is unknown shows a calm advisory, not a warning: nothing is known to be
	 * wrong. The built-in rule fires on the agent's own `overdue` verdict.
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate, type Tone } from '#lib/ui/index.js';
	import { formatAge, formatBytes } from '../docker/api';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	interface Database {
		engine: string;
		instance: string;
		version: string | null;
		running: boolean | null;
		port: number | null;
		/** `null`: the agent knows no backup folder for this database. */
		overdue: boolean | null;
		age: number | null;
		size: number | null;
	}

	const ENGINES: Record<string, string> = {
		postgresql: 'PostgreSQL',
		mysql: 'MySQL / MariaDB',
		sqlserver: 'SQL Server',
		hfsql: 'HFSQL',
		mongodb: 'MongoDB'
	};

	let databases = $state<Database[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	function backupText(d: Database): string {
		if (d.overdue === null) return m.devices_databases_backup_unknown();
		if (d.age === null) return m.devices_databases_backup_none();
		if (d.overdue) return m.devices_databases_backup_late({ age: formatAge(d.age) });
		return m.devices_databases_backup_ok({ age: formatAge(d.age) });
	}

	function plateOf(d: Database): { tone: Tone; label: string } {
		const state = d.running === false ? m.devices_databases_stopped() : m.devices_databases_running();
		const label = m.devices_databases_summary({ state, backup: backupText(d) });
		if (d.running === false || d.overdue === true) return { tone: 'warning', label };
		if (d.overdue === null) return { tone: 'advisory', label };
		return { tone: 'signal', label };
	}

	function fold(series: { metric: Record<string, string>; values: [number, string][] }[]): Database[] {
		const byKey = new Map<string, Database>();
		for (const serie of series) {
			const metric = serie.metric.__name__ ?? '';
			const value = Number(serie.values.at(-1)?.[1]);
			const engine = serie.metric.engine ?? '';
			const instance = serie.metric.instance ?? '';
			if (!Number.isFinite(value) || !engine || !instance) continue;
			const key = `${engine}:${instance}`;
			let d = byKey.get(key);
			if (!d) {
				d = { engine, instance, version: null, running: null, port: null, overdue: null, age: null, size: null };
				byKey.set(key, d);
			}
			if (metric.endsWith('_database_up')) d.running = value >= 1;
			else if (metric.endsWith('_database_port')) d.port = value;
			else if (metric.endsWith('_database_info')) d.version = serie.metric.version ?? null;
			else if (metric.endsWith('_database_backup_overdue')) d.overdue = value >= 1;
			else if (metric.endsWith('_database_backup_age_seconds')) d.age = value;
			else if (metric.endsWith('_database_backup_size_bytes')) d.size = value;
		}
		return [...byKey.values()].sort(
			(a, b) => a.engine.localeCompare(b.engine, 'en') || a.instance.localeCompare(b.instance, 'en')
		);
	}

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const series = await queryInstant(
				`{__name__=~"dumbmonit_agent_database_(up|port|info|backup_overdue|backup_age_seconds|backup_size_bytes)", target="${target.id}"}`,
				signal
			);
			databases = fold(series);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void target.id;
		loading = true;
		databases = [];
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const failing = $derived(databases.filter((d) => plateOf(d).tone === 'warning').length);
</script>

{#if !loading && (error || databases.length > 0)}
	<Panel title={m.devices_databases_title()} padded={false} class="rise-in">
		{#snippet aside()}
			{#if failing > 0}
				<Plate tone="warning" label={failing === 1 ? m.devices_databases_need_one() : m.devices_databases_need_other({ count: failing })} />
			{:else if databases.length > 0}
				<Plate tone="signal" label={m.devices_databases_fine()} />
			{/if}
		{/snippet}
		{#if error}
			<div class="px-4 py-4 sm:px-5">
				<ErrorNotice {error} title={m.devices_databases_err_title()} onretry={() => void load()} />
			</div>
		{:else}
			<ul class="divide-y divide-line">
				{#each databases as d (`${d.engine}:${d.instance}`)}
					{@const plate = plateOf(d)}
					<li class="flex flex-col gap-1 px-4 py-3 sm:px-5">
						<div class="flex flex-col gap-1 sm:flex-row sm:flex-wrap sm:items-center sm:gap-x-3">
							<Plate tone={plate.tone} label={plate.label} />
							<span class="min-w-0 text-sm font-semibold text-ink break-all">{ENGINES[d.engine] ?? d.engine}</span>
							<span class="min-w-0 text-sm text-ink-2 break-all">{d.instance}</span>
							{#if d.version}
								<span class="tnum text-sm text-ink-2">{m.devices_databases_version({ version: d.version })}</span>
							{/if}
							{#if d.port !== null}
								<span class="tnum text-sm text-ink-2">{m.devices_databases_port({ port: d.port })}</span>
							{/if}
							{#if d.size !== null}
								<span class="tnum text-sm text-ink-2">{m.devices_databases_backup_size({ size: formatBytes(d.size) })}</span>
							{/if}
						</div>
						{#if d.running === false}
							<p class="text-sm text-ink-2">{m.devices_databases_stopped_hint()}</p>
						{/if}
						{#if d.overdue === null}
							<p class="text-sm text-ink-2">{m.devices_databases_unknown_hint()}</p>
						{:else if d.overdue}
							<p class="text-sm text-warning-ink">{m.devices_databases_late_hint()}</p>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</Panel>
{/if}
