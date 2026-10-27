<script lang="ts">
	/**
	 * Reports → "Availability report (SLA)": the report of a period
	 * you choose, built on the spot and downloaded as a PDF or as CSV files.
	 * Dates are inclusive and read in the chosen timezone; the default is the
	 * previous calendar month.
	 */
	import { Download } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { toApiError, type StatusPage } from '#lib/api/index.js';
	import { downloadSlaReport, type SlaFormat } from '#lib/api/reports.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, ErrorNotice, Field } from '#lib/ui/index.js';

	interface Props {
		pages: StatusPage[];
		defaultTimezone: string;
		timezones: string[];
	}

	let { pages, defaultTimezone, timezones }: Props = $props();

	/** `YYYY-MM-DD` of a local date, without going through UTC (which shifts the day). */
	function day(date: Date): string {
		const pad = (value: number) => String(value).padStart(2, '0');
		return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
	}

	const now = new Date();
	const lastMonthFirst = new Date(now.getFullYear(), now.getMonth() - 1, 1);
	const lastMonthLast = new Date(now.getFullYear(), now.getMonth(), 0);

	let from = $state(day(lastMonthFirst));
	let to = $state(day(lastMonthLast));
	let timezone = $state(defaultTimezone);
	let format = $state<SlaFormat>('pdf');
	let objective = $state('');
	let pageId = $state('');
	// MSP mode: the client the report is about. An administrator may leave it on
	// "all clients"; a member of several clients has to pick one.
	const clients = $derived(auth.mspOn ? (auth.msp?.clients ?? []) : []);
	const mspAdmin = $derived(auth.msp?.admin === true);
	let clientId = $state(auth.currentClient === null ? '' : String(auth.currentClient));
	const clientValue = $derived(clientId !== '' ? clientId : mspAdmin ? '' : String(clients[0]?.id ?? ''));
	let busy = $state(false);
	let error = $state<unknown>(null);

	const FORMATS: { value: SlaFormat; label: string }[] = [
		{ value: 'pdf', label: m.settings_sla_format_pdf() },
		{ value: 'csv', label: m.settings_sla_format_csv() },
		{ value: 'incidents_csv', label: m.settings_sla_format_incidents() }
	];

	function lastMonth() {
		from = day(lastMonthFirst);
		to = day(lastMonthLast);
	}

	function thisMonth() {
		from = day(new Date(now.getFullYear(), now.getMonth(), 1));
		to = day(now);
	}

	async function download(event: SubmitEvent) {
		event.preventDefault();
		error = null;
		busy = true;
		try {
			const goal = Number(objective.replace(',', '.'));
			const { blob, filename } = await downloadSlaReport({
				from,
				to,
				tz: timezone.trim() || undefined,
				format,
				objective: objective.trim() !== '' && goal > 0 ? goal : undefined,
				page: pageId === '' ? undefined : Number(pageId),
				client: clientValue === '' ? undefined : Number(clientValue)
			});
			const url = URL.createObjectURL(blob);
			const link = document.createElement('a');
			link.href = url;
			link.download = filename;
			document.body.append(link);
			link.click();
			link.remove();
			setTimeout(() => URL.revokeObjectURL(url), 10_000);
		} catch (cause) {
			error = toApiError(cause);
		} finally {
			busy = false;
		}
	}
</script>

<form class="grid gap-4 sm:grid-cols-2" onsubmit={download} novalidate>
	<Field label={m.settings_sla_from()} for="sla-from" help={m.settings_sla_period_help()}>
		<input id="sla-from" type="date" class="input" bind:value={from} max={to} required disabled={busy} />
	</Field>
	<Field label={m.settings_sla_to()} for="sla-to">
		<input id="sla-to" type="date" class="input" bind:value={to} min={from} required disabled={busy} />
	</Field>
	<div class="flex flex-wrap gap-2 sm:col-span-2">
		<Button type="button" variant="ghost" size="sm" onclick={lastMonth} disabled={busy}>{m.settings_sla_last_month()}</Button>
		<Button type="button" variant="ghost" size="sm" onclick={thisMonth} disabled={busy}>{m.settings_sla_this_month()}</Button>
	</div>

	<Field label={m.settings_report_timezone()} for="sla-tz">
		<input id="sla-tz" type="text" class="input font-mono" list="sla-tz-list" bind:value={timezone} autocomplete="off" spellcheck="false" disabled={busy} />
		<datalist id="sla-tz-list">
			{#each timezones as zone (zone)}<option value={zone}></option>{/each}
		</datalist>
	</Field>
	<Field label={m.settings_sla_format()} for="sla-format">
		<select id="sla-format" class="input" bind:value={format} disabled={busy}>
			{#each FORMATS as option (option.value)}<option value={option.value}>{option.label}</option>{/each}
		</select>
	</Field>
	<Field label={m.settings_report_objective()} for="sla-objective" help={m.settings_report_objective_help()}>
		<input id="sla-objective" type="text" inputmode="decimal" class="input" bind:value={objective} placeholder="99.9" autocomplete="off" disabled={busy} />
	</Field>
	<Field label={m.settings_report_brand()} for="sla-brand" help={m.settings_report_brand_help()}>
		<select id="sla-brand" class="input" bind:value={pageId} disabled={busy}>
			<option value="">{m.settings_report_brand_none()}</option>
			{#each pages as statusPage (statusPage.id)}<option value={String(statusPage.id)}>{statusPage.title}</option>{/each}
		</select>
	</Field>

	{#if clients.length > 0}
		<Field label={m.reports_sla_client()} for="sla-client" help={m.reports_sla_client_help()}>
			<select id="sla-client" class="input" bind:value={clientId} disabled={busy}>
				{#if mspAdmin}<option value="">{m.reports_sla_client_all()}</option>{/if}
				{#each clients as client (client.id)}<option value={String(client.id)}>{client.name}</option>{/each}
			</select>
		</Field>
	{/if}

	<div class="sm:col-span-2">
		<Button type="submit" variant="primary" loading={busy} disabled={!from || !to}>
			<Download class="size-4" aria-hidden="true" />
			{m.settings_sla_download()}
		</Button>
	</div>
</form>

{#if error}
	<div class="mt-3" aria-live="polite"><ErrorNotice {error} title={m.settings_sla_error()} /></div>
{/if}
