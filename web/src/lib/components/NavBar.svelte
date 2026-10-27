<script lang="ts">
	/**
	 * Homelab shell. Top bar on desktop, bottom tab bar on phones. The active
	 * link carries a sliding pill that springs between items (the gooey nav,
	 * tamed). The bar holds the essentials only — Overview, Monitors, Alerts,
	 * Settings; every other page (security, map, logs, status pages, wall
	 * mode, documentation, GitHub) sits under "More" and in the command
	 * palette. The right-hand cluster keeps the palette, the theme and the
	 * account menu. Phones get the four destinations only. The version number
	 * lives in `VersionTag.svelte`, fixed at a page corner, not here.
	 */
	import { page } from '$app/state';
	import { untrack } from 'svelte';
	import { scale } from 'svelte/transition';
	import { Search, BookOpen, ChevronDown } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Menu, RollingNumber, bump, calmMotion } from '#lib/ui/index.js';
	import { NAV, isActive as entryActive } from './shell/nav.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { palette } from '#lib/stores/palette.svelte.js';
	import Logo from './Logo.svelte';
	import ThemeToggle from './ThemeToggle.svelte';
	import UserMenu from './UserMenu.svelte';
	import ClientSwitcher from './ClientSwitcher.svelte';

	const DOCS_URL = 'https://dumbmonit.readthedocs.io/en/latest/';
	const GITHUB_URL = 'https://github.com/laupernoe/dumbmonit';

	// A function: the labels follow the active language at render time.
	const links = () =>
		NAV.filter((entry) => entry.essential && visible(entry)).map((entry) => ({
			href: entry.href,
			label: entry.label(),
			icon: entry.icon,
			exact: entry.exact ?? false,
			phone: entry.phone ?? false
		}));
	/** Everything else: one click away under "More", and in the command palette. */
	const more = () => NAV.filter((entry) => !entry.essential && visible(entry));
	const phoneLinks = $derived(links().filter((link) => link.phone));
	/** MSP mode with something to pick: same rule as `ClientSwitcher`. */
	const switchable = $derived.by(() => {
		const count = auth.msp?.clients.length ?? 0;
		return auth.mspOn && (auth.msp?.admin ? count > 0 : count > 1);
	});

	function isActive(href: string, exact: boolean): boolean {
		const path = page.url.pathname;
		if (href === '/settings' && path.startsWith('/settings/status')) return false;
		return exact ? path === href : path.startsWith(href);
	}
	const moreActive = $derived(more().some((entry) => entryActive(entry, page.url.pathname)));

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
	const badgeOut = () => ({ duration: calmMotion() ? 0 : 220, start: 0.4 });

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

