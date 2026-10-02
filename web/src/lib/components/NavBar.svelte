<script lang="ts">
	/**
	 * Top bar on desktop, bottom tab bar on phones. The active link carries a
	 * sliding pill that springs between items (the gooey nav, tamed). The
	 * right-hand cluster holds the command palette, the documentation (Read the
	 * Docs, new tab), wall mode, the theme and the session.
	 */
	import { page } from '$app/state';
	import { untrack } from 'svelte';
	import { scale } from 'svelte/transition';
	import { Gauge, Server, BellRing, Globe, Settings2, Command, Search, LogOut, BookOpen, Tv } from 'lucide-svelte';
	import { getHealth } from '$lib/api';
	import { auth } from '$lib/stores/auth.svelte';
	import { Plate, RollingNumber, bump, reducedMotion } from '$lib/ui';
	import { alertsStore } from '$lib/stores/alerts.svelte';
	import { palette } from '$lib/stores/palette.svelte';
	import Logo from './Logo.svelte';
	import ThemeToggle from './ThemeToggle.svelte';

	const DOCS_URL = 'https://dumbmonit.readthedocs.io/en/latest/';

	const LINKS = [
		{ href: '/', label: 'Overview', icon: Gauge, exact: true },
		{ href: '/targets', label: 'Devices', icon: Server, exact: false },
		{ href: '/alerts', label: 'Alerts', icon: BellRing, exact: false },
		{ href: '/status', label: 'Status', icon: Globe, exact: false },
		{ href: '/settings', label: 'Settings', icon: Settings2, exact: false }
	];

	// Version shown next to the wordmark; read once, a failure just hides it.
	let version = $state<{ number: string; build?: string } | null>(null);
	$effect(() => {
		const controller = new AbortController();
		getHealth(controller.signal)
			.then((health) => (version = { number: health.version, build: health.build }))
			.catch(() => {});
		return () => controller.abort();
	});

	function isActive(href: string, exact: boolean): boolean {
		const path = page.url.pathname;
		return exact ? path === href : path.startsWith(href);
	}

	let list = $state<HTMLUListElement | null>(null);
	let pill = $state({ x: 0, w: 0, ready: false });

	function placePill() {
		if (!list) return;
		const active = list.querySelector<HTMLElement>('[data-active="true"]');
		if (!active) {
			pill = { ...pill, ready: false };
			return;
		}
		const lr = list.getBoundingClientRect();
		const ar = active.getBoundingClientRect();
		pill = { x: ar.left - lr.left, w: ar.width, ready: true };
	}

	const badge = $derived(alertsStore.available && !alertsStore.loading ? alertsStore.activeCount : 0);

	// A count that goes up bumps once, so a new alert is noticed from the
	// corner of the eye; one that goes down just rolls. Clearing to zero
	// shrinks the badge away.
	let badgeTop = $state<HTMLElement | null>(null);
	let badgeBottom = $state<HTMLElement | null>(null);
	let lastBadge = 0;
	$effect(() => {
		const next = badge;
		untrack(() => {
			if (next > lastBadge && lastBadge > 0) {
				bump(badgeTop);
				bump(badgeBottom);
			}
			lastBadge = next;
		});
	});
	const badgeOut = () => ({ duration: reducedMotion() ? 0 : 220, start: 0.4 });

	// The pill follows the route, and also the badge: a count appearing next to
	// "Alerts" shifts every item after it without changing the list's own size.
	$effect(() => {
		page.url.pathname;
		badge;
		requestAnimationFrame(placePill);
	});
	$effect(() => {
		if (!list) return;
		const ro = new ResizeObserver(placePill);
		ro.observe(list);
		// Each item too: web fonts arriving or the badge widening move the pill.
		for (const item of list.querySelectorAll('li')) ro.observe(item);
		return () => ro.disconnect();
	});
</script>

