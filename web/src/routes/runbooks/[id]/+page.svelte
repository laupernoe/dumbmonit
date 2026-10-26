<script lang="ts">
	/**
	 * One runbook, as linked from notifications: its text (HTML escaped) and
	 * what it applies to.
	 */
	import { page } from '$app/state';
	import type { Runbook } from '#lib/api/index.js';
	import { getRunbook } from '#lib/api/remediation.js';
	import { ErrorNotice, PageHeader, Panel, Skeleton } from '#lib/ui/index.js';
	import Markdown from '#lib/components/remediation/Markdown.svelte';
	import { formatDateTime } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';

	let runbook = $state<Runbook | null>(null);
	let error = $state<unknown>(null);

	const id = $derived(Number(page.params.id));

	$effect(() => {
		const wanted = id;
		runbook = null;
		error = null;
		getRunbook(wanted)
			.then((found) => (runbook = found))
			.catch((cause) => (error = cause));
	});

	function scopeLabel(scope: string): string {
		if (scope === 'rule') return m.remediation_runbooks_scope_rule();
		if (scope === 'group') return m.remediation_runbooks_scope_group();
		return m.remediation_runbooks_scope_kind();
	}
</script>

<svelte:head><title>{runbook?.title ?? m.remediation_runbooks_title()} · DumbMonit</title></svelte:head>

<a href="/actions#runbooks" class="mb-2 inline-flex min-h-10 items-center text-sm text-ink-2 hover:text-ink hover:underline">← {m.remediation_runbooks_back()}</a>

{#if error}
	<ErrorNotice {error} />
{:else if !runbook}
	<Skeleton class="h-40 w-full" />
{:else}
	<PageHeader title={runbook.title} description={formatDateTime(runbook.updated_at)} />
	<Panel>
		<Markdown source={runbook.body} />
		{#if runbook.links.length > 0}
			<p class="mt-4 flex flex-wrap gap-1 border-t border-line pt-3 text-[0.75rem] text-ink-2">
				{#each runbook.links as link (link.scope + ':' + link.value)}
					<span class="rounded-full border border-line px-2 py-0.5">{scopeLabel(link.scope)}: {link.value}</span>
				{/each}
			</p>
		{/if}
	</Panel>
{/if}
