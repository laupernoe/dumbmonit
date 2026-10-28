<script lang="ts">
	/**
	 * The app shell: an icon rail on the left, a header with the breadcrumb,
	 * LIVE, the time range, the command palette, the main action, wall mode
	 * and the panel toggle, the page in the middle, and the context panel on
	 * the right like an inspector.
	 *
	 * Phones get a bottom tab bar instead of the rail, and the panel opens as a
	 * drawer on anything narrower than a wide desktop. `i` toggles the panel.
	 */
	import type { Snippet } from 'svelte';
	import { tick } from 'svelte';
	import { page } from '$app/state';
	import { scale } from 'svelte/transition';
	import { BookOpen, Ellipsis, Monitor, PanelRight, Plus, Search, X } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { palette } from '#lib/stores/palette.svelte.js';
	import { instanceStore } from '#lib/stores/instance.svelte.js';
	import { CONSOLE_RANGES, consoleState } from '#lib/stores/console.svelte.js';
	import { isTypingTarget } from '#lib/palette/logic.js';
	import { Menu, calmMotion } from '#lib/ui/index.js';
	import ContextPanel from '#lib/components/enterprise/ContextPanel.svelte';
	import UserMenu from '#lib/components/UserMenu.svelte';
	import GithubMark from '#lib/components/GithubMark.svelte';
	import Logo from '#lib/components/Logo.svelte';
	import ClientSwitcher from '#lib/components/ClientSwitcher.svelte';
	import { NAV, isActive, visible, type NavEntry } from './nav.js';

	let { children }: { children: Snippet } = $props();

	const WIDE = '(min-width: 80rem)';
	const DOCS_URL = 'https://dumbmonit.readthedocs.io/en/latest/';
	const GITHUB_URL = 'https://github.com/laupernoe/dumbmonit';

	$effect(() => {
		void instanceStore.loadSettings();
	});

	const entries = $derived(NAV.filter(visible));
	const railTop = $derived(entries.filter((entry) => !entry.foot));
	const railFoot = $derived(entries.filter((entry) => entry.foot));
	const phoneTabs = $derived(entries.filter((entry) => entry.phone));
	const phoneMore = $derived(entries.filter((entry) => !entry.phone));
	const badge = $derived(alertsStore.available && !alertsStore.loading ? alertsStore.activeCount : 0);
	const path = $derived(page.url.pathname);
	const current = $derived(entries.find((entry) => isActive(entry, path)) ?? null);
	const onOverview = $derived(path === '/');

	/** The third crumb: the monitor's name on its own page. */
	const leaf = $derived.by(() => {
		const match = /^\/targets\/(\d+)/.exec(path);
		if (!match) return null;
		return alertsStore.targets.find((t) => t.id === Number(match[1]))?.name ?? null;
	});

	function togglePanel() {
		if (window.matchMedia(WIDE).matches) consoleState.togglePanel();
		else consoleState.drawerOpen = !consoleState.drawerOpen;
	}

	let drawer = $state<HTMLDivElement | null>(null);
	$effect(() => {
		if (consoleState.drawerOpen) void tick().then(() => drawer?.querySelector<HTMLElement>('button, a')?.focus());
	});
	// A navigation closes the drawer.
	$effect(() => {
		path;
		consoleState.drawerOpen = false;
	});

	$effect(() => {
		function onKeydown(event: KeyboardEvent) {
			if (event.key === 'Escape' && consoleState.drawerOpen) {
				consoleState.drawerOpen = false;
				return;
			}
			if (event.key !== 'i' || event.metaKey || event.ctrlKey || event.altKey || event.repeat) return;
			if (isTypingTarget(event.target as HTMLElement | null) || palette.isOpen) return;
			if (document.querySelector('[aria-modal="true"], dialog[open]')) return;
			event.preventDefault();
			togglePanel();
		}
		window.addEventListener('keydown', onKeydown);
		return () => window.removeEventListener('keydown', onKeydown);
	});

	const badgeOut = () => ({ duration: calmMotion() ? 0 : 200, start: 0.4 });
	const railLink = (active: boolean) =>
		`relative flex size-10 items-center justify-center rounded-[10px] transition-colors ${active ? 'bg-surface-2 text-ink' : 'text-ink-2 hover:bg-surface-2 hover:text-ink'}`;
