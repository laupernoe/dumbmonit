<script lang="ts">
	/**
	 * Settings → System → DumbMonit itself: the health of the engine (scheduler,
	 * database, metrics database), the data volume and database size, and the
	 * outgoing heartbeat — a periodic call to Healthchecks.io, Uptime Kuma,
	 * Better Stack… sent only while the engine is healthy, so that someone
	 * outside notices when this host goes quiet.
	 *
	 * The heartbeat URL usually holds its own secret: it is typed, sent, and
	 * never shown again (only its host). Leaving the field empty keeps it.
	 */
	import { ExternalLink, HeartPulse, Send } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getSelfMonitoring, setHeartbeat, testHeartbeat } from '#lib/api/selfMonitoring.js';
	import type { HeartbeatPing, HeartbeatTestResult, SelfMonitoring, SelfMonitoringComponent } from '#lib/api/types.js';
	import { formatRelative } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, ErrorNotice, Field, Panel, Plate, Skeleton, Toggle } from '#lib/ui/index.js';

	const DOCS_URL = 'https://dumbmonit.readthedocs.io/en/latest/install/self-monitoring/';
	const MIN_INTERVAL = 30;
	const MAX_INTERVAL = 3600;

	let data = $state<SelfMonitoring | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	// Form state, filled from the server on load.
	let enabled = $state(false);
	let url = $state('');
	let interval = $state(60);
	let sendFail = $state(false);
	let saving = $state(false);
	let saveError = $state<unknown>(null);
	let saved = $state(false);
	let intervalError = $state<string | null>(null);
	let urlError = $state<string | null>(null);

	let testing = $state(false);
	let testError = $state<unknown>(null);
	let tested = $state<HeartbeatTestResult | null>(null);

	function fill(value: SelfMonitoring) {
		data = value;
		enabled = value.heartbeat.enabled;
		interval = value.heartbeat.interval_seconds;
		sendFail = value.heartbeat.send_fail;
		url = '';
	}

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			fill(await getSelfMonitoring(signal));
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});

	function componentLabel(component: SelfMonitoringComponent): string {
		switch (component.name) {
			case 'scheduler':
				return m.settings_selfmon_component_scheduler();
			case 'database':
				return m.settings_selfmon_component_database();
			case 'victoria':
				return m.settings_selfmon_component_victoria();
		}
	}

	function bytes(value: number): string {
		if (value <= 0) return '0 B';
		const units = ['B', 'kB', 'MB', 'GB', 'TB'];
		let n = value;
		let unit = 0;
		while (n >= 1024 && unit < units.length - 1) {
			n /= 1024;
			unit += 1;
		}
		return `${n < 10 && unit > 0 ? n.toFixed(1) : Math.round(n)} ${units[unit]}`;
	}

	const freePercent = $derived(
		data && data.data_free_bytes !== null && data.data_total_bytes
			? Math.round((100 * data.data_free_bytes) / data.data_total_bytes)
			: null
	);

	function lastPingText(ping: HeartbeatPing): string {
		const when = formatRelative(ping.at);
		if (ping.outcome === 'withheld') return m.settings_selfmon_last_withheld({ when });
		if (ping.outcome === 'fail') return m.settings_selfmon_last_fail({ when });
		return ping.ok ? m.settings_selfmon_last_up({ when }) : m.settings_selfmon_last_rejected({ when });
	}

	function validate(): boolean {
		intervalError = null;
		urlError = null;
		const value = Number(interval);
		if (!Number.isInteger(value) || value < MIN_INTERVAL || value > MAX_INTERVAL) {
			intervalError = m.settings_selfmon_interval_invalid({ min: MIN_INTERVAL, max: MAX_INTERVAL });
		}
		const typed = url.trim();
		if (typed && !/^https?:\/\/\S+$/.test(typed)) urlError = m.settings_selfmon_url_invalid();
		if (enabled && !typed && !data?.heartbeat.has_url) urlError = m.settings_selfmon_url_required();
		return !intervalError && !urlError;
	}

	async function save(event: SubmitEvent) {
		event.preventDefault();
		saved = false;
		saveError = null;
		if (!validate()) return;
		saving = true;
		try {
			const typed = url.trim();
			const heartbeat = await setHeartbeat({
				enabled,
				interval_seconds: Number(interval),
				send_fail: sendFail,
				...(typed ? { url: typed } : {})
			});
			if (data) data = { ...data, heartbeat };
			url = '';
			saved = true;
		} catch (cause) {
			saveError = cause;
		} finally {
			saving = false;
		}
	}

	async function clearUrl() {
		saved = false;
		saveError = null;
		saving = true;
		try {
			const heartbeat = await setHeartbeat({ enabled: false, url: '' });
			if (data) data = { ...data, heartbeat };
			enabled = false;
			url = '';
		} catch (cause) {
			saveError = cause;
		} finally {
			saving = false;
		}
	}

	async function test() {
		tested = null;
		testError = null;
		urlError = null;
		const typed = url.trim();
		if (typed && !/^https?:\/\/\S+$/.test(typed)) {
			urlError = m.settings_selfmon_url_invalid();
			return;
		}
		if (!typed && !data?.heartbeat.has_url) {
			urlError = m.settings_selfmon_url_required();
			return;
		}
		testing = true;
		try {
			tested = await testHeartbeat(typed || undefined);
		} catch (cause) {
			testError = cause;
		} finally {
			testing = false;
		}
	}
