<script lang="ts">
	/**
	 * The jobs x days matrix of one backup integration, read from its stored
	 * series over the last thirty days (see `history.ts` for how a day is
	 * rebuilt from them). Renders nothing until the target has stored some, so
	 * a device without the series shows no empty frame.
	 */
	import { untrack } from 'svelte';
	import { queryRange, type Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import BackupMatrix from './BackupMatrix.svelte';
	import { HISTORY_DAYS, SOURCES, deriveJob, groupSeries, metricNames, type MatrixJob } from './history';

	interface Props {
		target: Target;
		/** Key of `SOURCES`: `veeam`, `duplicati`, `kopia`, `abb`, `hyperbackup` or `plakar`. */
		source: string;
	}

	let { target, source }: Props = $props();

	const spec = $derived(SOURCES[source] ?? null);

	let jobs = $state<MatrixJob[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		const current = spec;
		if (!current) return;
		error = null;
		try {
			const end = Date.now();
			const series = await queryRange(
				{
					query: `{__name__=~"${metricNames(current).join('|')}", target="${target.id}"}`,
					start: end - (HISTORY_DAYS + 1) * 86_400_000,
					end,
					// Half an hour keeps thirty days under the server's point cap.
					step: 1800
				},
				signal
			);
			const nowSeconds = end / 1000;
			const offsetMinutes = -new Date(end).getTimezoneOffset();
			jobs = groupSeries(series, current).map((job) => deriveJob(job, { nowSeconds, offsetMinutes }));
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void target.id;
		void spec;
		loading = true;
		jobs = [];
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 300_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});
</script>

{#if spec && !loading && (error || jobs.length > 0)}
	<Panel title={m.backup_history_title()} description={m.backup_history_description({ days: HISTORY_DAYS })} padded={false} class="rise-in">
		{#if error}
			<div class="px-5 py-4">
				<ErrorNotice {error} title={m.backup_history_err()} onretry={() => void load()} />
			</div>
		{:else}
			<BackupMatrix {jobs} />
		{/if}
	</Panel>
{/if}
