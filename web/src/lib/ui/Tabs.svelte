<script lang="ts" module>
	import type { Icon as LucideIcon } from 'lucide-svelte';

	export interface TabItem {
		id: string;
		label: string;
		icon?: typeof LucideIcon;
		/** A count after the label (alerts needing attention, CVEs). */
		badge?: number | string | null;
		/** The badge asks for attention: warning fill instead of the quiet one. */
		attention?: boolean;
		/** Not offered (and not reachable by `?tab=`): a tab only some roles see. */
		hidden?: boolean;
	}
</script>

<script lang="ts">
	/**
	 * Section tabs of a destination (Monitors, Alerts, Status): the one tab
	 * primitive of the app.
	 *
	 * - The active tab lives in the address (`?tab=<id>`, absent on the first
	 *   tab) so a section is linkable and the back button walks through them.
	 *   Switching drops the other parameters (each tab owns its own), except
	 *   the ones named in `keep`.
	 * - WAI-ARIA tabs: Left/Right/Home/End move and activate, only the active
	 *   tab is in the Tab order.
	 * - Phones: the strip sticks to the top while the panel scrolls under it and
	 *   scrolls sideways when the labels do not fit; the active tab is brought
	 *   into view. `--tabs-h` (the strip's height there, 0 elsewhere) lets a
	 *   sticky toolbar inside the panel stick right under it.
	 * - Motion: the underline slides to the new tab and the panel settles in
	 *   (160 ms, transform and opacity only); both are off under
	 *   `prefers-reduced-motion`.
	 */
	import type { Snippet } from 'svelte';
	import { tick } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { rovingIndex } from './roving.js';

	interface Props {
		tabs: readonly TabItem[];
		/** Accessible name of the tab strip. */
		label: string;
		/** The panel of the active tab. */
		panel: Snippet<[string]>;
		/** Query parameter holding the active tab. */
		param?: string;
		/** Parameters that survive a switch (shared between the tabs). */
		keep?: readonly string[];
		class?: string;
	}

	let { tabs, label, panel, param = 'tab', keep = [], class: className = '' }: Props = $props();

	const uid = $props.id();
	const visible = $derived(tabs.filter((tab) => !tab.hidden));
	const first = $derived(visible[0]?.id ?? '');
	const active = $derived.by(() => {
		const asked = page.url.searchParams.get(param);
		return visible.find((tab) => tab.id === asked)?.id ?? first;
	});

	const tabId = (id: string) => `${uid}-tab-${id}`;

	function select(id: string) {
		if (id === active) return;
		const url = new URL(page.url);
		const kept = new URLSearchParams();
		for (const name of keep) {
			for (const value of url.searchParams.getAll(name)) kept.append(name, value);
		}
		if (id !== first) kept.set(param, id);
		url.search = kept.toString();
		url.hash = '';
		void goto(url, { reset: false });
	}

	function onkeydown(event: KeyboardEvent) {
		const next = rovingIndex(
			event.key,
			visible.findIndex((tab) => tab.id === active),
			visible.length
		);
		if (next === null) return;
		event.preventDefault();
		const id = visible[next].id;
		select(id);
		void tick().then(() => document.getElementById(tabId(id))?.focus());
	}

	// --- The sliding underline ---------------------------------------------

	let strip = $state<HTMLElement>();
	let bar = $state({ x: 0, w: 0, shown: false, animated: false });

	function measure() {
		const el = strip?.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]');
		if (!el) return;
		bar = { ...bar, x: el.offsetLeft, w: el.offsetWidth, shown: true };
		// Phones: a strip wider than the screen scrolls; the active tab stays in view.
		if (strip && strip.scrollWidth > strip.clientWidth) {
			const left = el.offsetLeft - 16;
			const right = el.offsetLeft + el.offsetWidth + 16 - strip.clientWidth;
			if (left < strip.scrollLeft) strip.scrollLeft = Math.max(0, left);
			else if (right > strip.scrollLeft) strip.scrollLeft = right;
		}
	}

	$effect(() => {
		void active;
		void visible.length;
		void tick().then(measure);
	});

	$effect(() => {
		if (!strip) return;
		const observer = new ResizeObserver(measure);
		observer.observe(strip);
		// The labels change width once the font is in.
		void document.fonts?.ready.then(measure);
		// No slide on the very first paint: the bar is simply there.
		const frame = requestAnimationFrame(() => requestAnimationFrame(() => (bar.animated = true)));
		return () => {
			observer.disconnect();
			cancelAnimationFrame(frame);
		};
	});
