<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * One service found on an address: product, version, port, and what to do
	 * with it — "Add" (opens the add form pre-filled), "Monitored" (links to the
	 * existing monitor) or nothing when no collector matches. A low-confidence
	 * guess says so in words and is drawn dashed and quiet.
	 */
	import { CircleDot, Lock } from 'lucide-svelte';
	import type { DiscoveredService } from '#lib/api/types.js';
	import { Button, Plate } from '#lib/ui/index.js';
	import KindIcon from '#lib/components/KindIcon.svelte';
	import { kindLabel } from '#lib/kind-label.js';
	import { addUrl } from './discovery-prefill';

	interface Props {
		service: DiscoveredService;
		/** Address (and host name) the service sits on: names the suggested monitor. */
		ip: string;
		hostname?: string | null;
		/** Tighter layout for the graph cards. */
		compact?: boolean;
	}

	let { service, ip, hostname = null, compact = false }: Props = $props();

	const low = $derived(service.confidence === 'low');
	const suggestedName = $derived(hostname ? `${hostname} · ${service.product}` : `${ip} · ${service.product}`);
	const portText = $derived(`${service.port}/${service.protocol}`);
</script>

<div
	class={`flex min-w-0 items-center gap-2.5 ${compact ? 'h-[3.25rem] px-3' : 'min-h-14 px-4 py-2 max-sm:min-h-[3.5rem]'} ${low ? 'border-l-2 border-dashed border-line-strong' : ''}`}
	data-confidence={service.confidence}
>
	<span class={`flex size-8 shrink-0 items-center justify-center rounded-lg border ${service.kind ? 'border-signal/30 bg-signal-soft text-signal-ink' : 'border-line bg-surface-2 text-ink-2'}`}>
		{#if service.kind}<KindIcon kind={service.kind} class="size-4" />{:else}<CircleDot class="size-4" aria-hidden="true" />{/if}
	</span>
	<div class="min-w-0 flex-1">
		<p class={`truncate text-sm font-semibold ${low ? 'text-ink-2' : 'text-ink'}`} title={service.detail ?? undefined}>
			{service.product}{#if service.version}<span class="ml-1 font-normal text-ink-2">{service.version}</span>{/if}
		</p>
		<p class="flex min-w-0 items-center gap-1.5 truncate font-mono text-[0.75rem] text-ink-2">
			<span class="tnum">{portText}</span>
			{#if service.tls}<Lock class="size-3 shrink-0" aria-label={m.deviceform_scan_tls()} />{/if}
			{#if low}<span class="font-sans italic">· {m.deviceform_scan_uncertain()}</span>{/if}
		</p>
	</div>
	<div class="shrink-0">
		{#if service.monitored_target_id}
			<a
				href={`/targets/${service.monitored_target_id}`}
				class="inline-flex max-sm:min-h-10 items-center rounded focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal"
				aria-label={m.deviceform_scan_monitored_open({ name: service.product })}
			>
				<Plate tone="signal" label={m.deviceform_scan_monitored()} />
			</a>
		{:else if service.kind}
			<Button
				size="sm"
				variant="secondary"
				href={addUrl(service.kind, service.prefill, suggestedName)}
				aria-label={m.deviceform_scan_add_aria({ product: service.product, kind: kindLabel(service.kind), ip })}
			>
				{m.deviceform_scan_add()}
			</Button>
		{/if}
	</div>
</div>
