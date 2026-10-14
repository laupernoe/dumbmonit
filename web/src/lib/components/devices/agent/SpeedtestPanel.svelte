<script lang="ts">
	/**
	 * Last Internet speed test run by the agent (`dumbmonit_agent_speedtest_*`):
	 * download and upload throughput, latency and jitter, and how long ago it
	 * ran. Disabled by default — nothing is shown until `speedtest: true` and a
	 * download address are set in agent.yaml and a first test has completed.
	 */
	import { untrack } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { queryInstant, type Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate } from '#lib/ui/index.js';
	import { formatAge, formatBytes } from '../docker/api';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	interface SpeedtestStat {
		ok: boolean | null;
		age: number | null;
		downloadBps: number | null;
		uploadBps: number | null;
		latencyMs: number | null;
		jitterMs: number | null;
		minDownloadBps: number | null;
		minUploadBps: number | null;
		maxLatencyMs: number | null;
	}

	let stat = $state<SpeedtestStat | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	function fold(series: { metric: Record<string, string>; values: [number, string][] }[]): SpeedtestStat | null {
		if (series.length === 0) return null;
		const result: SpeedtestStat = {
			ok: null,
			age: null,
			downloadBps: null,
			uploadBps: null,
			latencyMs: null,
			jitterMs: null,
			minDownloadBps: null,
			minUploadBps: null,
			maxLatencyMs: null
		};
		for (const serie of series) {
			const metric = serie.metric.__name__ ?? '';
			const value = Number(serie.values.at(-1)?.[1]);
			if (!Number.isFinite(value)) continue;
			if (metric.endsWith('_ok')) result.ok = value >= 1;
			else if (metric.endsWith('_age_seconds')) result.age = value;
			else if (metric.endsWith('_download_bytes_per_second')) result.downloadBps = value;
			else if (metric.endsWith('_upload_bytes_per_second')) result.uploadBps = value;
			else if (metric.endsWith('_latency_ms')) result.latencyMs = value;
			else if (metric.endsWith('_jitter_ms')) result.jitterMs = value;
			else if (metric.endsWith('_download_min_bytes_per_second')) result.minDownloadBps = value;
			else if (metric.endsWith('_upload_min_bytes_per_second')) result.minUploadBps = value;
			else if (metric.endsWith('_max_latency_ms')) result.maxLatencyMs = value;
		}
		return result;
	}

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const series = await queryInstant(`{__name__=~"dumbmonit_agent_speedtest_.+", target="${target.id}"}`, signal);
			stat = fold(series);
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
		stat = null;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const belowThreshold = $derived(
		!!stat &&
			((stat.minDownloadBps !== null && stat.downloadBps !== null && stat.downloadBps < stat.minDownloadBps) ||
				(stat.minUploadBps !== null && stat.uploadBps !== null && stat.uploadBps < stat.minUploadBps) ||
				(stat.maxLatencyMs !== null && stat.latencyMs !== null && stat.latencyMs > stat.maxLatencyMs))
	);
</script>

{#if !loading && (error || stat)}
	<Panel title={m.devices_speedtest_title()} description={m.devices_speedtest_description()} class="rise-in">
		{#snippet aside()}
			{#if stat?.ok === false}
				<Plate tone="warning" label={m.devices_speedtest_failed()} />
			{:else if belowThreshold}
				<Plate tone="warning" label={m.devices_speedtest_below()} />
			{:else if stat?.ok}
				<Plate tone="signal" label={m.devices_speedtest_ok()} />
			{/if}
		{/snippet}
		{#if error}
			<ErrorNotice {error} title={m.devices_speedtest_err_title()} onretry={() => void load()} />
		{:else if stat}
			<dl class="grid gap-x-6 gap-y-2 text-sm sm:grid-cols-[auto_1fr]">
				<dt class="text-ink-2">{m.devices_speedtest_download()}</dt>
				<dd class="tnum text-ink">{stat.downloadBps !== null ? `${formatBytes(stat.downloadBps)}/s` : '—'}</dd>
				<dt class="text-ink-2">{m.devices_speedtest_upload()}</dt>
				<dd class="tnum text-ink">{stat.uploadBps !== null ? `${formatBytes(stat.uploadBps)}/s` : '—'}</dd>
				<dt class="text-ink-2">{m.devices_speedtest_latency()}</dt>
				<dd class="tnum text-ink">{stat.latencyMs !== null ? `${stat.latencyMs.toFixed(0)} ms` : '—'}</dd>
				<dt class="text-ink-2">{m.devices_speedtest_jitter()}</dt>
				<dd class="tnum text-ink">{stat.jitterMs !== null ? `${stat.jitterMs.toFixed(0)} ms` : '—'}</dd>
				<dt class="text-ink-2">{m.devices_speedtest_last_run()}</dt>
				<dd class="text-ink">{stat.age !== null ? m.devices_speedtest_ago({ age: formatAge(stat.age) }) : '—'}</dd>
			</dl>
		{/if}
	</Panel>
{/if}
