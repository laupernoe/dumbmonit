<script lang="ts">
	/**
	 * MDaemon Email Server and SecurityGateway: the mail services as seen from
	 * outside, the version, and (SecurityGateway 12.5+) the REST API's
	 * performance counters. One component for both kinds: the series share
	 * their shape, only the prefix differs (`dumbmonit_mdaemon_*`,
	 * `dumbmonit_securitygateway_*`).
	 *
	 * Read straight from the latest stored measurement — opening the page never
	 * connects to the mail server. Stopped services come first; every state is a
	 * word as well as a colour, and an unknown value reads "—".
	 *
	 * MDaemon's queues are not visible from outside: when the Windows agent runs
	 * on the same server, it reads them from MDaemon's performance counters, and
	 * they are shown below this panel (see `queues.ts` for how the agent is found).
	 */
	import { untrack } from 'svelte';
	import { listTargets, queryInstant, type Target } from '$lib/api';
	import { ErrorNotice, Panel, Plate } from '$lib/ui';
	import MdaemonQueues from './MdaemonQueues.svelte';
	import { REPORTING_QUERY, pickAgent } from './queues';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	interface ServiceStat {
		name: string;
		port: string;
		up: boolean;
		seconds: number | null;
	}

	interface Reading {
		services: ServiceStat[];
		version: string | null;
		build: string | null;
		versionSource: string | null;
		/** `null` when no credential is configured: the API is not watched. */
		apiUp: boolean | null;
		operationOk: boolean | null;
		countersAvailable: boolean | null;
		counters: { name: string; value: number }[];
	}

	const EMPTY: Reading = {
		services: [],
		version: null,
		build: null,
		versionSource: null,
		apiUp: null,
		operationOk: null,
		countersAvailable: null,
		counters: []
	};

	const LABELS: Record<string, string> = {
		smtp: 'SMTP',
		msa: 'SMTP submission (MSA)',
		smtps: 'SMTP over TLS',
		pop3: 'POP3',
		pop3s: 'POP3 over TLS',
		imap: 'IMAP',
		imaps: 'IMAP over TLS',
		webmail: 'Webmail',
		remote_admin: 'Remote Administration',
		remote_admin_https: 'Remote Administration (HTTPS)',
		xmpp: 'XMPP',
		web: 'Web interface',
		web_https: 'Web interface (HTTPS)'
	};

	const prefix = $derived(target.kind === 'securitygateway' ? 'securitygateway' : 'mdaemon');
	const apiName = $derived(target.kind === 'securitygateway' ? 'REST API' : 'XML API');

	let reading = $state<Reading>(EMPTY);
	let loading = $state(true);
	let error = $state<unknown>(null);

	const down = $derived(reading.services.filter((s) => !s.up).length);

	function fold(series: { metric: Record<string, string>; values: [number, string][] }[]): Reading {
		const out: Reading = { ...EMPTY, services: [], counters: [] };
		const services = new Map<string, ServiceStat>();
		const serviceOf = (m: Record<string, string>): ServiceStat => {
			const key = `${m.service ?? ''}:${m.port ?? ''}`;
			let s = services.get(key);
			if (!s) {
				s = { name: m.service ?? '', port: m.port ?? '', up: false, seconds: null };
				services.set(key, s);
			}
			return s;
		};
		const base = `dumbmonit_${prefix}_`;
		for (const serie of series) {
			const name = (serie.metric.__name__ ?? '').replace(base, '');
			const value = Number(serie.values.at(-1)?.[1]);
			if (!Number.isFinite(value)) continue;
			switch (name) {
				case 'service_up':
					serviceOf(serie.metric).up = value >= 1;
					break;
				case 'service_response_seconds':
					serviceOf(serie.metric).seconds = value;
					break;
				case 'api_up':
					out.apiUp = value >= 1;
					break;
				case 'api_operation_ok':
					out.operationOk = value >= 1;
					break;
				case 'counters_available':
					out.countersAvailable = value >= 1;
					break;
				case 'counter':
					if (serie.metric.counter) out.counters.push({ name: serie.metric.counter, value });
					break;
				case 'info':
					out.version = serie.metric.version || null;
					out.build = serie.metric.build || null;
					out.versionSource = serie.metric.source || null;
					break;
			}
		}
		// Stopped services first, then by port: the order an admin reads them in.
		out.services = [...services.values()].sort(
			(a, b) => Number(a.up) - Number(b.up) || Number(a.port) - Number(b.port) || a.name.localeCompare(b.name, 'en')
		);
		out.counters.sort((a, b) => a.name.localeCompare(b.name, 'en'));
		return out;
	}

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const series = await queryInstant(
				`{__name__=~"dumbmonit_${prefix}_(service_up|service_response_seconds|api_up|api_operation_ok|counters_available|counter|info)", target="${target.id}"}`,
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

	/** The agent on the same machine, when it reports MDaemon's queues. */
	let linked = $state<Target | null>(null);

	$effect(() => {
		const device = target;
		linked = null;
		if (device.kind !== 'mdaemon') return;
		const controller = new AbortController();
		Promise.all([listTargets(controller.signal), queryInstant(REPORTING_QUERY, controller.signal)])
			.then(([targets, reporting]) => {
				const ids = new Set(reporting.map((s) => Number(s.metric.target)).filter(Number.isFinite));
				linked = pickAgent(device, targets, ids);
			})
			// The queues are a bonus: failing to find them must not cover the page in errors.
			.catch(() => (linked = null));
		return () => controller.abort();
	});

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

	function label(name: string): string {
		return LABELS[name] ?? name;
	}

	function ms(seconds: number | null): string {
		if (seconds === null) return '—';
		return seconds < 1 ? `${Math.round(seconds * 1000)} ms` : `${seconds.toFixed(1)} s`;
	}

	function counterLabel(name: string): string {
		const words = name.replace(/_/g, ' ');
		return words.charAt(0).toUpperCase() + words.slice(1);
	}

	const sourceWord: Record<string, string> = {
		xml_api: 'read from the XML API',
		openapi: 'read from the REST API',
		smtp_banner: 'announced in the SMTP greeting'
	};

	const hasAnything = $derived(
		reading.services.length > 0 || reading.apiUp !== null || reading.version !== null
	);
</script>

{#if !loading && (hasAnything || error)}
<Panel title="Mail services" description="Checked from outside, as a mail client would." padded={false} class="rise-in">
	{#snippet aside()}
		{#if reading.services.length > 0}
			{#if down === 0}
				<Plate tone="signal" label="All answering" />
			{:else}
				<Plate tone="warning" label={down === 1 ? '1 service down' : `${down} services down`} />
			{/if}
		{/if}
	{/snippet}
	{#if error}
		<div class="px-5 py-4">
			<ErrorNotice {error} title="Could not load the mail services" onretry={() => void load()} />
		</div>
	{:else}
		{#if reading.services.length > 0}
			<ul class="divide-y divide-line">
				{#each reading.services as s (`${s.name}:${s.port}`)}
					<li class="flex flex-col gap-1 px-5 py-3 sm:flex-row sm:items-center sm:gap-x-3">
						<Plate tone={s.up ? 'signal' : 'warning'} label={s.up ? 'Answering' : 'Down'} />
						<span class="min-w-0 text-sm font-medium text-ink break-all">{label(s.name)}</span>
						<span class="tnum text-sm text-ink-2">port {s.port}</span>
						<span class="tnum text-sm text-ink-2 sm:ml-auto">{s.up ? ms(s.seconds) : '—'}</span>
					</li>
				{/each}
			</ul>
		{:else}
			<p class="px-5 py-4 text-sm text-ink-2">No port is checked on this device: its Services option is set to none.</p>
		{/if}

		<div class="flex flex-col gap-2 border-t border-line px-5 py-4 text-sm">
			<p class="text-ink-2">
				{#if reading.version}
					Version <span class="tnum font-medium text-ink">{reading.version}</span>{#if reading.build && reading.build !== reading.version}&nbsp;(build <span class="tnum">{reading.build}</span>){/if},
					{sourceWord[reading.versionSource ?? ''] ?? 'as reported'}.
				{:else}
					Version unknown: the greeting does not announce it{reading.apiUp === null ? ', and no credential is set for the API' : ''}.
				{/if}
			</p>
			{#if prefix === 'mdaemon' && !linked}
				<p class="text-ink-2">
					Queue sizes are not visible from outside: install the DumbMonit agent on this server and they appear here. If they do not, make that agent the parent of this device.
				</p>
			{/if}
			{#if reading.apiUp !== null}
				<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
					<Plate tone={reading.apiUp ? 'signal' : 'warning'} label={reading.apiUp ? `${apiName} answering` : `${apiName} not answering`} />
					{#if reading.operationOk === false}
						<span class="text-ink-2">The account may not run GetVersionInfo; the version still comes from the answer.</span>
					{/if}
				</div>
			{/if}
		</div>

		{#if prefix === 'securitygateway' && reading.apiUp}
			<div class="border-t border-line px-5 py-4">
				<h3 class="text-sm font-semibold text-ink">Performance counters</h3>
				{#if reading.counters.length > 0}
					<dl class="mt-2 grid grid-cols-1 gap-x-6 gap-y-1 text-sm sm:grid-cols-2">
						{#each reading.counters as c (c.name)}
							<div class="flex justify-between gap-3 border-b border-line py-1">
								<dt class="min-w-0 text-ink-2 break-all">{counterLabel(c.name)}</dt>
								<dd class="tnum font-medium text-ink">{c.value.toLocaleString('en')}</dd>
							</div>
						{/each}
					</dl>
				{:else if reading.countersAvailable === false}
					<p class="mt-1 text-sm text-ink-2">No counters read: the API describes none, or the account that owns the key may not read them. Give that account the Global Administrator role if they stay empty.</p>
				{:else}
					<p class="mt-1 text-sm text-ink-2">Counter reading is turned off in this device's options.</p>
				{/if}
			</div>
		{/if}
	{/if}
</Panel>
{/if}

{#if linked}
	<MdaemonQueues agent={linked.id} agentName={linked.name} />
{/if}
