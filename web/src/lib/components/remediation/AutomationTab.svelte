<script lang="ts">
	/**
	 * Alerts › Automation: the remediation log (who ran what, where, when, how
	 * it ended) for everyone, then runbooks ("what to do"), actions ("do it")
	 * and automatic policies ("do it on its own"), which only administrators
	 * edit. The section lives in `?view=` so a link can open it.
	 */
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import type { RemediationRun } from '#lib/api/index.js';
	import { listRemediationRuns } from '#lib/api/remediation.js';
	import { Button, ErrorNotice, Panel, Skeleton } from '#lib/ui/index.js';
	import Segmented from '#lib/components/devices/Segmented.svelte';
	import RunList from '#lib/components/remediation/RunList.svelte';
	import RunbooksSection from '#lib/components/remediation/RunbooksSection.svelte';
	import ActionsSection from '#lib/components/remediation/ActionsSection.svelte';
	import PoliciesSection from '#lib/components/remediation/PoliciesSection.svelte';
	import { m } from '#lib/paraglide/messages.js';

	type View = 'log' | 'runbooks' | 'actions' | 'policies';
	const VIEWS: View[] = ['log', 'runbooks', 'actions', 'policies'];
	const PAGE = 50;

	const view = $derived.by(() => {
		const asked = page.url.searchParams.get('view');
		return VIEWS.find((name) => name === asked) ?? 'log';
	});
	let runs = $state<RemediationRun[] | null>(null);
	let error = $state<unknown>(null);
	let more = $state(false);
	let loadingMore = $state(false);
	let actionsChanged = $state(0);

	const options = $derived(
		VIEWS.map((id) => ({
			id,
			label:
				id === 'log'
					? m.remediation_tab_log()
					: id === 'runbooks'
						? m.remediation_tab_runbooks()
						: id === 'actions'
							? m.remediation_tab_actions()
							: m.remediation_tab_policies()
		}))
	);

	function pick(name: View) {
		const url = new URL(page.url);
		if (name === 'log') url.searchParams.delete('view');
		else url.searchParams.set('view', name);
		void goto(url, { replace: true, reset: false });
	}

	async function loadRuns(before?: number) {
		try {
			const batch = await listRemediationRuns({ limit: PAGE, before });
			runs = before && runs ? [...runs, ...batch] : batch;
			more = batch.length === PAGE;
			error = null;
		} catch (cause) {
			error = cause;
		}
	}

	async function older() {
		if (!runs?.length) return;
		loadingMore = true;
		await loadRuns(runs[runs.length - 1].id);
		loadingMore = false;
	}

	onMount(() => {
		void loadRuns();
	});
</script>

<div class="mb-4 flex flex-wrap items-center justify-between gap-x-6 gap-y-3">
	<p class="max-w-2xl text-sm text-ink-2 max-sm:hidden">{m.remediation_page_description()}</p>
	<Segmented {options} value={view} onchange={pick} label={m.alerts_tab_automation()} />
</div>

{#if view === 'log'}
	<Panel title={m.remediation_tab_log()}>
		{#if error}
			<ErrorNotice {error} onretry={() => loadRuns()} />
		{:else if runs === null}
			<Skeleton class="h-24 w-full" />
		{:else}
			<RunList {runs} />
			{#if more}
				<Button size="sm" class="mt-3" loading={loadingMore} onclick={older}>{m.remediation_runs_more()}</Button>
			{/if}
		{/if}
	</Panel>
{:else if view === 'runbooks'}
	<RunbooksSection />
{:else if view === 'actions'}
	<ActionsSection onchanged={() => actionsChanged++} />
{:else}
	<PoliciesSection refresh={actionsChanged} />
{/if}
