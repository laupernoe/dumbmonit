<script lang="ts">
	/**
	 * The state of a commercial appliance or service as its last probe left it
	 * (pfSense, Unraid, Veeam, Tailscale, FortiGate, Sophos), or of a Hyper-V
	 * host as its agent reads it: one sentence saying what is wrong, a few
	 * figures, then only the lists worth reading. Each kind folds its own
	 * `dumbmonit_<prefix>_*` series in `folds.ts`; this component only lays
	 * them out.
	 *
	 * Everything is read from the stored series: opening the page never calls
	 * the device. Renders nothing until a probe has stored something, so an
	 * agent without Hyper-V shows no empty box.
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type Target } from '$lib/api';
	import { ErrorNotice, Panel, Plate } from '$lib/ui';
	import ClientDevicesTable from '../ClientDevicesTable.svelte';
	import Figure from '../Figure.svelte';
	import { FOLDS, type ApplianceView } from './folds';

	/** Kinds whose devices page also shows the client-devices table. */
	const WITH_DEVICES = new Set(['tailscale']);

	interface Props {
		target: Target;
		/** The fold to use; the target's kind by default (`hyperv` on an agent). */
		variant?: string;
	}

	let { target, variant }: Props = $props();

	const fold = $derived(FOLDS[variant ?? target.kind] ?? null);

	let view = $state<ApplianceView | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		const current = fold;
		if (!current) return;
		error = null;
		try {
			const series = await queryInstant(`{__name__=~"dumbmonit_${current.prefix}_.+", target="${target.id}"}`, signal);
			view = series.length > 0 ? current.fold(series, Date.now() / 1000) : null;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void target.id;
		void fold;
		loading = true;
		view = null;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const VERDICT_INK = { signal: 'text-ink-2', advisory: 'text-advisory-ink', warning: 'text-warning-ink' };
</script>

{#if fold && !loading && (error || view)}
	<Panel title={view?.title ?? fold.title} description={view?.description ?? ''} padded={false} class="rise-in">
		{#snippet aside()}
			{#if view}<Plate tone={view.verdict.tone} label={view.verdict.tone === 'signal' ? 'Healthy' : 'Needs attention'} />{/if}
		{/snippet}
		{#if error}
			<div class="px-5 py-4">
				<ErrorNotice {error} title={`Could not load the ${fold.title.toLowerCase()}`} onretry={() => void load()} />
			</div>
		{:else if view}
			<p class={`px-5 pt-4 text-sm ${VERDICT_INK[view.verdict.tone]}`}>{view.verdict.text}</p>
			{#if view.figures.length > 0}
				<div class="grid grid-cols-2 gap-x-6 gap-y-2 px-5 pt-4 sm:grid-cols-4">
					{#each view.figures as f (f.label)}
						<Figure label={f.label} value={f.value} tone={f.tone ?? 'ink'} hint={f.hint} />
					{/each}
				</div>
			{/if}
			{#each view.sections as section (section.title)}
				{#if section.rows.length > 0}
					<section class="border-t border-line px-5 py-4">
						<h3 class="label-tape">{section.title}</h3>
						<ul class="mt-2 flex flex-col gap-2">
							{#each section.rows as row (row.key)}
								<li class="flex flex-col gap-1 sm:flex-row sm:flex-wrap sm:items-center sm:gap-x-3">
									<Plate tone={row.tone} label={row.plate} />
									<span class="min-w-0 text-sm font-semibold text-ink break-all">{row.name}</span>
									{#each row.details ?? [] as d (d)}<span class="tnum text-sm text-ink-2">{d}</span>{/each}
								</li>
							{/each}
						</ul>
						{#if section.more}<p class="mt-2 text-sm text-ink-3">{section.more}</p>{/if}
					</section>
				{/if}
			{/each}
		{/if}
	</Panel>
{/if}

{#if WITH_DEVICES.has(target.kind)}
	<ClientDevicesTable {target} />
{/if}
