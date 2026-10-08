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
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
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

	function componentName(key: string): string {
		switch (key) {
			case 'core':
				return m.devicesb_pihole_panel_comp_core();
			case 'web':
				return m.devicesb_pihole_panel_comp_web();
			case 'ftl':
				return m.devicesb_pihole_panel_comp_ftl();
			case 'docker':
				return m.devicesb_pihole_panel_comp_docker();
			default:
				return key;
		}
	}

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
		return value === null ? null : Math.round(value).toLocaleString(getLocale());
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
			? m.devicesb_pihole_panel_blocking_unknown()
			: reading.blocking
				? m.devicesb_pihole_panel_blocking_on()
				: reading.timer !== null
					? m.devicesb_pihole_panel_blocking_paused({ span: formatSpan(reading.timer) })
					: m.devicesb_pihole_panel_blocking_off()
	);
</script>

{#if !loading && (hasAnything || error)}
	<Panel title="Pi-hole" description={m.devicesb_pihole_panel_description()} padded={false} class="rise-in">
		{#snippet aside()}
			{#if reading.blocking !== null}
				<Plate tone={reading.blocking ? 'signal' : 'advisory'} label={blockingWord} size="md" />
			{/if}
		{/snippet}
		{#if error}
			<div class="px-4 py-4 sm:px-5">
				<ErrorNotice {error} title={m.devicesb_pihole_panel_error_title()} onretry={() => void load()} />
			</div>
		{:else}
			<div class="grid grid-cols-2 gap-x-6 gap-y-2 px-4 pt-4 pb-2 sm:grid-cols-4 sm:px-5">
				<Figure label={m.devicesb_pihole_panel_fig_queries()} value={count(reading.queries)} />
				<Figure label={m.devicesb_pihole_panel_fig_blocked()} value={count(reading.blocked)} hint={reading.percent !== null ? m.devicesb_pihole_panel_fig_blocked_hint({ percent: percent(reading.percent) ?? '' }) : undefined} />
				<Figure label={m.devicesb_pihole_panel_fig_clients()} value={count(reading.clientsActive)} />
				<Figure
					label={m.devicesb_pihole_panel_fig_servfail()}
					value={count(reading.servfail)}
					tone={reading.servfail !== null && reading.servfail > 0 ? 'advisory' : 'ink'}
					hint={m.devicesb_pihole_panel_fig_servfail_hint()}
				/>
			</div>

			<ul class="divide-y divide-line border-t border-line text-sm">
				<li class="flex flex-col gap-1 px-4 py-3 sm:flex-row sm:px-5 sm:items-center sm:gap-x-3">
					<Plate tone={reading.gravityAge === null ? 'ghost' : gravityStale ? 'advisory' : 'signal'} label={reading.gravityAge === null ? m.devicesb_pihole_panel_gravity_unknown() : gravityStale ? m.devicesb_pihole_panel_gravity_stale() : m.devicesb_pihole_panel_gravity_ok()} />
					<span class="font-medium text-ink">{m.devicesb_pihole_panel_blocklists()}</span>
					<span class="tnum break-words text-ink-2 sm:ml-auto sm:text-right">
						{[
							reading.gravityDomains !== null ? m.devicesb_pihole_panel_gravity_domains({ count: count(reading.gravityDomains) ?? '' }) : '',
							reading.gravityAge !== null ? m.devicesb_pihole_panel_gravity_rebuilt({ span: formatSpan(reading.gravityAge) }) : ''
						]
							.filter(Boolean)
							.join(', ') || '—'}
					</span>
				</li>
				<li class="flex flex-col gap-1 px-4 py-3 sm:flex-row sm:px-5 sm:items-center sm:gap-x-3">
					{#if reading.updates === null}
						<Plate tone="ghost" label={m.devicesb_pihole_panel_ver_not_checked()} />
					{:else if reading.updates > 0}
						<Plate tone="info" label={m.devicesb_pihole_panel_ver_update()} />
					{:else}
						<Plate tone="signal" label={m.devicesb_pihole_panel_ver_ok()} />
					{/if}
					<span class="font-medium text-ink">{m.devicesb_pihole_panel_versions()}</span>
					<span class="tnum break-words text-ink-2 sm:ml-auto sm:text-right">
						{#if reading.versions.length > 0}
							{reading.versions
								.map((v) =>
									reading.outdated.includes(v.name)
										? m.devicesb_pihole_panel_ver_newer({ component: componentName(v.name), version: v.version })
										: m.devicesb_pihole_panel_ver_plain({ component: componentName(v.name), version: v.version })
								)
								.join(' · ')}
						{:else}
							—
						{/if}
					</span>
				</li>
				<li class="flex flex-col gap-1 px-4 py-3 sm:flex-row sm:px-5 sm:items-center sm:gap-x-3">
					{#if reading.messages === null}
						<Plate tone="ghost" label={m.devicesb_pihole_panel_msg_unknown()} />
					{:else if reading.messages > 0}
						<Plate tone="info" label={m.devicesb_pihole_panel_msg_count({ count: reading.messages })} />
					{:else}
						<Plate tone="signal" label={m.devicesb_pihole_panel_msg_none()} />
					{/if}
					<span class="font-medium text-ink">{m.devicesb_pihole_panel_diagnosis()}</span>
					<span class="break-words text-ink-2 sm:ml-auto sm:text-right">
						{#if reading.messageTypes.length > 0}
							{reading.messageTypes.map((m) => (m.count > 1 ? `${typeWord(m.type)} (${m.count})` : typeWord(m.type))).join(' · ')}
						{:else if reading.messages === 0}
							{m.devicesb_pihole_panel_msg_nothing()}
						{:else}
							—
						{/if}
					</span>
				</li>
			</ul>
		{/if}
	</Panel>
{/if}