</script>

<div class={`tabs-root ${className}`}>
	<div class="tabs-bar max-sm:sticky max-sm:top-[var(--safe-top)] max-sm:z-20 max-sm:-mx-4 max-sm:bg-canvas max-sm:px-4">
		<div
			bind:this={strip}
			role="tablist"
			aria-label={label}
			class="tabs-strip relative flex gap-0.5 overflow-x-auto border-b border-line sm:gap-1"
		>
			{#each visible as tab (tab.id)}
				{@const on = tab.id === active}
				{@const Icon = tab.icon}
				<button
					type="button"
					role="tab"
					id={tabId(tab.id)}
					aria-selected={on}
					aria-controls={`${uid}-panel`}
					tabindex={on ? 0 : -1}
					{onkeydown}
					onclick={() => select(tab.id)}
					class={`inline-flex min-h-11 shrink-0 items-center gap-2 rounded-t-md px-3 text-sm font-medium whitespace-nowrap transition-colors duration-150 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ink sm:px-3.5 ${on ? 'text-ink' : 'text-ink-2 hover:text-ink'}`}
				>
					{#if Icon}<Icon class="size-4 shrink-0 max-sm:hidden" aria-hidden="true" />{/if}
					{tab.label}
					{#if tab.badge !== undefined && tab.badge !== null && tab.badge !== 0 && tab.badge !== ''}
						<span
							class={`tnum inline-flex min-w-5 items-center justify-center rounded-full px-1.5 text-[0.6875rem] font-semibold ${tab.attention ? 'bg-warning text-white' : 'bg-surface-2 text-ink-2'}`}
						>
							{tab.badge}
						</span>
					{/if}
				</button>
			{/each}
			<span
				class="pointer-events-none absolute bottom-0 left-0 h-0.5 rounded-full bg-ink"
				class:tabs-bar-animated={bar.animated}
				style:width={`${bar.w}px`}
				style:transform={`translateX(${bar.x}px)`}
				style:opacity={bar.shown ? 1 : 0}
				aria-hidden="true"
			></span>
		</div>
	</div>

	<div class="mt-4 max-sm:mt-3">
		{#key active}
			<div role="tabpanel" id={`${uid}-panel`} aria-labelledby={tabId(active)} class="tabs-panel">
				{@render panel(active)}
			</div>
		{/key}
	</div>
</div>

<style>
	.tabs-root {
		--tabs-h: 0px;
	}
	@media (max-width: 639px) {
		.tabs-root {
			/* min-h-11 plus the strip's bottom border. */
			--tabs-h: calc(2.75rem + 1px);
		}
	}
	.tabs-strip {
		scrollbar-width: none;
	}
	.tabs-strip::-webkit-scrollbar {
		display: none;
	}
	.tabs-bar-animated {
		transition:
			transform 200ms cubic-bezier(0.2, 0, 0, 1),
			width 200ms cubic-bezier(0.2, 0, 0, 1);
	}
	.tabs-panel {
		animation: tabs-in 160ms cubic-bezier(0.2, 0, 0, 1) backwards;
	}
	@keyframes tabs-in {
		from {
			opacity: 0;
			transform: translateY(4px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.tabs-bar-animated {
			transition: none;
		}
		.tabs-panel {
			animation: none;
		}
	}
</style>
