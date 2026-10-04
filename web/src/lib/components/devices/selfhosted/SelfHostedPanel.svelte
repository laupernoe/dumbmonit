<script lang="ts">
	/**
	 * A self-hosted application (Nextcloud, Immich, Paperless-ngx, Jellyfin,
	 * Plex) as its own API describes it: first what is wrong, then the
	 * figures, then the named details (apps to update, queues, components,
	 * failed tasks). Read from the latest stored measurement and refreshed
	 * every minute — opening the page never calls the application.
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type Target } from '$lib/api';
	import { ErrorNotice, Panel, Plate, Skeleton, type Tone } from '$lib/ui';
	import ClientDevicesTable from '../ClientDevicesTable.svelte';
	import Figure from '../Figure.svelte';
	import { TITLES, buildView, overall, type AppView, type CheckState } from './view';

	/** Kinds whose devices page also shows the client-devices table. */
	const WITH_DEVICES = new Set(['immich']);

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let view = $state<AppView | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const series = await queryInstant(`{__name__=~"dumbmonit_${target.kind}_.+", target="${target.id}"}`, signal);
			view = buildView(target.kind, series);
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
		view = null;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	function tone(level: CheckState | null | undefined): Tone {
		if (level === 'warning') return 'warning';
		if (level === 'advisory') return 'advisory';
		if (level === 'ok') return 'signal';
		return 'ghost';
	}

	function word(level: CheckState | null | undefined): string {
		if (level === 'warning') return 'Failing';
		if (level === 'advisory') return 'Attention';
		if (level === 'ok') return 'OK';
		return 'Unknown';
	}

	const title = $derived(TITLES[target.kind] ?? 'Application');
	const description = $derived(view?.version ? `Version ${view.version}` : undefined);
	const verdict = $derived(view ? overall(view) : null);
</script>

{#if error}
	<Panel {title} class="rise-in">
		<ErrorNotice {error} title="Could not load the application's health" onretry={() => void load()} />
	</Panel>
{:else if loading}
	<Panel {title} padded={false} class="rise-in">
		<div class="flex flex-col gap-3 px-5 py-4" aria-busy="true" aria-label="Loading the application's health">
			<Skeleton class="h-10 w-full" rows={3} />
		</div>
	</Panel>
{:else if view}
	<div class="flex flex-col gap-6">
		<Panel {title} {description} padded={false} class="rise-in">
			{#snippet aside()}
				{#if verdict}
					<Plate tone={tone(verdict)} label={word(verdict)} size="md" />
				{/if}
			{/snippet}
			{#if !view.measured}
				<p class="px-5 py-4 text-sm text-ink-2">Waiting for the first probe: the application has not been read yet.</p>
			{:else}
				<div class="flex flex-col divide-y divide-line">
					{#if view.checks.length > 0}
						<ul class="flex flex-col gap-2 px-5 py-4">
							{#each view.checks as item, i (i)}
								<li class="flex flex-col gap-1 sm:flex-row sm:items-baseline sm:gap-3">
									<span class="shrink-0 sm:w-24">
										<Plate tone={tone(item.state)} label={word(item.state)} />
									</span>
									<span class="min-w-0 text-sm text-ink"><span class="font-semibold">{item.label}.</span>{' '}{item.detail}</span>
								</li>
							{/each}
						</ul>
					{/if}
					{#if view.figures.some((f) => f.value !== null)}
						<div class="grid grid-cols-2 gap-x-6 gap-y-2 px-5 pt-4 pb-2 sm:grid-cols-4">
							{#each view.figures.filter((f) => f.value !== null) as figure (figure.label)}
								<Figure label={figure.label} value={figure.value} />
							{/each}
						</div>
					{/if}
				</div>
			{/if}
		</Panel>

		{#if view.measured && view.breakdowns.length > 0}
			<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
				{#each view.breakdowns as breakdown (breakdown.title)}
					<Panel title={breakdown.title} description={breakdown.note} padded={false} class="rise-in">
						<ul class="divide-y divide-line">
							{#each breakdown.rows as row, i (i)}
								<li class="flex flex-col gap-1 px-5 py-2.5 sm:flex-row sm:items-center sm:gap-3">
									<p class="min-w-0 flex-1 truncate text-sm text-ink" title={row.label}>{row.label}</p>
									{#if row.value}
										<span class="min-w-0 text-sm text-ink-2 break-words sm:text-right">{row.value}</span>
									{/if}
									{#if row.state}
										<Plate tone={tone(row.state)} label={word(row.state)} />
									{/if}
								</li>
							{/each}
						</ul>
					</Panel>
				{/each}
			</div>
		{/if}

		{#if WITH_DEVICES.has(target.kind)}
			<ClientDevicesTable {target} />
		{/if}
	</div>
{/if}
