<script lang="ts">
	/**
	 * The app shell. On the left a rail with the five destinations, folded to
	 * icons by default (each name shows as a tooltip on hover or focus). The
	 * logo at its head is the fold switch, as in a chat app: folded, it turns
	 * into the "open the menu" icon under the pointer or the focus; unfolded,
	 * the logo and the name lead home and a "close the menu" icon sits at the
	 * right (Ctrl/⌘ B does the same). The choice is remembered in this browser.
	 * Settings, wall mode, the docs, GitHub and the account share the foot of
	 * the rail. Above the page a header: the pigeon (asleep, watching or on
	 * alert, like the network), the breadcrumb, LIVE, the client picker in MSP
	 * mode, the time range on the Overview, the command palette and the main
	 * action.
	 *
	 * Phones get the same five destinations as a bottom tab bar and the account
	 * in the header; wall mode and the docs are in Settings there.
	 *
	 * The rail is `sticky` in the shell's own box, so nothing else may sit under
	 * that box in the page flow (it would shorten the rail's course and push it
	 * up at the end of the scroll): the version tag lives in the page column.
	 */
	import { tick, type Snippet } from 'svelte';
	import { page } from '$app/state';
	import { scale } from 'svelte/transition';
	import { BookOpen, Building2, PanelLeftClose, PanelLeftOpen, Plus, Search, Tv } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { palette } from '#lib/stores/palette.svelte.js';
	import { instanceStore } from '#lib/stores/instance.svelte.js';
	import { CONSOLE_RANGES, consoleState } from '#lib/stores/console.svelte.js';
	import { calmMotion, sparkle } from '#lib/ui/index.js';
	import UserMenu from '#lib/components/UserMenu.svelte';
	import GithubMark from '#lib/components/GithubMark.svelte';
	import Logo from '#lib/components/Logo.svelte';
	import ClientSwitcher from '#lib/components/ClientSwitcher.svelte';
	import VersionTag from '#lib/components/VersionTag.svelte';
	import { isTypingTarget } from '#lib/palette/logic.js';
	import PigeonStatus from './PigeonStatus.svelte';
	import { CLIENTS_ENTRY, NAV, homeHref, isActive, visible, type NavEntry } from './nav.js';

	let { children }: { children: Snippet } = $props();

	const DOCS_URL = 'https://dumbmonit.readthedocs.io/en/latest/';
	const GITHUB_URL = 'https://github.com/laupernoe/dumbmonit';
	/** '1' once the reader unfolded the rail; folded otherwise. */
	const RAIL_KEY = 'dumbmonit-rail-expanded';

	function readExpanded(): boolean {
		try {
			return localStorage.getItem(RAIL_KEY) === '1';
		} catch {
			return false;
		}
	}

	let expanded = $state(readExpanded());
	let railEl = $state<HTMLElement | null>(null);
	/** The two switches (folded: the logo; unfolded: the icon at the right), one in the DOM at a time. */
	let openButton = $state<HTMLButtonElement | null>(null);
	let closeButton = $state<HTMLButtonElement | null>(null);
	/** What the switches show beside their name: the keyboard shortcut. */
	const toggleHint = palette.isMac ? '⌘B' : 'Ctrl B';

	/** From a switch, the focus follows to the one that replaces it. */
	async function toggleRail(fromSwitch = false) {
		expanded = !expanded;
		try {
			if (expanded) localStorage.setItem(RAIL_KEY, '1');
			else localStorage.removeItem(RAIL_KEY);
		} catch {
			// Not remembered: the rail is folded again on the next visit.
		}
		if (!fromSwitch) return;
		await tick();
		(expanded ? closeButton : openButton)?.focus();
	}

	/** Ctrl/⌘ B, like the sidebars of editors and chat apps; never while typing or under a dialog. */
	function onWindowKeydown(event: KeyboardEvent) {
		if (event.defaultPrevented || event.repeat || event.key.toLowerCase() !== 'b') return;
		if (event.altKey || event.shiftKey || !(event.metaKey || event.ctrlKey)) return;
		// Phones have no rail (it is hidden): nothing to fold.
		if (!railEl || railEl.getClientRects().length === 0) return;
		if (palette.isOpen || palette.helpOpen || isTypingTarget(event.target as HTMLElement | null)) return;
		if (document.querySelector('[aria-modal="true"], dialog[open]')) return;
		event.preventDefault();
		// Focus on one of the switches stays on the switch that takes its place.
		void toggleRail(document.activeElement === openButton || document.activeElement === closeButton);
	}
	/** Escape hides the rail's tooltips until the focus or the pointer moves on. */
	let tipsOff = $state(false);

	$effect(() => {
		void instanceStore.loadSettings();
	});

	const entries = $derived(NAV.filter(visible));
	/** The rail's main list; the `foot` entries (Settings) go in its bottom group. */
	const mainEntries = $derived(entries.filter((entry) => !entry.foot));
	const footEntries = $derived(entries.filter((entry) => entry.foot));
	const badge = $derived(alertsStore.available && !alertsStore.loading ? alertsStore.activeCount : 0);
	const path = $derived(page.url.pathname);
	const current = $derived(entries.find((entry) => isActive(entry, path)) ?? null);
	const onOverview = $derived(path === '/');
	const home = $derived(homeHref());
	const showClients = $derived(visible(CLIENTS_ENTRY));
	// Accounts limited by their teams have no wall (the layout sends them back).
	const showWall = $derived(!auth.scoped);
	// Not on the wall (a kiosk with nothing to spare at its edges) nor on the Status pages.
	const showVersion = $derived(!path.startsWith('/wall') && !path.startsWith('/status'));

	/** Pages off the rail still get a crumb. */
	const offRail = $derived(
		path === '/explore' || path.startsWith('/explore/') ? m.explore_title() : path === '/clients' ? m.msp_nav_clients() : null
	);

	/** The third crumb: the monitor's name on its own page. */
	const leaf = $derived.by(() => {
		const match = /^\/targets\/(\d+)/.exec(path);
		if (!match) return null;
		return alertsStore.targets.find((t) => t.id === Number(match[1]))?.name ?? null;
	});

	// LIVE tapped seven times in a row: a note for the night shift, for a moment.
	const beats = $derived(
		Math.round(alertsStore.targets.filter((t) => t.enabled).reduce((sum, t) => sum + 60 / Math.max(1, t.interval_secs), 0))
	);
	$effect(() => {
		if (!consoleState.egg) return;
		const timer = setTimeout(() => (consoleState.egg = false), 6000);
		return () => clearTimeout(timer);
	});

	function skipToContent(event: MouseEvent) {
		const main = document.getElementById('main');
		if (!main) return;
		event.preventDefault();
		main.focus({ preventScroll: true });
		main.scrollIntoView({ block: 'start' });
	}

	const badgeOut = () => ({ duration: calmMotion() ? 0 : 200, start: 0.4 });
	const itemClass = (active: boolean) =>
		`rail-item relative flex h-10 w-full min-w-0 items-center gap-3 rounded-[10px] px-2.5 text-sm font-medium transition-colors duration-150 ${active ? 'bg-surface-2 text-ink' : 'text-ink-2 hover:bg-surface-2 hover:text-ink'}`;
	/** Unfolded, the name sits beside the icon; folded, it becomes the tooltip. */
	const labelClass = $derived(expanded ? 'rail-label min-w-0 flex-1 truncate text-left' : 'rail-tip');
	const iconButton =
		'flex size-10 shrink-0 items-center justify-center rounded-lg text-ink-2 transition-colors duration-150 hover:bg-surface-2 hover:text-ink';
