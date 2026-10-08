<script lang="ts">
	/**
	 * The four Postfix queues, in the order a mail admin reads them: what just
	 * came in, what is going out now, what failed and will be retried, what a
	 * rule is holding back. The number that matters is the deferred one, and
	 * the age of its oldest message — that is the difference between slow mail
	 * and stuck mail.
	 */
	import type { PmgQueue } from '#lib/api/index.js';
	import { Plate } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { formatCount, formatSpan, queueHelp, queueLabel, queueTone } from './format';

	interface Props {
		queues: PmgQueue[];
	}

	let { queues }: Props = $props();
</script>

{#if queues.length === 0}
	<p class="px-5 py-4 text-sm text-ink-2">{m.devicesb_pmg_queues_empty()}</p>
{:else}
	<div class="grid gap-px bg-line sm:grid-cols-2 lg:grid-cols-4">
		{#each queues as queue (queue.queue)}
			{@const plate = queueTone(queue)}
			<div class="flex flex-col gap-2 bg-surface px-5 py-4">
				<div class="flex items-baseline justify-between gap-2">
					<h3 class="text-sm font-semibold text-ink">{queueLabel(queue.queue)}</h3>
					<Plate tone={plate.tone} label={plate.label} />
				</div>
				<p class="tnum text-2xl font-semibold text-ink">
					{formatCount(queue.messages)}
					<span class="text-sm font-normal text-ink-3">
						{queue.messages === 1 ? m.devicesb_pmg_queues_message_one() : m.devicesb_pmg_queues_message_other()}
					</span>
				</p>
				{#if queue.oldest_age_seconds !== null}
					<p class="tnum text-[0.8125rem] text-ink-2">
						{m.devicesb_pmg_queues_oldest({ span: formatSpan(queue.oldest_age_seconds) })}
					</p>
				{/if}
				{#if queue.domains > 0}
					<p class="text-[0.8125rem] text-ink-3">
						{queue.domains === 1
							? m.devicesb_pmg_queues_domains_one({ count: formatCount(queue.domains) })
							: m.devicesb_pmg_queues_domains_other({ count: formatCount(queue.domains) })}
					</p>
				{/if}
				<p class="text-[0.75rem] text-ink-3">{queueHelp(queue.queue)}</p>
				{#if queue.top_domains.length > 0}
					<ul class="mt-1 flex flex-col gap-0.5 border-t border-line pt-2">
						{#each queue.top_domains.slice(0, 4) as domain (domain.domain)}
							<li class="flex justify-between gap-2 text-[0.8125rem]">
								<span class="truncate text-ink-2" title={domain.domain}>{domain.domain}</span>
								<span class="tnum shrink-0 text-ink-3">{formatCount(domain.messages)}</span>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		{/each}
	</div>
{/if}
