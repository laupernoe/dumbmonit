<script lang="ts">
	/**
	 * Pi-hole: is it blocking, what did it answer and block in the last 24
	 * hours, how old are its blocklists, which versions run and whether an
	 * update waits, and how many messages its diagnosis page lists.
	 *
	 * Read straight from the latest stored measurement (`dumbmonit_pihole_*`):
	 * opening the page never connects to Pi-hole. Every state is a word as well
	 * as a colour, and an unknown value reads "—".
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type MetricSeries, type Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate } from '#lib/ui/index.js';
	import Figure from '../Figure.svelte';
	import { formatSpan } from '../pbs/format';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	interface Reading {
		blocking: boolean | null;
		timer: number | null;
		queries: number | null;
		blocked: number | null;
		percent: number | null;
		servfail: number | null;
		clientsActive: number | null;
		gravityDomains: number | null;
		gravityAge: number | null;
		versions: { name: string; version: string }[];
		outdated: string[];
		updates: number | null;
		messages: number | null;
		messageTypes: { type: string; count: number }[];
	}

	const EMPTY: Reading = {
		blocking: null,
		timer: null,
		queries: null,
		blocked: null,
		percent: null,
		servfail: null,
		clientsActive: null,
		gravityDomains: null,
		gravityAge: null,
		versions: [],
		outdated: [],
		updates: null,
		messages: null,
		messageTypes: []
	};

	const COMPONENTS: Record<string, string> = { core: 'Core', web: 'Web interface', ftl: 'FTL', docker: 'Docker image' };

	/** Eight days: the threshold of the built-in rule. */
	const GRAVITY_STALE = 8 * 86_400;

	let reading = $state<Reading>(EMPTY);
	let loading = $state(true);
	let error = $state<unknown>(null);

	function fold(series: MetricSeries[]): Reading {
		const out: Reading = { ...EMPTY, versions: [], outdated: [], messageTypes: [] };
		for (const serie of series) {
			const name = (serie.metric.__name__ ?? '').replace('dumbmonit_pihole_', '');
			const value = Number(serie.values.at(-1)?.[1]);
			if (!Number.isFinite(value)) continue;
			switch (name) {
				case 'blocking_enabled':
					out.blocking = value >= 1;
					break;
				case 'blocking_timer_seconds':
					out.timer = value;
					break;
				case 'queries_24h':
					out.queries = value;
					break;
				case 'queries_blocked_24h':
					out.blocked = value;
					break;
				case 'queries_blocked_percent':
					out.percent = value;
					break;
				case 'replies_servfail_24h':
					out.servfail = value;
					break;
				case 'clients_active':
					out.clientsActive = value;
					break;
				case 'gravity_domains':
					out.gravityDomains = value;
					break;
				case 'gravity_age_seconds':
					out.gravityAge = value;
					break;
				case 'updates_available':
					out.updates = value;
					break;
				case 'update_available':
					if (value >= 1 && serie.metric.component) out.outdated.push(serie.metric.component);
					break;
				case 'messages':
					out.messages = value;
					break;
				case 'messages_by_type':
					if (serie.metric.type) out.messageTypes.push({ type: serie.metric.type, count: value });
					break;
				case 'version_info':
					for (const key of ['core', 'web', 'ftl', 'docker']) {
						const version = serie.metric[key];
						if (version) out.versions.push({ name: key, version });
					}
					break;
			}
		}
		out.messageTypes.sort((a, b) => b.count - a.count || a.type.localeCompare(b.type, 'en'));
		return out;
	}

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const series = await queryInstant(
				`{__name__=~"dumbmonit_pihole_(blocking_enabled|blocking_timer_seconds|queries_24h|queries_blocked_24h|queries_blocked_percent|replies_servfail_24h|clients_active|gravity_domains|gravity_age_seconds|updates_available|update_available|messages|messages_by_type|version_info)", target="${target.id}"}`,
				signal
			);
			reading = fold(series);
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
		reading = EMPTY;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	function count(value: number | null): string | null {
		return value === null ? null : Math.round(value).toLocaleString('en');
	}

	function percent(value: number | null): string | null {
		if (value === null) return null;
		return `${value < 10 ? value.toFixed(1) : Math.round(value)}%`;
	}

	function typeWord(type: string): string {
		const words = type.replace(/_/g, ' ').toLowerCase();
		return words.charAt(0).toUpperCase() + words.slice(1);
	}

	const hasAnything = $derived(reading.blocking !== null || reading.queries !== null);
	const gravityStale = $derived(reading.gravityAge !== null && reading.gravityAge > GRAVITY_STALE);
	const blockingWord = $derived(
		reading.blocking === null
			? 'Unknown'
			: reading.blocking
				? 'Blocking'
				: reading.timer !== null
					? `Paused, back in ${formatSpan(reading.timer)}`
					: 'Not blocking'
	);
