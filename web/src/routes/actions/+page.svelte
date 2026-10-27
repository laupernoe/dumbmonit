<script lang="ts">
	/**
	 * Actions: the remediation log (who ran what, where, when, how it ended)
	 * for everyone, then runbooks, actions and automatic policies, which only
	 * administrators edit. The tab lives in the URL hash so a link can open it.
	 */
	import { onMount } from 'svelte';
	import { replaceState } from '$app/navigation';
	import type { RemediationRun } from '#lib/api/index.js';
	import { listRemediationRuns } from '#lib/api/remediation.js';
	import { Button, ErrorNotice, PageHeader, Panel, Skeleton } from '#lib/ui/index.js';
	import RunList from '#lib/components/remediation/RunList.svelte';
	import RunbooksSection from '#lib/components/remediation/RunbooksSection.svelte';
	import ActionsSection from '#lib/components/remediation/ActionsSection.svelte';
	import PoliciesSection from '#lib/components/remediation/PoliciesSection.svelte';
	import { m } from '#lib/paraglide/messages.js';

	type Tab = 'log' | 'runbooks' | 'actions' | 'policies';
	const TABS: Tab[] = ['log', 'runbooks', 'actions', 'policies'];
	const PAGE = 50;

	let tab = $state<Tab>('log');
	let runs = $state<RemediationRun[] | null>(null);
	let error = $state<unknown>(null);
	let more = $state(false);
	let loadingMore = $state(false);
	let actionsChanged = $state(0);

	function label(name: Tab): string {
		if (name === 'log') return m.remediation_tab_log();
		if (name === 'runbooks') return m.remediation_tab_runbooks();
		if (name === 'actions') return m.remediation_tab_actions();
		return m.remediation_tab_policies();
	}

	function pick(name: Tab) {
		tab = name;
		replaceState(`#${name}`, {});
	}

	async function loadRuns(before?: number) {
		try {
			const page = await listRemediationRuns({ limit: PAGE, before });
			runs = before && runs ? [...runs, ...page] : page;
			more = page.length === PAGE;
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
		const wanted = location.hash.slice(1) as Tab;
		if (TABS.includes(wanted)) tab = wanted;
		void loadRuns();
	});
</script>

<svelte:head><title>{m.remediation_page_title()} · DumbMonit</title></svelte:head>

<PageHeader title={m.remediation_page_title()} description={m.remediation_page_description()} />

<div class="mb-4 flex max-w-full gap-1 overflow-x-auto" role="tablist" aria-label={m.remediation_page_title()}>
	{#each TABS as name (name)}
		<button
			type="button"
			role="tab"
			aria-selected={tab === name}
			class={`min-h-10 shrink-0 rounded-lg px-3 text-sm font-medium transition-colors ${tab === name ? 'bg-surface-2 text-ink' : 'text-ink-2 hover:bg-surface-2 hover:text-ink'}`}
			onclick={() => pick(name)}
		>
			{label(name)}
		</button>
	{/each}
</div>

{#if tab === 'log'}
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
{:else if tab === 'runbooks'}
	<RunbooksSection />
{:else if tab === 'actions'}
	<ActionsSection onchanged={() => actionsChanged++} />
{:else}
	<PoliciesSection refresh={actionsChanged} />
{/if}
