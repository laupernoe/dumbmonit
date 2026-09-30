<script lang="ts">
	/**
	 * Home Assistant: whether the core runs, then what needs a hand — low
	 * batteries, updates, open repairs, unavailable entities — then the
	 * entities by domain. Read from the latest stored measurement; opening the
	 * page never connects to Home Assistant.
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type Target } from '$lib/api';
	import { ErrorNotice, Panel, Plate, type Tone } from '$lib/ui';
	import { formatCount } from '../truenas/format';
	import { humanize, readings, selector, type Reading } from '../instant';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	interface Domain {
		name: string;
		total: number;
		unavailable: number;
		unknown: number;
	}

	interface View {
		version: string | null;
		running: boolean | null;
		recovery: boolean;
		domains: Domain[];
		unavailable: { entity: string; name: string }[];
		batteries: number | null;
		low: { entity: string; name: string; level: number }[];
		updates: { entity: string; name: string; installed: string; latest: string }[];
		repairs: { issue: string; domain: string; severity: string }[] | null;
	}

	const EMPTY: View = {
		version: null,
		running: null,
		recovery: false,
		domains: [],
		unavailable: [],
		batteries: null,
		low: [],
		updates: [],
		repairs: null
	};

	const FAMILIES = [
		'info',
		'running',
		'recovery_mode',
		'entities',
		'entities_unavailable',
		'entities_unknown',
		'entity_unavailable',
		'batteries',
		'battery_low',
		'update_available',
		'repairs',
		'repair'
	];

	let view = $state<View>(EMPTY);
	let loading = $state(true);
	let error = $state<unknown>(null);

	function fold(rows: Reading[]): View {
		const out: View = { ...EMPTY, domains: [], unavailable: [], low: [], updates: [], repairs: null };
		const domains = new Map<string, Domain>();
		const domain = (name: string) => {
			let d = domains.get(name);
			if (!d) {
				d = { name, total: 0, unavailable: 0, unknown: 0 };
				domains.set(name, d);
			}
			return d;
		};
		const repairs: NonNullable<View['repairs']> = [];
		let sawRepairs = false;
		for (const { name, labels, value } of rows) {
			switch (name) {
				case 'info':
					out.version = labels.version || null;
					break;
				case 'running':
					out.running = value >= 1;
					break;
				case 'recovery_mode':
					out.recovery = value >= 1;
					break;
				case 'entities':
					domain(labels.domain ?? '').total = value;
					break;
				case 'entities_unavailable':
					domain(labels.domain ?? '').unavailable = value;
					break;
				case 'entities_unknown':
					domain(labels.domain ?? '').unknown = value;
					break;
				case 'entity_unavailable':
					out.unavailable.push({ entity: labels.entity ?? '', name: labels.name ?? labels.entity ?? '' });
					break;
				case 'batteries':
					out.batteries = value;
					break;
				case 'battery_low':
					out.low.push({ entity: labels.entity ?? '', name: labels.name ?? labels.entity ?? '', level: value });
					break;
				case 'update_available':
					out.updates.push({
						entity: labels.entity ?? '',
						name: labels.name ?? labels.entity ?? '',
						installed: labels.installed ?? '',
						latest: labels.latest ?? ''
					});
					break;
				case 'repairs':
					sawRepairs = true;
					break;
				case 'repair':
					repairs.push({ issue: labels.issue ?? '', domain: labels.domain ?? '', severity: labels.severity ?? 'warning' });
					break;
			}
		}
		const RANK: Record<string, number> = { critical: 0, error: 1, warning: 2 };
		out.repairs = sawRepairs ? repairs.sort((a, b) => (RANK[a.severity] ?? 3) - (RANK[b.severity] ?? 3)) : null;
		out.low.sort((a, b) => a.level - b.level || a.name.localeCompare(b.name, 'en'));
		out.updates.sort((a, b) => a.name.localeCompare(b.name, 'en'));
		out.unavailable.sort((a, b) => a.name.localeCompare(b.name, 'en'));
		out.domains = [...domains.values()].sort(
			(a, b) => b.unavailable + b.unknown - (a.unavailable + a.unknown) || a.name.localeCompare(b.name, 'en')
		);
		return out;
	}

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const series = await queryInstant(selector('homeassistant', FAMILIES, target.id), signal);
			view = fold(readings(series, 'homeassistant'));
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
		view = EMPTY;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	function severityTone(severity: string): Tone {
		return severity === 'warning' ? 'advisory' : 'warning';
	}

	const totals = $derived(
		view.domains.reduce(
			(acc, d) => ({ total: acc.total + d.total, unavailable: acc.unavailable + d.unavailable, unknown: acc.unknown + d.unknown }),
			{ total: 0, unavailable: 0, unknown: 0 }
		)
	);
	const problemDomains = $derived(view.domains.filter((d) => d.unavailable + d.unknown > 0));
	const quietDomains = $derived(view.domains.filter((d) => d.unavailable + d.unknown === 0));
	const attention = $derived(view.low.length + view.updates.length + (view.repairs?.length ?? 0) + view.unavailable.length);
	const hasAnything = $derived(view.version !== null || view.domains.length > 0);
</script>

{#if !loading && (hasAnything || error)}
	<Panel
		title="Home Assistant"
		description={view.version ? `Version ${view.version}` : undefined}
		padded={false}
		class="rise-in"
	>
		{#snippet aside()}
			{#if view.recovery}
				<Plate tone="warning" label="Recovery mode" />
			{:else if view.running === false}
				<Plate tone="warning" label="Not running" />
			{:else if view.running}
				<Plate tone="signal" label="Running" />
			{/if}
		{/snippet}
		{#if error}
			<div class="px-5 py-4">
				<ErrorNotice {error} title="Could not load Home Assistant" onretry={() => void load()} />
			</div>
		{:else}
			<div class="flex flex-wrap gap-x-6 gap-y-1 px-5 py-3 text-sm text-ink-2 tnum">
				<span><span class="font-medium text-ink">{formatCount(totals.total)}</span>{' '}entities</span>
				<span><span class="font-medium text-ink">{formatCount(totals.unavailable)}</span>{' '}unavailable</span>
				<span><span class="font-medium text-ink">{formatCount(totals.unknown)}</span>{' '}unknown</span>
				{#if view.batteries !== null}
					<span><span class="font-medium text-ink">{formatCount(view.low.length)}</span>{` of ${formatCount(view.batteries)} batteries low`}</span>
				{/if}
			</div>

			{#if attention === 0}
				<p class="border-t border-line px-5 py-4 text-sm text-ink-2">Nothing needs attention: no low battery, no update waiting, no open repair.</p>
			{/if}

			{#if view.repairs && view.repairs.length > 0}
				<div class="border-t border-line px-5 py-4">
					<h3 class="text-sm font-semibold text-ink">Repairs</h3>
					<ul class="mt-2 flex flex-col gap-2">
						{#each view.repairs as r (`${r.domain}:${r.issue}`)}
							<li class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
								<Plate tone={severityTone(r.severity)} label={humanize(r.severity)} />
								<span class="text-ink">{humanize(r.issue)}</span>
								<span class="text-ink-3">{r.domain}</span>
							</li>
						{/each}
					</ul>
				</div>
			{/if}

			{#if view.low.length > 0}
				<div class="border-t border-line px-5 py-4">
					<h3 class="text-sm font-semibold text-ink">Low batteries</h3>
					<ul class="mt-2 flex flex-col gap-1.5">
						{#each view.low as b (b.entity)}
							<li class="flex items-baseline gap-3 text-sm">
								<span class="tnum w-12 shrink-0 font-medium text-warning-ink">{b.level > 0 ? `${Math.round(b.level)}%` : 'Low'}</span>
								<span class="min-w-0 truncate text-ink" title={b.entity}>{b.name}</span>
							</li>
						{/each}
					</ul>
				</div>
			{/if}

			{#if view.updates.length > 0}
				<div class="border-t border-line px-5 py-4">
					<h3 class="text-sm font-semibold text-ink">Updates waiting</h3>
					<ul class="mt-2 flex flex-col gap-1.5">
						{#each view.updates as u (u.entity)}
							<li class="flex flex-wrap items-baseline gap-x-3 text-sm">
								<span class="min-w-0 text-ink" title={u.entity}>{u.name}</span>
								<span class="tnum text-ink-2">{`${u.installed || '?'} → ${u.latest || '?'}`}</span>
							</li>
						{/each}
					</ul>
				</div>
			{/if}

			{#if view.unavailable.length > 0}
				<div class="border-t border-line px-5 py-4">
					<h3 class="text-sm font-semibold text-ink">Unavailable entities</h3>
					<ul class="mt-2 grid grid-cols-1 gap-x-6 gap-y-1 sm:grid-cols-2">
						{#each view.unavailable as e (e.entity)}
							<li class="min-w-0 truncate text-sm">
								<span class="text-ink">{e.name}</span>{' '}<span class="text-ink-3">{e.entity}</span>
							</li>
						{/each}
					</ul>
					{#if view.unavailable.length >= 50}
						<p class="mt-2 text-[0.8rem] text-ink-3">Only the first 50 are named; the counts below include them all.</p>
					{/if}
				</div>
			{/if}

			{#if view.domains.length > 0}
				<div class="border-t border-line px-5 py-4">
					<h3 class="text-sm font-semibold text-ink">Entities by domain</h3>
					{#if problemDomains.length > 0}
						<table class="mt-2 w-full text-sm">
							<thead>
								<tr class="text-left text-[0.75rem] text-ink-3">
									<th class="py-1 font-normal">Domain</th>
									<th class="py-1 text-right font-normal">Entities</th>
									<th class="py-1 text-right font-normal">Unavailable</th>
									<th class="py-1 text-right font-normal">Unknown</th>
								</tr>
							</thead>
							<tbody class="tnum">
								{#each problemDomains as d (d.name)}
									<tr class="border-t border-line">
										<td class="py-1 text-ink">{d.name}</td>
										<td class="py-1 text-right text-ink-2">{formatCount(d.total)}</td>
										<td class="py-1 text-right {d.unavailable > 0 ? 'text-warning-ink' : 'text-ink-2'}">{formatCount(d.unavailable)}</td>
										<td class="py-1 text-right {d.unknown > 0 ? 'text-advisory-ink' : 'text-ink-2'}">{formatCount(d.unknown)}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					{/if}
					{#if quietDomains.length > 0}
						<p class="mt-2 text-[0.8rem] text-ink-3">
							{`All available: ${quietDomains.map((d) => `${d.name} (${formatCount(d.total)})`).join(', ')}.`}
						</p>
					{/if}
				</div>
			{/if}
		{/if}
	</Panel>
{/if}
