<script lang="ts">
	/**
	 * The demo's guided tour: eight stops, each one a page and, when it has
	 * one, an element ringed on it. A modal dialog — focus stays inside, Esc
	 * closes, arrow keys step — docked bottom-right on desktop and as a bottom
	 * sheet on phones, so the highlighted part of the page stays visible.
	 */
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { tick, untrack } from 'svelte';
	import { X, ArrowLeft, ArrowRight } from 'lucide-svelte';
	import { listTargets } from '$lib/api';
	import { Button } from '$lib/ui';
	import { demo } from '$lib/stores/demo.svelte';

	interface Step {
		title: string;
		body: string;
		/** Page to show, `null` to stay where the visitor is. */
		href: string | null | (() => Promise<string | null>);
		/** `data-tour` anchor to ring, first visible match wins. */
		anchor?: string;
		link?: { label: string; href: string };
	}

	/** The Proxmox cluster of the estate, found by kind rather than by id. */
	async function proxmoxPage(): Promise<string | null> {
		try {
			const targets = await listTargets();
			const cluster = targets.find((t) => t.kind === 'proxmox');
			return cluster ? `/targets/${cluster.id}` : '/targets';
		} catch {
			return '/targets';
		}
	}

	const STEPS: Step[] = [
		{
			title: 'The weather, then what needs you',
			body: 'The overview reads like a forecast: one sentence for the whole estate, and below it only the things worth your attention. A quiet sky means you can close the tab.',
			href: '/',
			anchor: 'weather'
		},
		{
			title: 'A device, all of it',
			body: 'This Proxmox cluster shows nodes, guests, storage, replication and backups on one page, from the same API the real collector reads. Every chart has the last seven days.',
			href: proxmoxPage,
			anchor: 'nav-devices'
		},
		{
			title: 'Alerts without the noise',
			body: 'Rules come ready-made. A switch that goes down silences everything behind it; seasonal baselines learn what a normal Tuesday looks like; forecasts warn a disk will fill before it does.',
			href: '/alerts#rules',
			anchor: 'nav-alerts'
		},
		{
			title: 'Notifications, 22 ways',
			body: 'Email, ntfy, Matrix, Discord, Slack, Telegram, Gotify, webhooks and more — with quiet hours and per-device overrides. The demo never sends anything.',
			href: '/alerts#notifications'
		},
		{
			title: 'Add a device',
			body: 'Pick a type from the catalogue — hypervisors, NAS, firewalls, servers, printers, websites — and the form asks only for what that type needs. SNMP devices are recognised on their own.',
			href: '/targets/new'
		},
		{
			title: 'The agent',
			body: 'One small binary for Linux and Windows pushes CPU, disks, services and containers. It can also relay probes from a remote site that the server cannot reach.',
			href: '/settings#agents'
		},
		{
			title: 'Status pages',
			body: 'Publish a public page for your users, with incidents, uptime badges and email subscriptions — without exposing the rest of the instance.',
			href: '/status',
			anchor: 'nav-status'
		},
		{
			title: 'Wall mode',
			body: 'For a screen in the room: the bulletin full screen, readable from across it, with an optional music player. That is the tour — look around, nothing you click can break.',
			href: null,
			anchor: 'nav-wall',
			link: { label: 'Open wall mode', href: '/wall' }
		}
	];

	let index = $state(0);
	let dialog = $state<HTMLDivElement | null>(null);
	let ring = $state<{ top: number; left: number; width: number; height: number } | null>(null);
	let anchorEl: HTMLElement | null = null;
	let returnFocus: HTMLElement | null = null;
	let serial = 0;

	const step = $derived(STEPS[index]);
	const last = $derived(index === STEPS.length - 1);

	function visibleAnchor(name: string): HTMLElement | null {
		for (const el of document.querySelectorAll<HTMLElement>(`[data-tour="${name}"]`)) {
			const box = el.getBoundingClientRect();
			if (box.width > 0 && box.height > 0) return el;
		}
		return null;
	}

	function placeRing() {
		if (!anchorEl || !anchorEl.isConnected) {
			ring = null;
			return;
		}
		const box = anchorEl.getBoundingClientRect();
		ring = { top: box.top - 6, left: box.left - 6, width: box.width + 12, height: box.height + 12 };
	}

	async function show(i: number) {
		const mine = ++serial;
		index = i;
		anchorEl = null;
		ring = null;
		const target = typeof STEPS[i].href === 'function' ? await (STEPS[i].href as () => Promise<string | null>)() : STEPS[i].href;
		if (mine !== serial) return;
		if (target) {
			const [path, hash] = target.split('#');
			if (page.url.pathname !== path) {
				await goto(target, { keepFocus: true, noScroll: false });
			} else if (hash && window.location.hash !== `#${hash}`) {
				window.location.hash = hash;
			}
		}
		await tick();
		const name = STEPS[i].anchor;
		if (!name) return;
		// The page may still be loading its data: look for the anchor a few times.
		for (let attempt = 0; attempt < 10 && mine === serial; attempt++) {
			const el = visibleAnchor(name);
			if (el) {
				anchorEl = el;
				el.scrollIntoView({ block: 'nearest', behavior: reducedMotion() ? 'auto' : 'smooth' });
				placeRing();
				return;
			}
			await new Promise((resolve) => setTimeout(resolve, 150));
		}
	}

	function reducedMotion(): boolean {
		return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
	}

	function close() {
		serial++;
		ring = null;
		anchorEl = null;
		demo.closeTour();
		returnFocus?.focus();
	}

	function next() {
		if (last) close();
		else void show(index + 1);
	}
	function back() {
		if (index > 0) void show(index - 1);
	}

	// Opening: remember who opened it, start at the first stop, focus the dialog.
	$effect(() => {
		if (!demo.tourOpen) return;
		returnFocus = (document.activeElement as HTMLElement | null) ?? null;
		// `show` reads the current URL before its first await: untracked, or every
		// navigation of the tour itself would re-run this effect and restart at step 1.
		untrack(() => void show(0));
		void tick().then(() => dialog?.focus());
	});

	$effect(() => {
		if (!demo.tourOpen) return;
		const update = () => placeRing();
		window.addEventListener('resize', update);
		window.addEventListener('scroll', update, true);
		const timer = setInterval(update, 500);
		return () => {
			window.removeEventListener('resize', update);
			window.removeEventListener('scroll', update, true);
			clearInterval(timer);
		};
	});

	function focusables(): HTMLElement[] {
		if (!dialog) return [];
		return [...dialog.querySelectorAll<HTMLElement>('a[href], button:not([disabled])')];
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			close();
		} else if (event.key === 'ArrowRight') {
			event.preventDefault();
			next();
		} else if (event.key === 'ArrowLeft') {
			event.preventDefault();
			back();
		} else if (event.key === 'Tab') {
			const items = focusables();
			if (items.length === 0) return;
			const first = items[0];
			const final = items[items.length - 1];
			const active = document.activeElement;
			if (event.shiftKey && (active === first || active === dialog)) {
				event.preventDefault();
				final.focus();
			} else if (!event.shiftKey && active === final) {
				event.preventDefault();
				first.focus();
			}
		}
	}
