<script lang="ts">
	/**
	 * Who is on call right now, one line per schedule: the person, since when,
	 * until when, and a plate when it is a replacement rather than the rotation.
	 */
	import { Phone } from 'lucide-svelte';
	import type { OnCallEntry, Person } from '#lib/api/index.js';
	import { formatDateTime } from '#lib/format.js';
	import { EmptyState, Panel, Plate } from '#lib/ui/index.js';
	import Avatar from './Avatar.svelte';
	import { personName } from './helpers';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		entries: OnCallEntry[];
		people: Person[];
	}
	let { entries, people }: Props = $props();
</script>

<Panel id="oncall-now" title={m.oncall_now_title()} description={m.oncall_now_description()}>
	{#if entries.length === 0}
		<EmptyState icon={Phone} title={m.oncall_now_empty_title()} description={m.oncall_now_empty_description()} class="!border-0 !py-6" />
	{:else}
		<ul class="grid gap-2.5 sm:grid-cols-2">
			{#each entries as entry (entry.schedule_id)}
				<li class="flex min-w-0 items-center gap-3 rounded-lg border border-line bg-canvas-deep px-3 py-2.5">
					{#if entry.user_id !== null}
						<Avatar {people} id={entry.user_id} />
					{:else}
						<span class="inline-flex size-9 shrink-0 items-center justify-center rounded-full border border-dashed border-line-strong text-ink-3" aria-hidden="true">?</span>
					{/if}
					<div class="min-w-0 flex-1">
						<p class="truncate text-[0.8125rem] text-ink-2">{entry.schedule_name}</p>
						{#if entry.user_id !== null}
							<p class="truncate font-semibold text-ink">{personName(people, entry.user_id)}</p>
							<p class="tnum text-[0.75rem] text-ink-2">
								{#if entry.since}{m.oncall_now_since({ when: formatDateTime(entry.since) })}{/if}
								{#if entry.since && entry.until}<span aria-hidden="true"> · </span>{/if}
								{#if entry.until}{m.oncall_now_until({ when: formatDateTime(entry.until) })}{:else}{m.oncall_now_open_ended()}{/if}
							</p>
						{:else}
							<p class="font-semibold text-ink-2">{m.oncall_now_nobody()}</p>
						{/if}
					</div>
					{#if entry.override_id !== null}
						<Plate tone="info" label={m.oncall_now_replacement()} class="shrink-0" />
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
</Panel>