</script>

{#snippet railLink(entry: NavEntry)}
	{@const active = isActive(entry, path)}
	<a href={entry.href} class={itemClass(active)} aria-current={active ? 'page' : undefined} data-tour={entry.tour}>
		<entry.icon class="size-5 shrink-0" strokeWidth={1.75} aria-hidden="true" />
		<span class={labelClass}>{entry.label()}</span>
		{#if entry.badge && badge > 0}
			<span
				transition:scale={badgeOut()}
				aria-hidden="true"
				class={`flex h-4 min-w-4 items-center justify-center rounded-full bg-warning px-1 font-mono text-[0.625rem] font-semibold text-canvas ${expanded ? 'ml-auto' : 'absolute top-1 right-1'}`}
				>{badge}</span
			>
			<span class="sr-only">{m.misc_nav_alerts_badge({ count: badge })}</span>
		{/if}
	</a>
{/snippet}

<a
	href="#main"
	onclick={skipToContent}
	class="sr-only focus:not-sr-only focus:fixed focus:top-2 focus:left-2 focus:z-[100] focus:flex focus:h-10 focus:items-center focus:rounded-lg focus:bg-primary focus:px-4 focus:text-sm focus:font-semibold focus:text-on-primary"
	>{m.shell_skip()}</a
>

<svelte:window onkeydown={onWindowKeydown} />

<div class="flex min-h-dvh bg-canvas text-ink">
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<nav
		id="main-rail"
		bind:this={railEl}
		aria-label={m.misc_nav_main()}
		class={`rail vt-nav-side sticky top-0 z-40 hidden h-dvh shrink-0 flex-col self-start border-r border-line bg-canvas px-2 py-3 transition-[width] duration-200 ease-standard sm:flex ${expanded ? 'w-[13.5rem]' : 'w-14'} ${tipsOff ? 'rail-tips-off' : ''}`}
		onkeydown={(event) => {
			if (event.key === 'Escape') tipsOff = true;
		}}
		onfocusin={() => (tipsOff = false)}
		onpointerleave={() => (tipsOff = false)}
	>
		{#if expanded}
			<!-- Unfolded: the logo and the name lead home; the icon at the right folds the rail. -->
			<a
				href={home}
				class="mb-3 flex h-10 w-[calc(100%-2.75rem)] min-w-0 shrink-0 items-center gap-2.5 rounded-[10px] px-1 transition-colors duration-150 hover:bg-surface-2"
				aria-label={m.misc_nav_home()}
			>
				<Logo class="size-8 shrink-0" />
				<span class="rail-label min-w-0 truncate text-sm font-semibold text-ink" aria-hidden="true">{instanceStore.displayName}</span>
			</a>
			<button
				bind:this={closeButton}
				type="button"
				class="rail-item brand-close absolute top-3 right-2 flex size-10 items-center justify-center rounded-[10px] text-ink-2 transition-colors duration-150 hover:bg-surface-2 hover:text-ink"
				aria-expanded="true"
				aria-controls="main-rail"
				aria-label={m.shell_nav_collapse()}
				aria-keyshortcuts="Control+B Meta+B"
				onclick={() => void toggleRail(true)}
			>
				<PanelLeftClose class="size-5" strokeWidth={1.75} aria-hidden="true" />
				<span class="rail-tip" aria-hidden="true">{m.shell_nav_collapse()}<kbd class="ml-2 rounded border border-line-strong px-1 font-mono text-[0.6875rem] text-ink-2">{toggleHint}</kbd></span>
			</button>
		{:else}
			<!-- Folded: the logo is the switch; under the pointer or the focus it turns into the "open" icon. -->
			<button
				bind:this={openButton}
				type="button"
				class="rail-item brand-open relative mb-3 grid size-10 shrink-0 place-items-center rounded-[10px] transition-colors duration-150 hover:bg-surface-2"
				aria-expanded="false"
				aria-controls="main-rail"
				aria-label={m.shell_nav_expand()}
				aria-keyshortcuts="Control+B Meta+B"
				onclick={() => void toggleRail(true)}
			>
				<span class="brand-logo"><Logo class="size-8" /></span>
				<span class="brand-glyph text-ink"><PanelLeftOpen class="size-5" strokeWidth={1.75} aria-hidden="true" /></span>
				<span class="rail-tip" aria-hidden="true">{m.shell_nav_expand()}<kbd class="ml-2 rounded border border-line-strong px-1 font-mono text-[0.6875rem] text-ink-2">{toggleHint}</kbd></span>
			</button>
		{/if}
		<!-- On a very short window only this list scrolls (the foot below stays in reach). -->
		<ul class="m-0 flex min-h-0 list-none flex-col gap-1 p-0 [@media(max-height:30rem)]:overflow-y-auto [@media(max-height:30rem)]:[scrollbar-width:none]">
			{#each mainEntries as entry (entry.href)}
				<li>{@render railLink(entry)}</li>
			{/each}
		</ul>
		<ul class="m-0 mt-auto flex shrink-0 list-none flex-col gap-1 p-0 pt-2">
			{#each footEntries as entry (entry.href)}
				<li>{@render railLink(entry)}</li>
			{/each}
			{#if showWall}
				<li>
					<a href="/wall" class={itemClass(false)} data-tour="nav-wall" title={expanded ? m.misc_nav_wall_title() : undefined}>
						<Tv class="size-5 shrink-0" strokeWidth={1.75} aria-hidden="true" />
						<span class={labelClass}>{m.misc_nav_wall()}</span>
					</a>
				</li>
			{/if}
			<li>
				<a href={DOCS_URL} target="_blank" rel="noopener" class={itemClass(false)}>
					<BookOpen class="size-5 shrink-0" strokeWidth={1.75} aria-hidden="true" />
					<span class={labelClass}>{m.misc_nav_docs()}<span class="sr-only"> {m.shell_new_tab()}</span></span>
				</a>
			</li>
			<li>
				<a href={GITHUB_URL} target="_blank" rel="noopener" class={itemClass(false)}>
					<GithubMark class="size-5 shrink-0" />
					<span class={labelClass}>GitHub<span class="sr-only"> {m.shell_new_tab()}</span></span>
				</a>
			</li>
			<li><UserMenu placement="rail" wide={expanded} /></li>
		</ul>
	</nav>

	<div class="flex min-w-0 flex-1 flex-col">
		<header class="vt-nav-top sticky top-0 z-30 flex h-14 shrink-0 items-center gap-2 border-b border-line bg-canvas/95 px-4 backdrop-blur sm:gap-3 sm:px-5">
			<PigeonStatus />
			<nav aria-label={m.console_breadcrumb()} class="flex min-w-0 items-center gap-2 text-sm text-ink-2">
				<span class="truncate max-sm:hidden">{instanceStore.displayName}</span>
				{#if current || offRail}
					<span aria-hidden="true" class="max-sm:hidden">/</span>
					{#if current && leaf}
						<a href={current.href} class="truncate hover:text-ink max-sm:hidden">{current.label()}</a>
						<span aria-hidden="true" class="max-sm:hidden">/</span>
						<span class="truncate font-medium text-ink" aria-current="page">{leaf}</span>
					{:else}
						<span class="truncate font-medium text-ink" aria-current="page">{current ? current.label() : offRail}</span>
					{/if}
				{/if}
			</nav>
			<button
				type="button"
				onclick={() => consoleState.tapLive()}
				aria-pressed={consoleState.live}
				aria-label={consoleState.live ? m.console_live_on() : m.console_live_off()}
				title={consoleState.live ? m.console_live_on() : m.console_live_off()}
				class={`flex h-10 shrink-0 items-center gap-1.5 rounded-lg px-2.5 font-mono text-xs font-medium transition-colors duration-200 ${consoleState.live ? 'bg-signal-soft text-signal-ink' : 'bg-surface-2 text-ink-2'}`}
			>
				<span aria-hidden="true" class={`size-1.5 rounded-full ${consoleState.live ? 'ob-live bg-signal' : 'bg-ink-3'}`}></span>
				{consoleState.live ? m.console_live() : m.console_paused()}
			</button>
			{#if consoleState.egg}
				<p role="status" class="rise-in m-0 min-w-0 truncate font-mono text-xs text-signal-ink max-lg:hidden">{m.console_egg({ count: beats })}</p>
			{/if}

			<div class="ml-auto flex min-w-0 items-center gap-2">
				<!-- MSP mode: which client every page shows (hidden when there is nothing to pick). -->
				<div class="empty:hidden max-sm:hidden"><ClientSwitcher id="client-switcher-console" /></div>
				{#if showClients}
					<a href={CLIENTS_ENTRY.href} class={`${iconButton} max-sm:hidden`} aria-label={CLIENTS_ENTRY.label()} title={CLIENTS_ENTRY.label()}>
						<Building2 class="size-[18px]" strokeWidth={1.75} aria-hidden="true" />
					</a>
				{/if}
				{#if onOverview}
					<div role="group" aria-label={m.console_range()} class="flex h-10 overflow-hidden rounded-lg border border-line max-md:hidden">
						{#each CONSOLE_RANGES as range (range)}
							<button
								type="button"
								aria-pressed={consoleState.range === range}
								onclick={() => consoleState.setRange(range)}
								class={`min-w-10 px-2.5 font-mono text-xs font-medium transition-colors duration-150 ${consoleState.range === range ? 'bg-surface-2 text-ink' : 'text-ink-2 hover:text-ink'}`}
							>{range}</button>
						{/each}
					</div>
				{/if}
				<button
					type="button"
					onclick={() => palette.open()}
					class="flex h-10 w-10 items-center justify-center gap-2 rounded-lg border border-line bg-surface px-2.5 text-ink-2 transition-colors duration-150 hover:text-ink lg:w-72 lg:justify-start"
					aria-label={m.misc_nav_palette_open()}
				>
					<Search class="size-4 shrink-0" aria-hidden="true" />
					<span class="flex-1 truncate text-left text-sm max-lg:hidden">{m.console_search_placeholder()}</span>
					<kbd class="rounded border border-line-strong px-1.5 py-px font-mono text-[0.6875rem] text-ink-2 max-lg:hidden">{palette.shortcutLabel}</kbd>
				</button>
				<!-- Monitors and the add page carry their own "Add a monitor": no second one up here. -->
				{#if auth.isAdmin && path !== '/targets' && !path.startsWith('/targets/new')}
					<a
						href="/targets/new"
						use:sparkle
						class="flex h-10 min-w-10 items-center justify-center gap-1.5 rounded-lg bg-primary px-3 text-sm font-semibold text-on-primary transition-[filter] duration-150 hover:brightness-110 max-sm:px-0"
						aria-label={m.console_new_monitor()}
					>
						<Plus class="size-4 sm:hidden" aria-hidden="true" />
						<span class="max-sm:hidden">{m.console_new_monitor()}</span>
					</a>
				{/if}
				<div class="sm:hidden"><UserMenu /></div>
			</div>
		</header>
		<!-- Phones: the client picker gets a strip of its own under the header. -->
		<div class="empty:hidden border-b border-line px-4 py-2 sm:hidden"><ClientSwitcher wide id="client-switcher-phone" /></div>

		<main id="main" tabindex="-1" class="flex min-w-0 flex-1 flex-col px-4 pt-4 pb-[calc(var(--tabbar-h)+1rem+var(--safe-bottom))] outline-none sm:px-6 sm:pt-5 sm:pb-8">
			{@render children()}
		</main>
		{#if showVersion}<VersionTag />{/if}
	</div>
</div>

<!-- Phones: the same five destinations as tabs. -->
<nav aria-label={m.misc_nav_main()} class="vt-nav-bottom fixed inset-x-0 bottom-0 z-30 border-t border-line bg-canvas pr-[var(--safe-right)] pb-[var(--safe-bottom)] pl-[var(--safe-left)] sm:hidden">
	<ul class="m-0 grid list-none p-0" style:grid-template-columns={`repeat(${entries.length}, minmax(0, 1fr))`}>
		{#each entries as entry (entry.href)}
			{@const active = isActive(entry, path)}
			<li class="min-w-0">
				<a
					href={entry.href}
					aria-current={active ? 'page' : undefined}
					data-tour={entry.tour}
					class={`relative flex h-14 flex-col items-center justify-center gap-0.5 text-[0.6875rem] font-medium transition-colors duration-150 ${active ? 'text-ink' : 'text-ink-3'}`}
				>
					{#if active}<span aria-hidden="true" class="state-in absolute top-0 h-0.5 w-8 rounded-full bg-ink"></span>{/if}
					<entry.icon class="size-5" strokeWidth={1.75} aria-hidden="true" />
					<span class="max-w-full truncate px-0.5">{entry.label()}</span>
					{#if entry.badge && badge > 0}
						<span aria-hidden="true" class="absolute top-1.5 right-[calc(50%-1.4rem)] flex h-4 min-w-4 items-center justify-center rounded-full bg-warning px-1 font-mono text-[0.625rem] font-semibold text-canvas">{badge}</span>
						<span class="sr-only">{m.misc_nav_alerts_badge({ count: badge })}</span>
					{/if}
				</a>
			</li>
		{/each}
	</ul>
</nav>

<style>
	/*
	 * The folded rail's switch: the logo and the "open the menu" icon share one
	 * cell. Under the pointer or the keyboard focus the logo fades and shrinks
	 * a little while the icon takes its place (170 ms, the console's curve), so
	 * the logo reads as a button. Nothing moves under reduced motion: the swap
	 * is immediate.
	 */
	.brand-logo,
	.brand-glyph {
		display: grid;
		grid-area: 1 / 1;
		place-items: center;
		transition:
			opacity 170ms var(--ease-standard),
			transform 170ms var(--ease-standard);
	}
	.brand-glyph {
		opacity: 0;
		transform: scale(0.82);
	}
	.brand-open:hover .brand-logo,
	.brand-open:focus-visible .brand-logo {
		opacity: 0;
		transform: scale(0.82);
	}
	.brand-open:hover .brand-glyph,
	.brand-open:focus-visible .brand-glyph {
		opacity: 1;
		transform: scale(1);
	}

	/* The unfolded rail's switch fades in once the rail has nearly reached its width. */
	.brand-close {
		animation: brand-close-in 140ms var(--ease-standard) 100ms both;
	}
	@keyframes brand-close-in {
		from {
			opacity: 0;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.brand-logo,
		.brand-glyph {
			transition: none;
		}
		.brand-close {
			animation: none;
		}
	}
</style>
