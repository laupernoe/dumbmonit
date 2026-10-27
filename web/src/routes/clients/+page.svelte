<script lang="ts">
	/**
	 * MSP mode: the clients at a glance. The MSP administrator sees one row per
	 * client — its LED, what is alerting, how full its quota is — and opens a
	 * client to narrow every page to it. A member of clients sees his own,
	 * each with its availability report of last month.
	 */
	import { Building2, FileDown } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { clientLogoUrl, getClientsOverview, toApiError, type ClientHealth, type ClientRef } from '#lib/api/index.js';
	import { downloadSlaReport } from '#lib/api/reports.js';
	import { Button, EmptyState, ErrorNotice, Led, PageHeader, Plate, Skeleton } from '#lib/ui/index.js';

	const admin = $derived(auth.msp?.admin === true);

	let health = $state<ClientHealth[] | null>(null);
	let loading = $state(false);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			health = await getClientsOverview(signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		if (!admin) return;
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});

	/** Rows for a member: the clients `/api/auth/me` offers him. */
	const mine = $derived<ClientRef[]>(admin ? [] : (auth.msp?.clients ?? []));

	function healthOf(row: ClientHealth): { tone: 'signal' | 'advisory' | 'warning' | 'ghost'; label: string } {
		if (row.targets === 0) return { tone: 'ghost', label: m.msp_overview_state_empty() };
		if (row.critical > 0) return { tone: 'warning', label: m.msp_overview_state_critical({ count: row.alerting_targets }) };
		if (row.warning > 0 || row.alerting_targets > 0)
			return { tone: 'advisory', label: m.msp_overview_state_warning({ count: row.alerting_targets }) };
		return { tone: 'signal', label: m.msp_overview_state_ok() };
	}

	/** Share of the quota in use, 0 to 100; `null` without a quota. */
	function quotaUse(row: ClientHealth): number | null {
		if (row.monitor_quota === null) return null;
		if (row.monitor_quota === 0) return 100;
		return Math.min(100, Math.round((row.targets / row.monitor_quota) * 100));
	}

	let downloading = $state<number | null>(null);
	let downloadError = $state<unknown>(null);
	async function report(client: number) {
		downloading = client;
		downloadError = null;
		try {
			const { blob, filename } = await downloadSlaReport({ format: 'pdf', client });
			const url = URL.createObjectURL(blob);
			const link = document.createElement('a');
			link.href = url;
			link.download = filename;
			document.body.append(link);
			link.click();
			link.remove();
			setTimeout(() => URL.revokeObjectURL(url), 10_000);
		} catch (cause) {
			downloadError = toApiError(cause);
		} finally {
			downloading = null;
		}
	}

	const initial = (name: string) => [...name.trim()][0]?.toUpperCase() ?? '?';
</script>

<svelte:head><title>{m.msp_overview_title()} · DumbMonit</title></svelte:head>

<PageHeader title={m.msp_overview_title()} description={admin ? m.msp_overview_description_admin() : m.msp_overview_description_member()} />

