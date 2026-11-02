<script lang="ts">
	/**
	 * The pigeon in the header, small, reading the network like the Overview's
	 * banner does: asleep when all is well, watching when something needs a
	 * look, on alert (pinprick eyes and a "!" badge) when a monitor is down.
	 * Its state is in its name for a screen reader, and in a shape, never in
	 * colour alone. It hops once when its state changes; under reduced motion
	 * it only changes face.
	 */
	import { countAlertsByTarget } from '#lib/alerts-attention.js';
	import { m } from '#lib/paraglide/messages.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { reducedMotion } from '#lib/ui/index.js';
	import Mascot from '#lib/components/Mascot.svelte';
	import { bannerLine } from '#lib/components/enterprise/banner.js';
	import { consoleStatus } from '#lib/components/enterprise/status.js';

	type Mood = 'sleep' | 'watch' | 'alert';

	const mood = $derived.by<Mood>(() => {
		if (alertsStore.loading && alertsStore.targets.length === 0) return 'watch';
		const counts = countAlertsByTarget(alertsStore.alerts);
		const monitors = alertsStore.targets.map((target) => ({
			id: target.id,
			name: target.name,
			status: consoleStatus(target, alertsStore.probes.get(target.id), counts.get(target.id))
		}));
		const line = bannerLine(monitors, alertsStore.alerts);
		if (line.kind === 'down') return 'alert';
		if (line.kind === 'attention' || line.kind === 'waiting') return 'watch';
		return 'sleep';
	});

	const label = $derived(
		mood === 'alert' ? m.shell_pigeon_alert() : mood === 'watch' ? m.shell_pigeon_watch() : m.shell_pigeon_sleep()
	);

	let host = $state<HTMLSpanElement | null>(null);
	let previous: Mood | null = null;
	$effect(() => {
		const next = mood;
		if (previous !== null && previous !== next && host && !reducedMotion()) {
			host.animate(
				[{ transform: 'translateY(0)' }, { transform: 'translateY(-3px)', offset: 0.4 }, { transform: 'translateY(0)' }],
				{ duration: 280, easing: 'cubic-bezier(0.2, 0, 0, 1)' }
			);
		}
		previous = next;
	});
</script>

<span bind:this={host} class="relative inline-flex size-8 shrink-0 items-center justify-center" title={label}>
	<Mascot mood={mood} {label} blink={false} class="size-7" />
	{#if mood === 'alert'}
		<span
			aria-hidden="true"
			class="state-in absolute -top-0.5 -right-0.5 grid size-3.5 place-items-center rounded-full bg-warning font-mono text-[0.5625rem] leading-none font-bold text-canvas"
			>!</span
		>
	{/if}
</span>
