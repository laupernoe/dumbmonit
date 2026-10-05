<script lang="ts">
	import '../app.css';
	import type { Component } from 'svelte';
	import { page } from '$app/state';
	import { goto, onNavigate } from '$app/navigation';
	import { theme } from '$lib/stores/theme.svelte';
	import { alertsStore } from '$lib/stores/alerts.svelte';
	import { palette } from '$lib/stores/palette.svelte';
	import { auth, safeDestination, isPublicRoute, isStandaloneRoute } from '$lib/stores/auth.svelte';
	import NavBar from '$lib/components/NavBar.svelte';
	import Logo from '$lib/components/Logo.svelte';
	import DemoBanner from '$lib/components/demo/DemoBanner.svelte';
	import DemoNotice from '$lib/components/demo/DemoNotice.svelte';
	import Tour from '$lib/components/demo/Tour.svelte';
	import { demo } from '$lib/stores/demo.svelte';
	import { reducedMotion } from '$lib/ui';

	let { children } = $props();

	// The command palette (Ctrl/⌘ K) is not mounted on `/wall`, a full-screen
	// kiosk display that owns its own Escape handling and has no use for it,
	// and is otherwise loaded only on its first open: forty-odd icons and the
	// device catalogue it searches are not worth shipping to every page.
	const onWall = $derived(page.url.pathname.startsWith('/wall'));
	let CommandPalette = $state<Component | null>(null);
	$effect(() => {
		if (onWall || CommandPalette || !palette.isOpen) return;
		void import('$lib/components/CommandPalette.svelte').then((mod) => {
			CommandPalette = mod.default;
		});
	});
	$effect(() => {
		if (onWall) return;
		function onKeydown(event: KeyboardEvent) {
			if (!palette.matches(event)) return;
			event.preventDefault();
			palette.toggle();
		}
		window.addEventListener('keydown', onKeydown);
		return () => window.removeEventListener('keydown', onKeydown);
	});

	// The system theme can flip during a session (evening switch).
	$effect(() => theme.watchSystem());
	$effect(() => {
		theme.apply();
	});

	// Session state is established once at startup; the global 401 handler is
	// installed here too, so no page has to know authentication exists.
	$effect(() => {
		void auth.init();
	});

	const publicPath = $derived(isPublicRoute(page.url.pathname));

	// Three situations: fresh instance, no session, valid session.
	$effect(() => {
		if (!auth.checked) return;
		const path = page.url.pathname;
		// A public status page is the same for everyone: no redirect either way.
		if (isStandaloneRoute(path)) return;

		if (!auth.available) {
			if (isPublicRoute(path)) void goto('/', { replaceState: true });
			return;
		}
		if (!auth.configured) {
			if (path !== '/setup') void goto('/setup', { replaceState: true });
			return;
		}
		if (!auth.authenticated) {
			if (path === '/setup') {
				void goto('/login', { replaceState: true });
			} else if (!isPublicRoute(path)) {
				const target = path + page.url.search;
				void goto(`/login?redirect=${encodeURIComponent(target)}`, { replaceState: true });
			}
			return;
		}
		if (isPublicRoute(path)) {
			void goto(safeDestination(page.url.searchParams.get('redirect')), { replaceState: true });
		}
	});

	// Public demo: refused changes explain themselves, and the tour opens once
	// on the first visit (the banner reopens it).
	$effect(() => {
		if (!auth.demo || !auth.canUseApi) return;
		demo.install();
		if (!demo.tourSeen && !isPublicRoute(page.url.pathname)) demo.openTour();
	});

	// Page changes cross-fade through the View Transitions API: the bars stay,
	// the page underneath leaves in 120 ms and the next one rises in. Only
	// between pages — a step inside a page (`?kind=`, `#section`) swaps at once —
	// and never under reduced motion or where the API is missing.
	onNavigate((navigation) => {
		if (typeof document === 'undefined' || !('startViewTransition' in document)) return;
		if (!navigation.from || !navigation.to) return;
		if (navigation.from.url.pathname === navigation.to.url.pathname) return;
		// Wall mode covers the bars with its own overlay: a captured bar would
		// float above it for the length of the fade.
		if ([navigation.from, navigation.to].some((end) => end.url.pathname.startsWith('/wall'))) return;
		if (reducedMotion()) return;
		return new Promise<void>((resolve) => {
			const transition = document.startViewTransition(async () => {
				resolve();
				// A cancelled navigation just ends the fade; it is not an error here.
				await navigation.complete.catch(() => {});
			});
			// A transition skipped by the next click rejects these: nothing to report.
			transition.ready.catch(() => {});
			transition.finished.catch(() => {});
		});
	});

	// The alert count is shared by the whole app; it only polls once a session exists.
	$effect(() => {
		if (!auth.canUseApi) return;
		return alertsStore.startPolling();
	});
</script>

<svelte:head>
	<title>DumbMonit</title>
</svelte:head>

<div class="flex min-h-full flex-col">
	{#if publicPath}
		{@render children()}
	{:else if !auth.canUseApi}
		<main class="flex flex-1 flex-col items-center justify-center gap-4 px-4 py-16">
			<Logo class="size-10 animate-pulse" />
			<p class="text-sm text-ink-2">
				{auth.checked ? 'Taking you to sign in…' : 'Checking your session…'}
			</p>
		</main>
	{:else}
		{#if auth.demo}
			<DemoBanner />
		{/if}
		<NavBar />
		<main class="mx-auto w-full max-w-7xl flex-1 px-4 pt-6 pb-24 sm:px-6 sm:pb-12">
			{@render children()}
		</main>
		{#if CommandPalette && !onWall}
			<CommandPalette />
		{/if}
		{#if auth.demo}
			<Tour />
		{/if}
	{/if}
	{#if auth.demo}
		<DemoNotice />
	{/if}
</div>