</script>

{#snippet railItem(entry: NavEntry)}
	{@const active = isActive(entry, path)}
	<a href={entry.href} class={railLink(active)} aria-label={entry.label()} title={entry.label()} aria-current={active ? 'page' : undefined}>
		<entry.icon class="size-[18px]" strokeWidth={1.7} aria-hidden="true" />
		{#if entry.badge && badge > 0}
			<span transition:scale={badgeOut()} class="absolute top-1.5 right-1.5 flex h-3.5 min-w-3.5 items-center justify-center rounded-[7px] bg-warning px-[3px] font-mono text-[0.5625rem] font-semibold text-canvas" aria-label={m.misc_nav_alerts_badge({ count: badge })}>{badge}</span>
		{/if}
	</a>
{/snippet}

<div class="flex min-h-dvh bg-canvas text-ink">
	<nav aria-label={m.misc_nav_main()} class="sticky top-0 hidden h-dvh w-14 shrink-0 flex-col items-center gap-1 border-r border-line py-3 sm:flex">
		<a href="/" class="mb-3 flex size-10 shrink-0 items-center justify-center rounded-[10px]" aria-label={m.misc_nav_home()}>
			<Logo class="size-8" />
		</a>
		<!-- Short screens scroll the destinations; the foot (docs, account, Settings) stays put. -->
		<div class="flex min-h-0 flex-1 flex-col items-center gap-1 overflow-y-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
			{#each railTop as entry (entry.href)}
				{@render railItem(entry)}
			{/each}
		</div>
		<div class="flex shrink-0 flex-col items-center gap-1 pt-2">
			<a href={DOCS_URL} target="_blank" rel="noopener" class={railLink(false)} aria-label={m.misc_nav_docs_sr()} title={m.misc_nav_docs_title()}>
				<BookOpen class="size-[18px]" strokeWidth={1.7} aria-hidden="true" />
			</a>
			<a href={GITHUB_URL} target="_blank" rel="noopener" class={railLink(false)} aria-label={m.misc_nav_github()} title={m.misc_nav_github_title()}>
				<GithubMark class="size-[18px]" />
			</a>
			<!-- The account sits just above Settings, the last icon of the rail. -->
			<UserMenu placement="rail" />
			{#each railFoot as entry (entry.href)}
				{@render railItem(entry)}
			{/each}
		</div>
	</nav>

	<div class="flex min-w-0 flex-1 flex-col">
		<header class="sticky top-0 z-30 flex h-[52px] shrink-0 items-center gap-3 border-b border-line bg-canvas/95 px-4 backdrop-blur sm:px-5">
			<nav aria-label={m.console_breadcrumb()} class="flex min-w-0 items-center gap-2 text-ink-2">
				<span class="truncate max-sm:hidden">{instanceStore.displayName}</span>
				{#if current}
					<span aria-hidden="true" class="max-sm:hidden">/</span>
					{#if leaf}
						<a href={current.href} class="truncate hover:text-ink">{current.label()}</a>
						<span aria-hidden="true">/</span>
						<span class="truncate font-medium text-ink" aria-current="page">{leaf}</span>
					{:else}
						<span class="truncate font-medium text-ink" aria-current="page">{current.label()}</span>
					{/if}
				{/if}
			</nav>
			<button
				type="button"
				onclick={() => consoleState.tapLive()}
				aria-pressed={consoleState.live}
				aria-label={consoleState.live ? m.console_live_on() : m.console_live_off()}
				title={consoleState.live ? m.console_live_on() : m.console_live_off()}
				class={`flex h-10 shrink-0 items-center gap-1.5 rounded-md px-2 font-mono text-[0.6875rem] font-medium sm:h-6 ${consoleState.live ? 'bg-signal-soft text-signal-ink' : 'bg-surface-2 text-ink-2'}`}
			>
				<span aria-hidden="true" class={`size-1.5 rounded-full ${consoleState.live ? 'ob-live bg-signal' : 'bg-ink-3'}`}></span>
				{consoleState.live ? m.console_live() : m.console_paused()}
			</button>

			<div class="ml-auto flex min-w-0 items-center gap-2">
				<!-- MSP mode: which client every page shows (hidden when there is nothing to pick). -->
				<div class="empty:hidden max-sm:hidden"><ClientSwitcher id="client-switcher-console" /></div>
				{#if onOverview}
					<div role="group" aria-label={m.console_range()} class="flex h-8 overflow-hidden rounded-lg border border-line max-md:hidden">
						{#each CONSOLE_RANGES as range (range)}
							<button
								type="button"
								aria-pressed={consoleState.range === range}
								onclick={() => consoleState.setRange(range)}
								class={`min-w-10 px-2.5 font-mono text-xs font-medium ${consoleState.range === range ? 'bg-line text-ink' : 'text-ink-2 hover:text-ink'}`}
							>{range}</button>
						{/each}
					</div>
				{/if}
				<button
					type="button"
					onclick={() => palette.open()}
					class="flex h-10 w-10 items-center justify-center gap-2 rounded-lg border border-line bg-surface px-2.5 text-ink-2 hover:text-ink sm:h-8 lg:w-72 lg:justify-start"
					aria-label={m.misc_nav_palette_open()}
				>
					<Search class="size-3.5 shrink-0" aria-hidden="true" />
					<span class="flex-1 truncate text-left text-[0.8125rem] max-lg:hidden">{m.console_search_placeholder()}</span>
					<kbd class="rounded border border-line-strong px-1.5 py-px font-mono text-[0.6875rem] text-ink-2 max-lg:hidden">{palette.shortcutLabel}</kbd>
				</button>
				{#if auth.isAdmin}
					<a href="/targets/new" class="flex h-10 items-center gap-1.5 rounded-lg bg-primary px-3 text-[0.8125rem] font-semibold text-on-primary hover:brightness-110 sm:h-8" aria-label={m.console_new_monitor()}>
						<Plus class="size-4 sm:hidden" aria-hidden="true" />
						<span class="max-sm:hidden">{m.console_new_monitor()}</span>
					</a>
				{/if}
				<a
					href="/wall"
					class="flex size-10 items-center justify-center rounded-lg text-ink-2 hover:bg-surface-2 hover:text-ink sm:size-8"
					aria-label={m.misc_nav_wall()}
					title={m.misc_nav_wall()}
				>
					<Monitor class="size-4" aria-hidden="true" />
				</a>
				<button
					type="button"
					onclick={togglePanel}
					class="flex size-10 items-center justify-center rounded-lg text-ink-2 hover:bg-surface-2 hover:text-ink sm:size-8"
					aria-label={m.console_panel_toggle()}
					title={`${m.console_panel_toggle()} (i)`}
				>
					<PanelRight class="size-4" aria-hidden="true" />
				</button>
			</div>
		</header>

		<div class="flex min-h-0 flex-1">
			<main class="flex min-w-0 flex-1 flex-col px-4 pt-4 pb-[calc(var(--tabbar-h)+1rem+var(--safe-bottom))] sm:px-5 sm:pb-6">
				{@render children()}
			</main>
			{#if consoleState.panelOpen}
				<aside aria-label={m.console_panel()} class="sticky top-[52px] hidden h-[calc(100dvh-52px)] w-[360px] shrink-0 border-l border-line bg-canvas-deep xl:block">
					<ContextPanel />
				</aside>
			{/if}
		</div>
	</div>
</div>

{#if consoleState.drawerOpen}
	<div class="fixed inset-0 z-40 bg-canvas/70 xl:hidden" role="presentation" onclick={() => (consoleState.drawerOpen = false)}></div>
	<div
		bind:this={drawer}
		role="dialog"
		aria-modal="true"
		aria-label={m.console_panel()}
		class="fixed inset-y-0 right-0 z-50 flex w-[min(360px,100vw)] flex-col border-l border-line bg-canvas-deep pt-[var(--safe-top)] pb-[var(--safe-bottom)] shadow-float xl:hidden"
	>
		<div class="flex items-center justify-end border-b border-line px-2 py-1">
			<button type="button" class="flex size-10 items-center justify-center rounded-lg text-ink-2 hover:bg-surface-2 hover:text-ink" aria-label={m.console_panel_close()} onclick={() => (consoleState.drawerOpen = false)}>
				<X class="size-4" aria-hidden="true" />
			</button>
		</div>
		<div class="min-h-0 flex-1"><ContextPanel /></div>
	</div>
{/if}

<!-- Phones: the essentials as tabs, the rest under "More". -->
<nav aria-label={m.misc_nav_main()} class="fixed inset-x-0 bottom-0 z-30 border-t border-line bg-canvas pr-[var(--safe-right)] pb-[var(--safe-bottom)] pl-[var(--safe-left)] sm:hidden">
	<ul class="grid" style:grid-template-columns={`repeat(${phoneTabs.length + 1}, minmax(0, 1fr))`}>
		{#each phoneTabs as entry (entry.href)}
			{@const active = isActive(entry, path)}
			<li class="min-w-0">
				<a href={entry.href} aria-current={active ? 'page' : undefined} class={`relative flex h-14 flex-col items-center justify-center gap-0.5 text-[0.6875rem] font-medium ${active ? 'text-ink' : 'text-ink-3'}`}>
					<entry.icon class="size-5" strokeWidth={1.7} aria-hidden="true" />
					<span class="max-w-full truncate px-0.5">{entry.label()}</span>
					{#if entry.badge && badge > 0}
						<span class="absolute top-1.5 right-[calc(50%-1.4rem)] flex h-4 min-w-4 items-center justify-center rounded-full bg-warning px-1 font-mono text-[0.625rem] font-semibold text-canvas">{badge}</span>
					{/if}
				</a>
			</li>
		{/each}
		<li class="min-w-0">
			<Menu label={m.misc_nav_more()} placement="up" align="right" class="h-full w-full">
				{#snippet trigger({ toggle, open })}
					<button type="button" onclick={toggle} aria-expanded={open} class="flex h-14 w-full flex-col items-center justify-center gap-0.5 text-[0.6875rem] font-medium text-ink-3">
						<Ellipsis class="size-5" aria-hidden="true" />
						<span>{m.misc_nav_more()}</span>
					</button>
				{/snippet}
				{#snippet children({ close })}
					<div class="grid min-w-52 p-1">
						<div class="empty:hidden px-1 pb-1"><ClientSwitcher wide id="client-switcher-console-phone" /></div>
						{#each phoneMore as entry (entry.href)}
							<a role="menuitem" href={entry.href} onclick={close} class="flex min-h-11 items-center gap-2.5 rounded-md px-2.5 text-sm text-ink hover:bg-surface-2">
								<entry.icon class="size-4 text-ink-2" aria-hidden="true" />{entry.label()}
							</a>
						{/each}
						<a role="menuitem" href={DOCS_URL} target="_blank" rel="noopener" onclick={close} class="flex min-h-11 items-center gap-2.5 rounded-md px-2.5 text-sm text-ink hover:bg-surface-2" title={m.misc_nav_docs_title()}>
							<BookOpen class="size-4 text-ink-2" aria-hidden="true" />{m.misc_nav_docs()}
						</a>
						<a role="menuitem" href={GITHUB_URL} target="_blank" rel="noopener" onclick={close} class="flex min-h-11 items-center gap-2.5 rounded-md px-2.5 text-sm text-ink hover:bg-surface-2" title={m.misc_nav_github_title()}>
							<GithubMark class="size-4 text-ink-2" />GitHub
						</a>
						{#if auth.canSignOut}
							<button type="button" role="menuitem" onclick={() => { close(); void auth.logout(); }} class="flex min-h-11 items-center gap-2.5 rounded-md px-2.5 text-left text-sm text-ink hover:bg-surface-2">{m.misc_nav_sign_out()}</button>
						{/if}
					</div>
				{/snippet}
			</Menu>
		</li>
	</ul>
</nav>
