<script lang="ts">
	/**
	 * Small version for a monitor's own page: only that monitor's expiry dates,
	 * as a list. Renders nothing when it has none (most monitors), and nothing
	 * on a failure: the page does not depend on it.
	 */
	import { onMount } from 'svelte';
	import { listDeadlines } from '#lib/api/deadlines.js';
	import type { Deadline, TargetId } from '#lib/api/types.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Panel } from '#lib/ui/index.js';
	import DeadlineList from './DeadlineList.svelte';

	interface Props {
		target: TargetId;
		class?: string;
	}
	let { target, class: cls = '' }: Props = $props();

	let items = $state<Deadline[]>([]);

	onMount(() => {
		const controller = new AbortController();
		listDeadlines({ target, signal: controller.signal }).then(
			(rows) => (items = rows),
			() => (items = [])
		);
		return () => controller.abort();
	});
</script>

{#if items.length > 0}
	<Panel title={m.deadlines_target_title()} class={cls}>
		<DeadlineList {items} hideTarget />
	</Panel>
{/if}