{#snippet badge(id: number, name: string, logo: boolean)}
	{#if logo}
		<img src={clientLogoUrl(id)} alt="" class="size-10 shrink-0 rounded-md border border-line bg-canvas object-contain p-0.5" />
	{:else}
		<span class="inline-flex size-10 shrink-0 items-center justify-center rounded-md border border-line bg-surface-2 text-base font-bold text-ink" aria-hidden="true">{initial(name)}</span>
	{/if}
{/snippet}

{#if !auth.mspOn}
	<EmptyState icon={Building2} title={m.msp_overview_off_title()} description={m.msp_overview_off_body()}>
		{#snippet action()}
			{#if auth.isAdmin}<Button variant="primary" href="/settings/team#clients">{m.msp_overview_off_action()}</Button>{/if}
		{/snippet}
	</EmptyState>
{:else}
	{#if downloadError}<div class="mb-3"><ErrorNotice error={downloadError} title={m.msp_overview_report_error()} /></div>{/if}

	{#if admin}
		{#if error}
			<ErrorNotice {error} title={m.msp_overview_load_error()} onretry={() => void load()} />
		{:else if loading && !health}
			<Skeleton class="h-16 w-full" rows={4} />
		{:else if health}
			<ul class="grid gap-2">
				{#each health as row (row.id)}
					{@const status = healthOf(row)}
					{@const use = quotaUse(row)}
					<li class="rounded-[var(--radius-card)] border border-line bg-canvas p-3">
						<div class="flex flex-wrap items-center gap-x-4 gap-y-3">
							<div class="flex min-w-0 flex-[1_1_14rem] items-center gap-3">
								{@render badge(row.id, row.name, row.has_logo)}
								<div class="min-w-0">
									<p class="font-semibold text-ink [overflow-wrap:anywhere]">{row.name}</p>
									<p class="flex items-center gap-2 text-sm text-ink-2">
										<Led tone={status.tone} blink={status.tone === 'warning'} size="sm" />
										{status.label}
									</p>
								</div>
							</div>
							<div class="flex flex-wrap items-center gap-1.5">
								{#if row.critical > 0}<Plate tone="warning" label={m.msp_overview_critical({ count: row.critical })} />{/if}
								{#if row.warning > 0}<Plate tone="advisory" label={m.msp_overview_warning({ count: row.warning })} />{/if}
								{#if row.paused > 0}<Plate tone="muted" bare label={m.msp_overview_paused({ count: row.paused })} />{/if}
							</div>
							<div class="grid min-w-[9rem] flex-[0_1_12rem] gap-1">
								<p class="tnum text-sm text-ink-2">
									{row.monitor_quota === null
										? m.msp_overview_targets({ count: row.targets })
										: m.msp_overview_targets_quota({ count: row.targets, quota: row.monitor_quota })}
								</p>
								{#if use !== null}
									<div class="h-1.5 overflow-hidden rounded-full bg-surface-2" role="meter" aria-valuemin={0} aria-valuemax={100} aria-valuenow={use} aria-label={m.msp_overview_quota_label()}>
										<div class={`h-full rounded-full ${use >= 100 ? 'bg-warning' : use >= 85 ? 'bg-advisory' : 'bg-signal'}`} style:width={`${use}%`}></div>
									</div>
								{/if}
							</div>
							<div class="flex shrink-0 flex-wrap items-center gap-2">
								<Button variant="secondary" loading={downloading === row.id} onclick={() => void report(row.id)} title={m.msp_overview_report_title()}>
									<FileDown class="size-4" aria-hidden="true" />
									{m.msp_overview_report()}
								</Button>
								<Button variant="primary" onclick={() => auth.selectClient(row.id, '/targets')}>{m.msp_overview_open()}</Button>
							</div>
						</div>
					</li>
				{:else}
					<li><EmptyState icon={Building2} title={m.msp_overview_empty_title()} description={m.msp_overview_empty_body()} /></li>
				{/each}
			</ul>
		{/if}
	{:else if mine.length === 0}
		<EmptyState icon={Building2} title={m.msp_overview_none_title()} description={m.msp_overview_none_body()} />
	{:else}
		<ul class="grid gap-2">
			{#each mine as client (client.id)}
				<li class="flex flex-wrap items-center gap-3 rounded-[var(--radius-card)] border border-line bg-canvas p-3">
					{@render badge(client.id, client.name, client.has_logo)}
					<p class="min-w-0 flex-1 font-semibold text-ink [overflow-wrap:anywhere]">{client.name}</p>
					<Button variant="secondary" loading={downloading === client.id} onclick={() => void report(client.id)} title={m.msp_overview_report_title()}>
						<FileDown class="size-4" aria-hidden="true" />
						{m.msp_overview_report()}
					</Button>
				</li>
			{/each}
		</ul>
	{/if}
{/if}