</script>

{#if !loading && (hasAnything || error)}
	<Panel title="Pi-hole" description="Last 24 hours, as Pi-hole counts them." padded={false} class="rise-in">
		{#snippet aside()}
			{#if reading.blocking !== null}
				<Plate tone={reading.blocking ? 'signal' : 'advisory'} label={blockingWord} size="md" />
			{/if}
		{/snippet}
		{#if error}
			<div class="px-5 py-4">
				<ErrorNotice {error} title="Could not load Pi-hole's figures" onretry={() => void load()} />
			</div>
		{:else}
			<div class="grid grid-cols-2 gap-x-6 gap-y-2 px-5 pt-4 pb-2 sm:grid-cols-4">
				<Figure label="Queries" value={count(reading.queries)} />
				<Figure label="Blocked" value={count(reading.blocked)} hint={reading.percent !== null ? `${percent(reading.percent)} of queries` : undefined} />
				<Figure label="Active clients" value={count(reading.clientsActive)} />
				<Figure
					label="Upstream failures"
					value={count(reading.servfail)}
					tone={reading.servfail !== null && reading.servfail > 0 ? 'advisory' : 'ink'}
					hint="SERVFAIL answers"
				/>
			</div>

			<ul class="divide-y divide-line border-t border-line text-sm">
				<li class="flex flex-col gap-1 px-5 py-3 sm:flex-row sm:items-center sm:gap-x-3">
					<Plate tone={reading.gravityAge === null ? 'ghost' : gravityStale ? 'advisory' : 'signal'} label={reading.gravityAge === null ? 'Unknown' : gravityStale ? 'Stale' : 'Up to date'} />
					<span class="font-medium text-ink">Blocklists</span>
					<span class="tnum text-ink-2 sm:ml-auto">
						{reading.gravityDomains !== null ? `${count(reading.gravityDomains)} domains` : '—'}{reading.gravityAge !== null ? `, rebuilt ${formatSpan(reading.gravityAge)} ago` : ''}
					</span>
				</li>
				<li class="flex flex-col gap-1 px-5 py-3 sm:flex-row sm:items-center sm:gap-x-3">
					{#if reading.updates === null}
						<Plate tone="ghost" label="Not checked yet" />
					{:else if reading.updates > 0}
						<Plate tone="info" label="Update available" />
					{:else}
						<Plate tone="signal" label="Up to date" />
					{/if}
					<span class="font-medium text-ink">Versions</span>
					<span class="tnum text-ink-2 sm:ml-auto">
						{#if reading.versions.length > 0}
							{reading.versions
								.map((v) => `${COMPONENTS[v.name] ?? v.name} ${v.version}${reading.outdated.includes(v.name) ? ' (newer available)' : ''}`)
								.join(' · ')}
						{:else}
							—
						{/if}
					</span>
				</li>
				<li class="flex flex-col gap-1 px-5 py-3 sm:flex-row sm:items-center sm:gap-x-3">
					{#if reading.messages === null}
						<Plate tone="ghost" label="Unknown" />
					{:else if reading.messages > 0}
						<Plate tone="info" label={reading.messages === 1 ? '1 message' : `${reading.messages} messages`} />
					{:else}
						<Plate tone="signal" label="No message" />
					{/if}
					<span class="font-medium text-ink">Diagnosis</span>
					<span class="text-ink-2 sm:ml-auto">
						{#if reading.messageTypes.length > 0}
							{reading.messageTypes.map((m) => (m.count > 1 ? `${typeWord(m.type)} (${m.count})` : typeWord(m.type))).join(' · ')}
						{:else if reading.messages === 0}
							Nothing listed on Pi-hole's diagnosis page.
						{:else}
							—
						{/if}
					</span>
				</li>
			</ul>
		{/if}
	</Panel>
{/if}