<header class="vt-nav-top sticky top-0 z-30 hidden border-b border-line bg-canvas/85 backdrop-blur-md sm:block">
	<div class="mx-auto flex h-14 max-w-7xl items-center gap-6 px-4 sm:px-6">
		<div class="flex shrink-0 items-center gap-2">
			<a href="/" class="flex items-center gap-2.5" aria-label="DumbMonit, overview">
				<Logo class="size-7" />
				<span class="text-[1.05rem] font-bold tracking-tight text-ink">DumbMonit</span>
			</a>
			{#if version}
				<a
					href="/settings#about"
					class="tnum hidden rounded-md border border-line px-1.5 py-0.5 text-[0.6875rem] font-semibold text-ink-3 transition-colors hover:text-ink md:inline-block"
					title={version.build ? `Version ${version.number}, build ${version.build}` : `Version ${version.number}`}
				>v{version.number}</a>
			{/if}
		</div>

		<nav aria-label="Main" class="relative">
			<ul bind:this={list} class="relative flex items-center gap-1">
				<li
					class="pointer-events-none absolute top-0 h-full rounded-lg bg-surface-2 transition-[transform,width,opacity] duration-500 ease-spring"
					style:transform={`translateX(${pill.x}px)`}
					style:width={`${pill.w}px`}
					style:opacity={pill.ready ? 1 : 0}
					aria-hidden="true"
				></li>
				{#each LINKS as link (link.href)}
					{@const active = isActive(link.href, link.exact)}
					<li class="relative">
						<a
							href={link.href}
							data-tour={`nav-${link.label.toLowerCase()}`}
							data-active={active}
							aria-current={active ? 'page' : undefined}
							class={`relative flex h-9 items-center gap-1.5 rounded-lg px-3 text-sm font-semibold transition-colors ${active ? 'text-ink' : 'text-ink-2 hover:text-ink'}`}
						>
							{link.label}
							{#if link.href === '/alerts' && badge > 0}
								<span bind:this={badgeTop} in:scale={badgeOut()} out:scale={badgeOut()} class="tnum ml-0.5 inline-flex h-5 min-w-5 items-center justify-center rounded-full bg-warning px-1.5 text-[0.6875rem] font-bold text-white" aria-label={`${badge} active alerts`}><RollingNumber value={badge} /></span>
							{/if}
						</a>
					</li>
				{/each}
			</ul>
		</nav>

		<div class="ml-auto flex items-center gap-1">
			<button
				type="button"
				class="inline-flex h-9 items-center gap-1.5 rounded-lg px-2.5 text-[0.8125rem] font-semibold text-ink-2 transition-colors hover:bg-surface-2 hover:text-ink"
				onclick={() => palette.open()}
				aria-label="Open the command palette"
				title={`Command palette (${palette.shortcutLabel})`}
			>
				{#if palette.isMac}
					<Command class="size-4" aria-hidden="true" />
				{:else}
					<Search class="size-4" aria-hidden="true" />
				{/if}
				<span class="tnum">{palette.shortcutLabel}</span>
			</button>
			<a
				href={DOCS_URL}
				target="_blank"
				rel="noopener"
				class="inline-flex h-9 items-center gap-1.5 rounded-lg px-2.5 text-[0.8125rem] font-semibold text-ink-2 transition-colors hover:bg-surface-2 hover:text-ink"
				title="Documentation (opens in a new tab)"
			>
				<BookOpen class="size-4" aria-hidden="true" />
				<span class="hidden lg:inline">Docs</span>
				<span class="sr-only lg:hidden">Documentation, opens in a new tab</span>
			</a>
			<a
				href="/wall"
				data-tour="nav-wall"
				class="inline-flex size-9 items-center justify-center rounded-lg text-ink-2 transition-colors hover:bg-surface-2 hover:text-ink"
				aria-label="Wall mode"
				title="Wall mode: the bulletin full screen, for a monitor in the room"
			>
				<Tv class="size-[18px]" aria-hidden="true" />
			</a>
			<ThemeToggle />
			{#if auth.user}
				<!-- Who is signed in, and with which role: the role decides what the pages offer. -->
				<div class="ml-1 hidden items-center gap-2 pl-2 md:flex" title={`Signed in as ${auth.user.username}`}>
					<span class="max-w-[10rem] truncate text-[0.8125rem] font-semibold text-ink-2">{auth.displayName}</span>
					<Plate tone={auth.isAdmin ? 'signal' : 'ghost'} bare label={auth.isAdmin ? 'Admin' : 'Viewer'} />
				</div>
			{/if}
			{#if auth.canSignOut}
				<button
					type="button"
					class="inline-flex size-9 items-center justify-center rounded-lg text-ink-2 transition-colors hover:bg-surface-2 hover:text-ink"
					onclick={() => void auth.logout()}
					aria-label="Sign out"
					title="Sign out"
				>
					<LogOut class="size-[18px]" aria-hidden="true" />
				</button>
			{/if}
		</div>
	</div>
</header>

<!-- Phones: the five destinations as thumb-reachable tabs. -->
<nav aria-label="Main" class="vt-nav-bottom fixed inset-x-0 bottom-0 z-30 border-t border-line bg-canvas/90 pb-[env(safe-area-inset-bottom)] backdrop-blur-md sm:hidden">
	<ul class="grid grid-cols-5">
		{#each LINKS as link (link.href)}
			{@const active = isActive(link.href, link.exact)}
			<li>
				<a
					href={link.href}
					data-tour={`nav-${link.label.toLowerCase()}`}
					aria-current={active ? 'page' : undefined}
					class={`relative flex h-14 flex-col items-center justify-center gap-0.5 text-[0.6875rem] font-semibold ${active ? 'text-signal-ink' : 'text-ink-3'}`}
				>
					<link.icon class="size-5" aria-hidden="true" />
					{link.label}
					{#if link.href === '/alerts' && badge > 0}
						<span bind:this={badgeBottom} in:scale={badgeOut()} out:scale={badgeOut()} class="tnum absolute top-1.5 right-[calc(50%-1.5rem)] inline-flex h-4 min-w-4 items-center justify-center rounded-full bg-warning px-1 text-[0.625rem] font-bold text-white"><RollingNumber value={badge} /></span>
					{/if}
				</a>
			</li>
		{/each}
	</ul>
</nav>
