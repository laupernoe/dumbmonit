<script lang="ts">
	/** Discreet "where" tag of a monitor: the name of its location, truncated to fit. */
	import { MapPin } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';

	let { name, class: className = '' }: { name: string; class?: string } = $props();
</script>

<span
	class={`inline-flex min-w-0 items-center gap-1 rounded-full border border-line px-1.5 py-px text-[0.6875rem] leading-4 text-ink-3 ${className}`}
	title={m.devices_site_badge_aria({ name })}
>
	<MapPin class="size-3 shrink-0" aria-hidden="true" />
	<span class="truncate" aria-hidden="true">{name}</span>
	<span class="sr-only">{m.devices_site_badge_aria({ name })}</span>
</span>
