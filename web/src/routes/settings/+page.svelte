<script lang="ts">
	/**
	 * Settings: what is administrative — your account, who can sign in and
	 * how, the tokens that let agents and assistants in, the integration packs
	 * installed, the theme, and the
	 * server's health. Each section loads its own data; this page only lays
	 * them out and offers a rail of anchors on wide screens, a strip of chips
	 * on narrow ones.
	 *
	 * Notification channels and the policy moved to Alerts → Notifications,
	 * status pages to their own Status page (September 2026): their old
	 * `/settings#…` links are redirected below so a bookmark still lands.
	 */
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { PageHeader } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import SecuritySection from '#lib/components/settings/SecuritySection.svelte';
	import UsersSection from '#lib/components/settings/UsersSection.svelte';
	import SsoSection from '#lib/components/settings/SsoSection.svelte';
	import AgentsSection from '#lib/components/settings/AgentsSection.svelte';
	import AssistantSection from '#lib/components/settings/AssistantSection.svelte';
	import AppearanceSection from '#lib/components/settings/AppearanceSection.svelte';
	import BackupSection from '#lib/components/settings/BackupSection.svelte';
	import PacksSection from '#lib/components/settings/PacksSection.svelte';
	import AboutSection from '#lib/components/settings/AboutSection.svelte';
	import MusicSection from '#lib/components/settings/MusicSection.svelte';
	import PushSection from '#lib/components/settings/PushSection.svelte';
	import ReportsSection from '#lib/components/settings/ReportsSection.svelte';

	/** Sections that used to live here, and where they went. */
	const MOVED: Record<string, string> = {
		notifications: '/alerts#notifications',
		'notifications-channels': '/alerts#notifications',
		'notifications-policy': '/alerts#notifications-policy',
		'quiet-hours': '/alerts#notifications',
		status: '/status',
		'status-pages': '/status',
		incidents: '/status#incidents'
	};

	// Accounts only exist once the instance is protected; viewers never see these two.
	const showAccounts = $derived(auth.available && auth.configured && auth.isAdmin);

	// Grouped so the rail reads as what it is: account/security/instance
	// administration, set apart from the room-display options nobody needs
	// admin judgement to touch. A `null` group label renders no heading — the
	// first group stays unlabelled, as it did before grouping existed.
	const GROUPS = $derived([
		{
			label: null,
			items: [
				{ id: 'security', label: m["settings.section.security"]() },
				{ id: 'push', label: 'Push notifications' },
				...(showAccounts
					? [
							{ id: 'users', label: m["settings.section.users"]() },
							{ id: 'sso', label: m["settings.section.sso"]() }
						]
					: []),
				{ id: 'agents', label: m["settings.section.agents"]() },
				{ id: 'assistant', label: m["settings.section.assistant"]() },
				{ id: 'packs', label: m["settings.section.packs"]() },
				{ id: 'backup', label: m["settings.section.backup"]() },
				...(auth.isAdmin ? [{ id: 'reports', label: m["settings.section.reports"]() }] : [])
			]
		},
		{
			label: m["settings.group.wall_display"](),
			items: [
				{ id: 'appearance', label: m["settings.section.appearance"]() },
				{ id: 'music', label: m["settings.section.music"]() }
			]
		},
		{ label: null, items: [{ id: 'about', label: m["settings.section.about"]() }] }
	]);
	const SECTIONS = $derived(GROUPS.flatMap((group) => group.items));

	// The rail follows the scroll: the topmost section in view is the current one.
	let visible = $state<string>('security');
	$effect(() => {
		const observer = new IntersectionObserver(
			(entries) => {
				const hits = entries.filter((e) => e.isIntersecting).sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top);
				if (hits[0]) visible = hits[0].target.id;
			},
			{ rootMargin: '-25% 0px -60% 0px' }
		);
		for (const s of SECTIONS) {
			const el = document.getElementById(s.id);
			if (el) observer.observe(el);
		}
		return () => observer.disconnect();
	});

	// A deep link (`/settings#agents`) lands on its section after the sections
	// rendered; a link to a section that moved goes where it lives now. The
	// sections above the target grow as their data replaces the skeletons, so
	// the landing is repeated on each growth for a few seconds — unless the
	// reader has started scrolling.
	let column = $state<HTMLDivElement | null>(null);
	$effect(() => {
		const hash = page.url.hash.slice(1);
		if (!hash || !column) return;
		const moved = MOVED[hash];
		if (moved) {
			void goto(moved, { replace: true });
			return;
		}
		const land = () => document.getElementById(hash)?.scrollIntoView();
		land();
		const observer = new ResizeObserver(land);
		observer.observe(column);
		const stop = () => observer.disconnect();
		const timer = setTimeout(stop, 4000);
		const opts = { passive: true, once: true } as const;
		window.addEventListener('wheel', stop, opts);
		window.addEventListener('touchmove', stop, opts);
		return () => {
			clearTimeout(timer);
			stop();
			window.removeEventListener('wheel', stop);
			window.removeEventListener('touchmove', stop);
		};
	});

	// Phones: keep the current chip in view inside the strip.
	let strip = $state<HTMLUListElement | null>(null);
	$effect(() => {
		if (!strip) return;
		const chip = strip.querySelector<HTMLElement>(`[href="#${visible}"]`);
		if (!chip) return;
		const left = chip.offsetLeft - 16;
		const right = chip.offsetLeft + chip.offsetWidth + 16;
		if (left < strip.scrollLeft) strip.scrollTo({ left, behavior: 'smooth' });
		else if (right > strip.scrollLeft + strip.clientWidth) strip.scrollTo({ left: right - strip.clientWidth, behavior: 'smooth' });
	});
