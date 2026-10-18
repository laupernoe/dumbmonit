<script lang="ts">
	/**
	 * Settings, in seven tabs, each its own URL: My account (language, theme,
	 * password, two-factor, push), Team & access (users, single sign-on),
	 * Monitoring (agents, packs), Notifications (channels, policy, reports),
	 * Status pages (public pages and their announcements), Wall display and
	 * System (backup, DumbMonit itself, API & assistants, about). A rail on
	 * wide screens, a strip of chips on narrow ones, and a search box over the
	 * section names.
	 *
	 * Sections keep their old anchor ids: `/settings#agents` is sent to
	 * `/settings/monitoring#agents`. The old `/alerts#notifications` and
	 * `/status` links forward here too.
	 */
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import type { Component } from 'svelte';
	import { BellRing, Globe, HardDrive, Search, Server, Tv, UserRound, Users } from 'lucide-svelte';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { PageHeader } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import {
		DEFAULT_TAB,
		MOVED,
		SECTION_TAB,
		TAB_IDS,
		isTab,
		searchIndex,
		searchSettings,
		tabLabel,
		type SettingsTab
	} from '#lib/components/settings/tabs.js';
	import SecuritySection from '#lib/components/settings/SecuritySection.svelte';
	import SecurityScoreSection from '#lib/components/settings/SecurityScoreSection.svelte';
	import UsersSection from '#lib/components/settings/UsersSection.svelte';
	import SsoSection from '#lib/components/settings/SsoSection.svelte';
	import AgentsSection from '#lib/components/settings/AgentsSection.svelte';
	import AssistantSection from '#lib/components/settings/AssistantSection.svelte';
	import AppearanceSection from '#lib/components/settings/AppearanceSection.svelte';
	import BackupSection from '#lib/components/settings/BackupSection.svelte';
	import SelfMonitoringSection from '#lib/components/settings/SelfMonitoringSection.svelte';
	import PacksSection from '#lib/components/settings/PacksSection.svelte';
	import AboutSection from '#lib/components/settings/AboutSection.svelte';
	import MusicSection from '#lib/components/settings/MusicSection.svelte';
	import WallNewsSection from '#lib/components/settings/WallNewsSection.svelte';
	import WallWidgetsSection from '#lib/components/settings/WallWidgetsSection.svelte';
	import WallModeSection from '#lib/components/settings/WallModeSection.svelte';
	import PushSection from '#lib/components/settings/PushSection.svelte';
	import ReportsSection from '#lib/components/settings/ReportsSection.svelte';
	import ChannelsSection from '#lib/components/notifications/ChannelsSection.svelte';
	import NotificationPolicySection from '#lib/components/notifications/NotificationPolicySection.svelte';
	import StatusPagesSection from '#lib/components/settings/StatusPagesSection.svelte';

	// Accounts only exist once the instance is protected; viewers never see that tab.
	const showAccounts = $derived(auth.available && auth.configured && auth.isAdmin);

	const ICONS: Record<SettingsTab, typeof UserRound> = {
		account: UserRound,
		team: Users,
		monitoring: Server,
		notifications: BellRing,
		status: Globe,
		wall: Tv,
		system: HardDrive
	};

	const tabs = $derived(TAB_IDS.filter((id) => id !== 'team' || showAccounts));
	const asked = $derived(page.params.tab);
	const tab = $derived<SettingsTab>(isTab(asked) && tabs.includes(asked) ? asked : DEFAULT_TAB);

	/** The sections of each tab, top to bottom. */
	const SECTIONS = $derived<Record<SettingsTab, Component[]>>({
		account: [AppearanceSection, SecuritySection, PushSection],
		team: [UsersSection, SsoSection],
		monitoring: [AgentsSection, PacksSection, SecurityScoreSection],
		notifications: [ChannelsSection, NotificationPolicySection, ...(auth.isAdmin ? [ReportsSection] : [])],
		status: [StatusPagesSection],
		wall: [WallModeSection, MusicSection, ...(auth.isAdmin ? [WallNewsSection, WallWidgetsSection] : [])],
		system: [BackupSection, SelfMonitoringSection, AssistantSection, AboutSection]
	});

	// An unknown tab (or one this account cannot see) lands on the first one.
	$effect(() => {
		if (asked !== undefined && asked !== tab) void goto(`/settings/${tab}${page.url.search}${page.url.hash}`, { replace: true });
	});

	// A deep link (`/settings/system#about`) lands on its section after the
	// sections rendered; an anchor from another tab, or from before tabs
	// existed, goes to the tab that holds it; one that moved out of Settings
	// goes where it lives now. Sections above the target grow as their data
	// replaces the skeletons, so the landing is repeated on each growth for a
	// few seconds — unless the reader has started scrolling.
	let column = $state<HTMLDivElement | null>(null);
	$effect(() => {
		const hash = page.url.hash.slice(1);
		if (!hash || !column) return;
		const moved = MOVED[hash];
		if (moved) {
			void goto(moved, { replace: true });
			return;
		}
		const home = SECTION_TAB[hash];
		if (home && home !== tab && tabs.includes(home)) {
			void goto(`/settings/${home}${page.url.search}#${hash}`, { replace: true });
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

	// --- Search over section names and a few words each ------------------------
	let query = $state('');
	const results = $derived(
		searchSettings(searchIndex(), query).filter(
			(entry) => tabs.includes(entry.tab) && (auth.isAdmin || (entry.id !== 'reports' && entry.id !== 'news' && entry.id !== 'widgets'))
		)
	);
	const href = (entry: { tab: SettingsTab; id: string }) => `/settings/${entry.tab}#${entry.id}`;

	function onsearchkey(event: KeyboardEvent) {
		if (event.key === 'Enter' && results[0]) {
			event.preventDefault();
			void goto(href(results[0]));
			query = '';
		} else if (event.key === 'Escape') {
			query = '';
		}
	}
</script>

<svelte:head><title>{tabLabel(tab)} · {m['settings.page.title']()} · DumbMonit</title></svelte:head>

{#snippet searchBox(id: string)}
	<div class="relative">
		<label for={id} class="sr-only">{m.settings_search_label()}</label>
		<Search class="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-ink-3" aria-hidden="true" />
		<input
			{id}
			type="search"
			class="input h-10 w-full !pl-9"
			placeholder={m.settings_search_placeholder()}
			autocomplete="off"
			bind:value={query}
			onkeydown={onsearchkey}
		/>
	</div>
	{#if query.trim()}
		<div class="mt-2" aria-live="polite">
			{#if results.length === 0}
				<p class="px-1 text-[0.8125rem] text-ink-2">{m.settings_search_none()}</p>
			{:else}
				<ul class="space-y-0.5">
					{#each results as entry (entry.id)}
						<li>
							<a
								href={href(entry)}
								class="flex min-h-10 flex-col justify-center rounded-md px-2.5 py-1.5 text-sm hover:bg-surface-2"
								onclick={() => (query = '')}
							>
								<span class="font-semibold text-ink">{entry.label}</span>
								<span class="text-[0.75rem] text-ink-2">{tabLabel(entry.tab)}</span>
							</a>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}
{/snippet}

<PageHeader title={m['settings.page.title']()} />

<!-- Tablets: the search, then wrapping chips, one per tab. Phones: a native select, never a scroller. -->
<div class="mb-5 max-sm:hidden lg:hidden">
	{@render searchBox('settings-search-mobile')}
</div>
<nav class="sticky top-0 z-20 -mx-4 mb-5 border-b border-line bg-canvas/90 px-4 backdrop-blur-md max-sm:mb-3 max-sm:bg-canvas max-sm:backdrop-blur-none sm:top-14 sm:-mx-6 sm:px-6 lg:hidden" aria-label={m['settings.nav.aria_label']()}>
	<div class="py-2 sm:hidden">
		<select
			aria-label={m['settings.nav.aria_label']()}
			value={tab}
			onchange={(e) => goto(`/settings/${e.currentTarget.value}`)}
			class="h-11 w-full min-w-0 rounded-lg border border-line-strong bg-surface px-3 text-sm font-semibold text-ink"
		>
			{#each tabs as id (id)}
				<option value={id}>{tabLabel(id)}</option>
			{/each}
		</select>
	</div>
	<ul class="flex flex-wrap gap-1.5 py-2 max-sm:hidden">
		{#each tabs as id (id)}
			<li class="min-w-0">
				<a
					href="/settings/{id}"
					data-tab={id}
					class={`inline-flex h-10 items-center rounded-full border px-3.5 text-[0.8125rem] font-semibold transition-colors ${tab === id ? 'border-line-strong bg-surface-2 text-ink' : 'border-line text-ink-2 hover:text-ink'}`}
					aria-current={tab === id ? 'page' : undefined}
				>
					{tabLabel(id)}
				</a>
			</li>
		{/each}
	</ul>
</nav>

<div class="lg:grid lg:grid-cols-[14rem_minmax(0,1fr)] lg:gap-10">
	<aside class="hidden lg:block">
		<div class="sticky top-20">
			{@render searchBox('settings-search')}
			<nav class="mt-4" aria-label={m['settings.nav.aria_label']()}>
				<ul class="space-y-0.5 text-sm">
					{#each tabs as id (id)}
						{@const Icon = ICONS[id]}
						<li>
							<a
								href="/settings/{id}"
								class={`flex h-10 items-center gap-2.5 rounded-md px-2.5 transition-colors hover:bg-surface-2 hover:text-ink ${tab === id ? 'bg-surface-2 font-semibold text-ink' : 'text-ink-2'}`}
								aria-current={tab === id ? 'page' : undefined}
							>
								<Icon class="size-4 shrink-0" aria-hidden="true" />
								{tabLabel(id)}
							</a>
						</li>
					{/each}
				</ul>
			</nav>
		</div>
	</aside>

	<div bind:this={column} class="grid min-w-0 content-start gap-6 max-sm:gap-3 [&_section[id]]:scroll-mt-20 sm:[&_section[id]]:scroll-mt-32 lg:[&_section[id]]:scroll-mt-20">
		<h2 class="sr-only">{tabLabel(tab)}</h2>
		{#key tab}
			{#each SECTIONS[tab] as Section, i (i)}
				<div class="min-w-0 rise-in" style={`--rise-delay: ${i * 40}ms`}><Section /></div>
			{/each}
		{/key}
	</div>
</div>