<!-- Phones have no top bar: the client switcher gets a strip of its own. -->
{#if switchable}
	<div class="sticky top-0 z-30 border-b border-line bg-canvas px-4 pt-[var(--safe-top)] sm:hidden">
		<div class="py-1.5"><ClientSwitcher wide id="client-switcher-phone" /></div>
	</div>
{/if}

<header class="vt-nav-top sticky top-0 z-30 hidden border-b pt-[var(--safe-top)] border-line bg-canvas/85 backdrop-blur-md sm:block">
	<div class="mx-auto flex h-14 max-w-7xl items-center gap-6 px-4 sm:px-6">
		<div class="flex shrink-0 items-center gap-2">
			<a href="/" class="flex items-center gap-2.5" aria-label={m.misc_nav_home()}>
				<Logo class="size-7" />
				<span class="text-[1.05rem] font-bold tracking-tight text-ink">DumbMonit</span>
			</a>
		</div>

		<nav aria-label={m.misc_nav_main()} class="relative">
			<ul bind:this={list} class="relative flex items-center gap-1">
				<li
					class="pointer-events-none absolute top-0 h-full rounded-lg bg-surface-2 transition-[transform,width,opacity] duration-500 ease-spring"
					style:transform={`translateX(${pill.x}px)`}
					style:width={`${pill.w}px`}
					style:opacity={pill.ready ? 1 : 0}
					aria-hidden="true"
				></li>
				{#each links() as link (link.href)}
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
								<span bind:this={badgeTop} in:scale={badgeOut()} out:scale={badgeOut()} class="tnum ml-0.5 inline-flex h-5 min-w-5 items-center justify-center rounded-full bg-warning px-1.5 text-[0.6875rem] font-bold text-white" aria-label={m.misc_nav_alerts_badge({ count: badge })}><RollingNumber value={badge} /></span>
							{/if}
						</a>
					</li>
				{/each}
			</ul>
		</nav>

		<Menu label={m.misc_nav_more()} align="left">
			{#snippet trigger({ toggle, open })}
				<button
					type="button"
					onclick={toggle}
					aria-expanded={open}
					aria-haspopup="menu"
					data-tour="nav-wall"
					class={`flex h-9 items-center gap-1 rounded-lg px-3 text-sm font-semibold transition-colors hover:bg-surface-2 ${moreActive ? 'text-ink' : 'text-ink-2 hover:text-ink'}`}
				>
					{m.misc_nav_more()}
					<ChevronDown class={`size-3.5 transition-transform ${open ? 'rotate-180' : ''}`} aria-hidden="true" />
				</button>
			{/snippet}
			{#snippet children({ close })}
				<div class="grid min-w-56 p-1">
					{#each more() as entry (entry.href)}
						<a
							role="menuitem"
							href={entry.href}
							onclick={close}
							aria-current={entryActive(entry, page.url.pathname) ? 'page' : undefined}
							class="flex min-h-10 items-center gap-2.5 rounded-md px-2.5 text-sm text-ink hover:bg-surface-2"
						>
							<entry.icon class="size-4 text-ink-2" aria-hidden="true" />{entry.label()}
						</a>
					{/each}
					<div class="my-1 border-t border-line" role="separator"></div>
					<a role="menuitem" href={DOCS_URL} target="_blank" rel="noopener" onclick={close} class="flex min-h-10 items-center gap-2.5 rounded-md px-2.5 text-sm text-ink hover:bg-surface-2" title={m.misc_nav_docs_title()}>
						<BookOpen class="size-4 text-ink-2" aria-hidden="true" />{m.misc_nav_docs()}
					</a>
					<a role="menuitem" href={GITHUB_URL} target="_blank" rel="noopener" onclick={close} class="flex min-h-10 items-center gap-2.5 rounded-md px-2.5 text-sm text-ink hover:bg-surface-2" title={m.misc_nav_github_title()}>
						<!-- GitHub mark from Simple Icons (CC0-1.0): lucide no longer ships brand icons. -->
						<svg viewBox="0 0 24 24" fill="currentColor" class="size-4 text-ink-2" aria-hidden="true"><path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"/></svg>
						GitHub
					</a>
				</div>
			{/snippet}
		</Menu>

		<div class="ml-auto flex items-center gap-1">
			<ClientSwitcher />
			<button
				type="button"
				class="inline-flex size-9 items-center justify-center rounded-lg text-ink-2 transition-colors hover:bg-surface-2 hover:text-ink"
				onclick={() => palette.open()}
				aria-label={m.misc_nav_palette_open()}
				title={m.misc_nav_palette_title({ shortcut: palette.shortcutLabel })}
			>
				<Search class="size-4" aria-hidden="true" />
			</button>
			<ThemeToggle />
			<UserMenu />
		</div>
	</div>
</header>

<!--
	Phones: the four destinations as thumb-reachable tabs, nothing else. Wall
	mode and the documentation sit at the foot of Settings. A solid bar: a
	backdrop blur over a scrolling list costs a repaint every frame.
-->
<nav aria-label={m.misc_nav_main()} class="vt-nav-bottom fixed inset-x-0 bottom-0 z-30 border-t border-line bg-canvas pr-[var(--safe-right)] pb-[var(--safe-bottom)] pl-[var(--safe-left)] sm:hidden">
	<ul class="grid" style:grid-template-columns={`repeat(${phoneLinks.length}, minmax(0, 1fr))`}>
		{#each phoneLinks as link (link.href)}
			{@const active = isActive(link.href, link.exact)}
			<li class="min-w-0">
				<a
					href={link.href}
					data-tour={`nav-${link.label.toLowerCase()}`}
					aria-current={active ? 'page' : undefined}
					class={`relative flex h-14 flex-col items-center justify-center gap-0.5 px-0.5 text-[0.6875rem] font-semibold ${active ? 'text-signal-ink' : 'text-ink-3'}`}
				>
					<link.icon class="size-5" aria-hidden="true" />
					<span class="max-w-full text-center leading-[1.1] [overflow-wrap:anywhere] hyphens-auto">{link.label}</span>
					{#if link.href === '/alerts' && badge > 0}
						<span bind:this={badgeBottom} in:scale={badgeOut()} out:scale={badgeOut()} class="tnum absolute top-1.5 right-[calc(50%-1.5rem)] inline-flex h-4 min-w-4 items-center justify-center rounded-full bg-warning px-1 text-[0.625rem] font-bold text-white"><RollingNumber value={badge} /></span>
					{/if}
				</a>
			</li>
		{/each}
	</ul>
</nav>