</script>

{#if demo.tourOpen}
	<!-- Scrim: dims the page; the ring cuts the highlighted element out of it. -->
	<div class="tour-scrim fixed inset-0 z-[60]" class:has-ring={ring !== null} aria-hidden="true"></div>
	{#if ring}
		<div
			class="tour-ring pointer-events-none fixed z-[61] rounded-xl border-2 border-signal"
			style:top={`${ring.top}px`}
			style:left={`${ring.left}px`}
			style:width={`${ring.width}px`}
			style:height={`${ring.height}px`}
			aria-hidden="true"
		></div>
	{/if}

	<div
		bind:this={dialog}
		role="dialog"
		aria-modal="true"
		aria-labelledby="tour-title"
		aria-describedby="tour-body"
		tabindex="-1"
		{onkeydown}
		class="tour-card fixed inset-x-0 bottom-0 z-[62] rounded-t-[var(--radius-card)] border border-line bg-surface p-5 pb-[calc(1.25rem+env(safe-area-inset-bottom))] text-ink shadow-float outline-none sm:inset-x-auto sm:right-6 sm:bottom-6 sm:w-[24rem] sm:rounded-[var(--radius-card)] sm:pb-5"
	>
		<div class="flex items-start justify-between gap-3">
			<p class="tnum text-[0.75rem] font-semibold tracking-wide text-ink-3 uppercase">
				Tour · {index + 1} of {STEPS.length}
			</p>
			<button
				type="button"
				class="-mt-1.5 -mr-2 inline-flex size-8 items-center justify-center rounded-md text-ink-3 hover:bg-surface-2 hover:text-ink"
				aria-label="Close the tour"
				onclick={close}
			>
				<X class="size-4" aria-hidden="true" />
			</button>
		</div>
		<h2 id="tour-title" class="mt-1 text-lg font-bold tracking-tight">{step.title}</h2>
		<p id="tour-body" class="mt-2 text-sm leading-relaxed text-ink-2">{step.body}</p>
		{#if step.link}
			<a href={step.link.href} class="mt-2 inline-block text-sm font-semibold text-signal-ink hover:underline" onclick={close}>{step.link.label}</a>
		{/if}

		<div class="mt-4 flex items-center gap-2">
			<div class="flex flex-1 gap-1" aria-hidden="true">
				{#each STEPS as _, i (i)}
					<span class={`h-1.5 rounded-full transition-all ${i === index ? 'w-4 bg-signal' : 'w-1.5 bg-line-strong'}`}></span>
				{/each}
			</div>
			{#if index > 0}
				<Button variant="ghost" size="sm" onclick={back}>
					<ArrowLeft class="size-4" aria-hidden="true" />
					Back
				</Button>
			{/if}
			<Button variant="primary" size="sm" onclick={next}>
				{last ? 'Done' : 'Next'}
				{#if !last}<ArrowRight class="size-4" aria-hidden="true" />{/if}
			</Button>
		</div>
		<p class="sr-only">Use the left and right arrow keys to move between steps, Escape to close.</p>
	</div>
{/if}

<style>
	.tour-scrim {
		background: rgb(10 16 30 / 0.5);
	}
	.tour-scrim.has-ring {
		background: transparent;
	}
	.tour-ring {
		box-shadow: 0 0 0 9999px rgb(10 16 30 / 0.5);
		transition:
			top 0.35s cubic-bezier(0.16, 1, 0.3, 1),
			left 0.35s cubic-bezier(0.16, 1, 0.3, 1),
			width 0.35s cubic-bezier(0.16, 1, 0.3, 1),
			height 0.35s cubic-bezier(0.16, 1, 0.3, 1);
	}
	.tour-card {
		animation: tour-in 0.3s cubic-bezier(0.16, 1, 0.3, 1);
	}
	@keyframes tour-in {
		from {
			opacity: 0;
			transform: translateY(12px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.tour-ring {
			transition: none;
		}
		.tour-card {
			animation: none;
		}
	}
</style>