</script>

<Panel id="self-monitoring" title={m.settings_selfmon_title()} description={m.settings_selfmon_description()}>
	{#snippet aside()}
		{#if data}
			<Plate
				tone={data.status === 'ok' ? 'signal' : 'warning'}
				label={data.status === 'ok' ? m.settings_selfmon_status_ok() : m.settings_selfmon_status_degraded()}
			/>
		{/if}
	{/snippet}

	{#if error}
		<ErrorNotice {error} title={m.settings_selfmon_read_error()} onretry={() => void load()} />
	{:else if loading && !data}
		<Skeleton class="h-32 w-full" />
	{:else if data}
		<div class="grid gap-8">
			<!-- Engine -->
			<section>
				<ul class="grid gap-2">
					{#each data.components as component (component.name)}
						<li class="flex min-h-10 flex-wrap items-center justify-between gap-x-4 gap-y-1">
							<span class="text-sm font-semibold text-ink">{componentLabel(component)}</span>
							<Plate
								tone={component.ok ? 'signal' : 'warning'}
								label={component.ok ? m.settings_selfmon_component_ok() : m.settings_selfmon_component_down()}
							/>
							{#if component.error}
								<p class="w-full break-words text-[0.8125rem] text-ink-2">{component.error}</p>
							{/if}
						</li>
					{/each}
				</ul>
				<dl class="mt-4 grid gap-x-6 gap-y-2 text-sm sm:grid-cols-2">
					<div>
						<dt class="text-ink-3">{m.settings_selfmon_data_volume()}</dt>
						<dd class="tnum text-ink">
							{#if data.data_free_bytes !== null && data.data_total_bytes !== null}
								{m.settings_selfmon_data_free({
									free: bytes(data.data_free_bytes),
									total: bytes(data.data_total_bytes),
									percent: freePercent ?? 0
								})}
							{:else}
								{m.settings_selfmon_unknown()}
							{/if}
						</dd>
					</div>
					<div>
						<dt class="text-ink-3">{m.settings_selfmon_database_size()}</dt>
						<dd class="tnum text-ink">{bytes(data.database_bytes)}</dd>
					</div>
					{#if data.vm_embedded}
						<div>
							<dt class="text-ink-3">{m.settings_selfmon_vm_restarts()}</dt>
							<dd class="tnum text-ink">{data.vm_restarts}</dd>
						</div>
					{/if}
					<div>
						<dt class="text-ink-3">{m.settings_selfmon_scheduler_backlog()}</dt>
						<dd class="tnum text-ink">{data.scheduler_backlog}</dd>
					</div>
				</dl>
				<p class="mt-3 max-w-prose text-[0.8125rem] text-ink-2">{m.settings_selfmon_rules_hint()}</p>
			</section>

			<!-- Heartbeat -->
			<section class="border-t border-line pt-6">
				<div class="flex items-start gap-2">
					<HeartPulse class="mt-0.5 size-4 shrink-0 text-ink-2" aria-hidden="true" />
					<div class="min-w-0">
						<h3 class="font-semibold text-ink">{m.settings_selfmon_heartbeat_title()}</h3>
						<p class="mt-1 max-w-prose text-sm text-ink-2">{m.settings_selfmon_heartbeat_intro()}</p>
						<a
							href={DOCS_URL}
							target="_blank"
							rel="noopener"
							class="mt-1 inline-flex min-h-10 items-center gap-1 text-[0.8125rem] font-semibold text-ink-2 hover:text-ink hover:underline"
						>
							{m.settings_selfmon_docs()}
							<ExternalLink class="size-3.5" aria-hidden="true" />
						</a>
					</div>
				</div>

				{#if data.heartbeat.last_ping}
					{@const ping = data.heartbeat.last_ping}
					<div class="mt-3 flex flex-wrap items-center gap-2 text-sm text-ink-2" aria-live="polite">
						<Plate
							tone={ping.outcome === 'up' && ping.ok ? 'signal' : 'warning'}
							label={ping.outcome === 'up' && ping.ok ? m.settings_selfmon_ping_sent() : m.settings_selfmon_ping_not_sent()}
						/>
						<span>{lastPingText(ping)}</span>
						{#if ping.error}
							<span class="w-full break-words text-[0.8125rem]">{ping.error}</span>
						{/if}
					</div>
				{/if}

				{#if !auth.isAdmin}
					<p class="mt-3 text-[0.8125rem] text-ink-3">
						{data.heartbeat.enabled
							? m.settings_selfmon_heartbeat_on_host({ host: data.heartbeat.url_host ?? '' })
							: m.settings_selfmon_heartbeat_off()}
					</p>
					<p class="mt-1 text-[0.8125rem] text-ink-3">{m.settings_selfmon_admin_only()}</p>
				{:else}
					<form class="mt-4 grid max-w-xl gap-4" onsubmit={save} novalidate>
						<Field label={m.settings_selfmon_enabled()} for="heartbeat-enabled" inline>
							<Toggle id="heartbeat-enabled" bind:checked={enabled} disabled={saving} label={m.settings_selfmon_enabled()} />
						</Field>
						<Field
							label={m.settings_selfmon_url()}
							for="heartbeat-url"
							error={urlError}
							help={data.heartbeat.has_url
								? m.settings_selfmon_url_saved({ host: data.heartbeat.url_host ?? '' })
								: m.settings_selfmon_url_help()}
						>
							<input
								id="heartbeat-url"
								type="url"
								class="input font-mono"
								bind:value={url}
								placeholder="https://hc-ping.com/…"
								autocomplete="off"
								spellcheck="false"
								disabled={saving}
								aria-invalid={urlError ? 'true' : undefined}
								oninput={() => (urlError = null)}
							/>
						</Field>
						<Field
							label={m.settings_selfmon_interval()}
							for="heartbeat-interval"
							error={intervalError}
							help={m.settings_selfmon_interval_help({ min: MIN_INTERVAL, max: MAX_INTERVAL })}
						>
							<input
								id="heartbeat-interval"
								type="number"
								min={MIN_INTERVAL}
								max={MAX_INTERVAL}
								step="1"
								class="input max-w-40"
								bind:value={interval}
								disabled={saving}
								aria-invalid={intervalError ? 'true' : undefined}
								oninput={() => (intervalError = null)}
							/>
						</Field>
						<Field label={m.settings_selfmon_send_fail()} for="heartbeat-fail" inline help={m.settings_selfmon_send_fail_help()}>
							<Toggle id="heartbeat-fail" bind:checked={sendFail} disabled={saving} label={m.settings_selfmon_send_fail()} />
						</Field>

						{#if saveError}
							<ErrorNotice error={saveError} title={m.settings_selfmon_save_error()} />
						{/if}
						{#if testError}
							<ErrorNotice error={testError} title={m.settings_selfmon_test_error()} />
						{/if}

						<div class="flex flex-wrap items-center gap-3" aria-live="polite">
							<Button type="submit" variant="primary" loading={saving}>{m.settings_selfmon_save()}</Button>
							<Button variant="secondary" loading={testing} disabled={saving} onclick={() => void test()}>
								<Send class="size-4" aria-hidden="true" />
								{m.settings_selfmon_test()}
							</Button>
							{#if data.heartbeat.has_url}
								<Button variant="ghost" disabled={saving} onclick={() => void clearUrl()}>{m.settings_selfmon_clear_url()}</Button>
							{/if}
							{#if saved}
								<Plate tone="signal" draw label={m.settings_selfmon_saved()} />
							{/if}
							{#if tested}
								<Plate
									tone={tested.ok ? 'signal' : 'warning'}
									label={tested.ok ? m.settings_selfmon_test_ok() : m.settings_selfmon_test_failed()}
								/>
							{/if}
						</div>
						{#if tested?.error}
							<p class="break-words text-[0.8125rem] text-ink-2">{tested.error}</p>
						{/if}
					</form>
				{/if}
			</section>
		</div>
	{/if}
</Panel>
