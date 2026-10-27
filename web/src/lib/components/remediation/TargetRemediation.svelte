<script lang="ts">
	/**
	 * On a device's page: the runbooks that apply to it (folder, device type)
	 * and its recent remediation runs. Hidden when both are empty.
	 */
	import { onMount } from 'svelte';
	import type { TargetRemediation } from '#lib/api/index.js';
	import { getTargetRemediation } from '#lib/api/remediation.js';
	import { Disclosure } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import Markdown from './Markdown.svelte';
	import RunList from './RunList.svelte';

	interface Props {
		targetId: number;
	}

	let { targetId }: Props = $props();
	let data = $state<TargetRemediation | null>(null);

	onMount(() => {
		getTargetRemediation(targetId)
			.then((found) => (data = found))
			.catch(() => (data = null));
	});
</script>

{#if data && (data.runbooks.length > 0 || data.runs.length > 0)}
	<section class="mt-6" aria-labelledby="device-remediation">
		<h2 id="device-remediation" class="mb-3 text-base font-semibold tracking-tight text-ink">{m.remediation_target_title()}</h2>
		<div class="space-y-2">
			{#each data.runbooks as runbook (runbook.id)}
				<Disclosure title={runbook.title}>
					<div class="px-4 pb-3"><Markdown source={runbook.body} /></div>
				</Disclosure>
			{/each}
			{#if data.runs.length > 0}
				<div class="rounded-[var(--radius-card)] border border-line bg-surface px-4 py-2">
					<RunList runs={data.runs} showTarget={false} />
				</div>
			{/if}
		</div>
	</section>
{/if}