</script>

<svelte:head><title>{m["settings.page.title"]()} · DumbMonit</title></svelte:head>

<PageHeader title={m["settings.page.title"]()} description={m["settings.page.description"]()} />

<!-- Phones and tablets: a strip of chips under the title, one per section. -->
<nav class="sticky top-0 z-20 -mx-4 mb-5 border-b border-line bg-canvas/90 px-4 backdrop-blur-md sm:top-14 sm:-mx-6 sm:px-6 lg:hidden" aria-label={m["settings.nav.aria_label"]()}>
	<ul bind:this={strip} class="flex gap-1.5 overflow-x-auto py-2 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
		{#each SECTIONS as section (section.id)}
			<li class="shrink-0">
				<a
					href="#{section.id}"
					class={`inline-flex h-8 items-center rounded-full border px-3 text-[0.8125rem] font-semibold whitespace-nowrap transition-colors ${visible === section.id ? 'border-line-strong bg-surface-2 text-ink' : 'border-line text-ink-2 hover:text-ink'}`}
					aria-current={visible === section.id ? 'location' : undefined}
				>
					{section.label}
				</a>
			</li>
		{/each}
	</ul>
</nav>

<div class="lg:grid lg:grid-cols-[12rem_minmax(0,1fr)] lg:gap-10">
	<aside class="hidden lg:block">
		<nav class="sticky top-20" aria-label={m["settings.nav.aria_label"]()}>
			<ul class="space-y-0.5 text-sm">
				{#each GROUPS as group, i (group.label ?? i)}
					{#if group.label}
						<li class={`px-2.5 ${i === 0 ? 'pb-1.5' : 'pt-3 pb-1.5'} text-[0.75rem] font-semibold tracking-wide text-ink-3 uppercase`}>
							{group.label}
						</li>
					{/if}
					{#each group.items as section (section.id)}
						<li>
							<a
								href="#{section.id}"
								class={`block rounded-md px-2.5 py-1.5 transition-colors hover:bg-surface-2 hover:text-ink ${visible === section.id ? 'bg-surface-2 font-semibold text-ink' : 'text-ink-2'}`}
								aria-current={visible === section.id ? 'location' : undefined}
							>
								{section.label}
							</a>
						</li>
					{/each}
				{/each}
			</ul>
			<p class="mt-5 border-t border-line pt-4 text-[0.8125rem] leading-relaxed text-ink-2">
				{m["settings.hint.notifications_lead"]()}
				<a href="/alerts#notifications" class="font-semibold text-ink hover:underline">{m["settings.hint.notifications_link"]()}</a>.
				{m["settings.hint.status_lead"]()}
				<a href="/status" class="font-semibold text-ink hover:underline">{m["settings.hint.status_link"]()}</a>.
			</p>
		</nav>
	</aside>

	<div bind:this={column} class="grid min-w-0 gap-6 [&_section[id]]:scroll-mt-14 sm:[&_section[id]]:scroll-mt-28 lg:[&_section[id]]:scroll-mt-20">
		<div class="min-w-0 rise-in" style="--rise-delay: 0ms"><SecuritySection /></div>
		<div class="min-w-0 rise-in" style="--rise-delay: 20ms"><PushSection /></div>
		{#if showAccounts}
			<div class="min-w-0 rise-in" style="--rise-delay: 40ms"><UsersSection /></div>
			<div class="min-w-0 rise-in" style="--rise-delay: 80ms"><SsoSection /></div>
		{/if}
		<div class="min-w-0 rise-in" style="--rise-delay: 120ms"><AgentsSection /></div>
		<div class="min-w-0 rise-in" style="--rise-delay: 160ms"><AssistantSection /></div>
		<div class="min-w-0 rise-in" style="--rise-delay: 200ms"><PacksSection /></div>
		<div class="min-w-0 rise-in" style="--rise-delay: 240ms"><BackupSection /></div>
		{#if auth.isAdmin}
			<div class="min-w-0 rise-in" style="--rise-delay: 260ms"><ReportsSection /></div>
		{/if}
		<div class="min-w-0 rise-in" style="--rise-delay: 280ms"><AppearanceSection /></div>
		<div class="min-w-0 rise-in" style="--rise-delay: 300ms"><MusicSection /></div>
		<div class="min-w-0 rise-in" style="--rise-delay: 320ms"><AboutSection /></div>
	</div>
</div>
