<script lang="ts">
	/**
	 * Pip: the mascot pigeon, perched in a corner of every signed-in page,
	 * occasionally hopping up with a short, context-aware tip in a speech
	 * bubble. Calm on purpose — one hop per area per visit, never on top of
	 * content, dismissible, and silenceable for good.
	 *
	 * Mounted once by the root layout (not on `/wall`, not on the public
	 * pages, both of which skip this branch already). `deviceContext` lets the
	 * device detail and "add a device" pages hand Pip the collector's own
	 * setup notice, so the tutorial it gives there always matches what
	 * `GET /api/collectors` actually describes — never a hardcoded guess.
	 */
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { ArrowRight, RefreshCw, X } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { palette } from '#lib/stores/palette.svelte.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, reducedMotion } from '#lib/ui/index.js';
	import Mascot from '#lib/components/Mascot.svelte';
	import { deviceContext } from './deviceContext.svelte';
	import { AREA_TIPS, areaForPath, hidePipForGood, pipHidden, storeNextTip, takeTipIndex, type Tip } from './tips';

	const area = $derived(areaForPath(page.url.pathname));
	const tips = $derived(AREA_TIPS[area].filter((tip) => auth.isAdmin || !tip.admin));

	// On the device detail and "add a device" pages, the collector's own setup
	// notice becomes a tip of its own, ahead of the generic ones — real
	// guidance for this exact kind, not a canned line.
	const setupTip = $derived.by<Tip | null>(() => {
		if (area !== 'device') return null;
		const info = deviceContext.info;
		if (!info || !info.setup.title) return null;
		return {
			id: `setup:${info.kind}`,
			text: () => (info.setup.steps.length > 0 ? `${info.setup.title}: ${info.setup.steps[0]}` : info.setup.title),
			action: info.setup.doc_url ? () => m.tips_pip_guide({ label: info.label }) : undefined,
			href: info.setup.doc_url || undefined
		};
	});

	let hidden = $state(true);
	let open = $state(false);
	let index = $state(0);
	let bird = $state<HTMLElement | null>(null);
	/** Areas already popped up once this page load — so navigating back and
	 *  forth does not hop Pip up again and again. */
	const shownAreas = new Set<string>();
	let autoTimer: ReturnType<typeof setTimeout> | null = null;

	onMount(() => {
		hidden = pipHidden();
	});

	const list = $derived(setupTip ? [setupTip, ...tips] : tips);
	const tip = $derived(list.length === 0 ? null : list[index % list.length]);

	// A new area resets the rotation to wherever this browser left it (the
	// dynamic setup tip, when there is one, always leads) and, once, offers
	// an unprompted hop — but only if the reader has not already put Pip away.
	$effect(() => {
		const current = area;
		const hasSetup = setupTip !== null;
		index = hasSetup || tips.length === 0 ? 0 : takeTipIndex(current) % tips.length;
		if (autoTimer) clearTimeout(autoTimer);
		if (hidden || shownAreas.has(current)) return;
		autoTimer = setTimeout(
			() => {
				shownAreas.add(current);
				open = true;
			},
			hasSetup ? 900 : 2200
		);
		return () => {
			if (autoTimer) clearTimeout(autoTimer);
		};
	});

	function hop() {
		if (!bird || reducedMotion()) return;
		bird.animate(
			[{ transform: 'translateY(0) rotate(-3deg)' }, { transform: 'translateY(-9px) rotate(4deg)', offset: 0.4 }, { transform: 'translateY(0) rotate(-3deg)' }],
			{ duration: 420, easing: 'cubic-bezier(0.22, 1, 0.36, 1)' }
		);
	}

	function toggle() {
		open = !open;
		if (open) hop();
	}

	function another() {
		index += 1;
		storeNextTip(area, index);
		hop();
	}

	function close() {
		open = false;
	}

	function quiet() {
		hidePipForGood();
		hidden = true;
		open = false;
	}
</script>

{#if !hidden}
	<div class="pointer-events-none fixed bottom-[4.75rem] left-3 z-20 flex flex-col items-start gap-2 sm:bottom-3">
		{#if open && tip}
			<div
				class="pip-bubble pointer-events-auto max-w-[min(20rem,calc(100vw-6rem))] max-sm:max-h-[40dvh] max-sm:overflow-y-auto rounded-[var(--radius-card)] border border-line bg-surface p-3 text-sm shadow-lift"
				role="note"
				aria-label={m.tips_pip_bubble_aria()}
			>
				<div class="flex items-start justify-between gap-2">
					<p class="min-w-0 leading-snug text-ink-2">{tip.text()}</p>
					<button
						type="button"
						class="-mt-1 -mr-1 inline-flex size-6 shrink-0 items-center justify-center rounded-md text-ink-3 transition-colors hover:bg-surface-2 hover:text-ink"
						onclick={close}
						aria-label={m.tips_pip_close_tip_aria()}
						title={m.tips_pip_close_tip_title()}
					>
						<X class="size-3.5" aria-hidden="true" />
					</button>
				</div>
				<div class="mt-2 flex flex-wrap items-center gap-1.5">
					{#if tip.action && tip.href === 'palette'}
						<Button variant="secondary" size="sm" onclick={() => palette.open()}>
							{tip.action()}
							<span class="tnum text-ink-3">{palette.shortcutLabel}</span>
						</Button>
					{:else if tip.action && tip.href}
						<Button variant="secondary" size="sm" href={tip.href} target={tip.href.startsWith('http') ? '_blank' : undefined} rel={tip.href.startsWith('http') ? 'noopener' : undefined}>
							{tip.action()}
							<ArrowRight class="size-3.5" aria-hidden="true" />
						</Button>
					{/if}
					{#if list.length > 1}
						<Button variant="ghost" size="sm" onclick={another} aria-label={m.tips_pip_another()} title={m.tips_pip_another()}>
							<RefreshCw class="size-3.5" aria-hidden="true" />
						</Button>
					{/if}
				</div>
				<button type="button" class="mt-2 block text-[0.75rem] text-ink-3 underline-offset-2 hover:underline" onclick={quiet}>
					{m.tips_pip_quiet()}
				</button>
			</div>
		{/if}
		<button
			bind:this={bird}
			type="button"
			class="pointer-events-auto flex size-11 items-center justify-center rounded-full border border-line bg-surface shadow-lift transition-transform hover:scale-105 active:scale-95"
			onclick={toggle}
			aria-label={open ? m.tips_pip_close_pip() : m.tips_pip_open()}
			aria-expanded={open}
			title={open ? m.tips_pip_close_pip() : m.tips_pip_open()}
		>
			<Mascot class="size-8" mood={open ? 'happy' : 'watch'} />
		</button>
	</div>
{/if}

<style>
	.pip-bubble {
		animation: pip-in 260ms cubic-bezier(0.22, 1, 0.36, 1);
	}
	@keyframes pip-in {
		from {
			opacity: 0;
			transform: translateY(6px) scale(0.98);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.pip-bubble {
			animation: none;
		}
	}
</style>
