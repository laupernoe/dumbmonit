<script lang="ts">
	/**
	 * restic and Borg repositories the agent was told to watch (`restic_repos`,
	 * `borg_repos` in agent.yaml): whether each one can be read, and the age of
	 * its last snapshot. The passwords stay on the machine; only the repository
	 * names reach the server.
	 *
	 * Without any declared repository the agent emits nothing and this renders
	 * nothing. Thresholds follow the built-in rule: past 48 hours without a
	 * snapshot the plate turns to a warning.
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate, type Tone } from '#lib/ui/index.js';
	import { formatAge, formatBytes } from '../docker/api';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	const DAY = 86_400;
	const TOO_OLD = 48 * 3600;

	interface RepoStat {
		tool: string;
		repo: string;
		reachable: boolean | null;
		age: number | null;
		bytes: number | null;
	}

	let repos = $state<RepoStat[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	function plateOf(r: RepoStat): { tone: Tone; label: string } {
		if (r.reachable === false) return { tone: 'warning', label: 'Cannot be read' };
		if (r.age === null) return { tone: 'ghost', label: 'No snapshot yet' };
		if (r.age < DAY) return { tone: 'signal', label: `Backed up ${formatAge(r.age)} ago` };
		if (r.age <= TOO_OLD) return { tone: 'advisory', label: `Last backup ${formatAge(r.age)} ago` };
		return { tone: 'warning', label: `No backup for ${formatAge(r.age)}` };
	}

	function fold(series: { metric: Record<string, string>; values: [number, string][] }[]): RepoStat[] {
		const byKey = new Map<string, RepoStat>();
		for (const serie of series) {
			const metric = serie.metric.__name__ ?? '';
			const value = Number(serie.values.at(-1)?.[1]);
			const tool = serie.metric.tool ?? '';
			const repo = serie.metric.repo ?? '';
			if (!Number.isFinite(value) || !tool || !repo) continue;
			const key = `${tool}:${repo}`;
			let r = byKey.get(key);
			if (!r) {
				r = { tool, repo, reachable: null, age: null, bytes: null };
				byKey.set(key, r);
			}
			if (metric.endsWith('_repo_reachable')) r.reachable = value >= 1;
			else if (metric.endsWith('_last_snapshot_age_seconds')) r.age = value;
			else if (metric.endsWith('_last_snapshot_bytes')) r.bytes = value;
		}
		return [...byKey.values()].sort((a, b) => a.tool.localeCompare(b.tool, 'en') || a.repo.localeCompare(b.repo, 'en'));
	}

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const series = await queryInstant(
				`{__name__=~"dumbmonit_agent_backup_repo_(reachable|last_snapshot_age_seconds|last_snapshot_bytes)", target="${target.id}"}`,
				signal
			);
			repos = fold(series);
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
		repos = [];
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const failing = $derived(repos.filter((r) => plateOf(r).tone === 'warning').length);
</script>

{#if !loading && (error || repos.length > 0)}
	<Panel title="Backup repositories" description="restic and Borg repositories declared in agent.yaml." padded={false} class="rise-in">
		{#snippet aside()}
			{#if failing > 0}
				<Plate tone="warning" label={failing === 1 ? '1 needs attention' : `${failing} need attention`} />
			{:else if repos.length > 0}
				<Plate tone="signal" label="Backups fine" />
			{/if}
		{/snippet}
		{#if error}
			<div class="px-5 py-4">
				<ErrorNotice {error} title="Could not load the repositories" onretry={() => void load()} />
			</div>
		{:else}
			<ul class="divide-y divide-line">
				{#each repos as r (`${r.tool}:${r.repo}`)}
					{@const plate = plateOf(r)}
					<li class="flex flex-col gap-1 px-5 py-3">
						<div class="flex flex-col gap-1 sm:flex-row sm:flex-wrap sm:items-center sm:gap-x-3">
							<Plate tone={plate.tone} label={plate.label} />
							<span class="min-w-0 text-sm font-semibold text-ink break-all">{r.repo}</span>
							<span class="text-sm text-ink-2">{r.tool === 'borg' ? 'Borg' : 'restic'}</span>
							{#if r.bytes !== null}
								<span class="tnum text-sm text-ink-2">{`${formatBytes(r.bytes)} in the last snapshot`}</span>
							{/if}
						</div>
						{#if r.reachable === false}
							<p class="text-sm text-ink-2">
								The agent could not open this repository. Its log says why (unreachable, password refused, binary missing):
								<span class="font-mono">journalctl -u dumbmonit-agent</span>.
							</p>
						{:else if r.age !== null && r.age > TOO_OLD}
							<p class="text-sm text-warning-ink">Check the job that runs the backups: nothing new for more than 48 hours.</p>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</Panel>
{/if}
