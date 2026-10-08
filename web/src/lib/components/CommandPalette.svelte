<script lang="ts">
	/**
	 * Command palette (Ctrl/⌘ K): jump to a page, run an action, open a device.
	 *
	 * One input, results in three groups (pages, actions, devices). Devices are
	 * fetched when the palette opens and cached for 30 s so a second opening is
	 * instant. Matching is substring-per-word: every word of the query must
	 * appear somewhere in the entry's name, address, kind or keywords; entries
	 * whose name starts with the query rank first.
	 */
	import { goto } from '$app/navigation';
	import { tick } from 'svelte';
	import type { Icon as LucideIcon } from 'lucide-svelte';
	import {
		Search,
		Gauge,
		Server,
		BellRing,
		Globe,
		Settings2,
		Tv,
		BookOpen,
		ShieldCheck,
		Plus,
		Radar,
		CalendarClock,
		BellPlus,
		Megaphone,
		SunMoon,
		CornerDownLeft,
		Cpu,
		RadioTower,
		Bot,
		KeyRound
	} from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { listTargets, type Target, type TargetId } from '#lib/api/index.js';
	import { displayState, STATE_LABEL, STATE_TONE, type ProbeStatus } from '#lib/format.js';
	import { loadProbeStatuses } from '#lib/metrics.js';
	import { palette } from '#lib/stores/palette.svelte.js';
	import { theme } from '#lib/stores/theme.svelte.js';
	import { Led } from '#lib/ui/index.js';
	import { kindIcon } from '#lib/components/device-form/kinds.js';

	type Group = 'pages' | 'actions' | 'devices';
	const GROUP_LABEL: Record<Group, () => string> = {
		pages: () => m.misc_palette_group_pages(),
		actions: () => m.misc_palette_group_actions(),
		devices: () => m.misc_palette_group_devices()
	};

	interface Entry {
		id: string;
		group: Group;
		label: string;
		/** Secondary line: a device's address, an action's destination. */
		detail?: string;
		/** Extra words that match but are not shown. */
		keywords?: string;
		icon?: typeof LucideIcon;
		/** Device entries carry a state LED instead of an icon. */
		led?: { tone: 'signal' | 'advisory' | 'warning' | 'ghost'; label: string };
		run: () => void;
	}

	const MAX_PER_GROUP = 8;
	const CACHE_MS = 30_000;

	const go = (href: string) => () => void goto(href);

	// A function: labels follow the active language when the palette opens.
	// Search keywords stay in English on purpose (they are matched, not shown).
	const staticEntries = (): Entry[] => [
		{ id: 'page:/', group: 'pages', label: m.misc_nav_overview(), keywords: 'home bulletin sky', icon: Gauge, run: go('/') },
		{ id: 'page:/targets', group: 'pages', label: m.misc_nav_devices(), keywords: 'rack targets hosts', icon: Server, run: go('/targets') },
		{ id: 'page:/alerts', group: 'pages', label: m.misc_nav_alerts(), keywords: 'needs you warnings advisories', icon: BellRing, run: go('/alerts') },
		{ id: 'page:/alerts#notifications', group: 'pages', label: m.misc_palette_notifications(), detail: m.misc_palette_notifications_detail(), keywords: 'channels policy quiet hours alerts', icon: BellRing, run: go('/alerts#notifications') },
		{ id: 'page:/security', group: 'pages', label: m.misc_nav_security(), detail: m.misc_palette_security_detail(), keywords: 'security score grade pingcastle secure score checks', icon: ShieldCheck, run: go('/security') },
		{ id: 'page:/status', group: 'pages', label: m.misc_palette_status_pages(), detail: m.misc_palette_status_detail(), keywords: 'public incidents maintenance announcements uptime', icon: Globe, run: go('/status') },
		{ id: 'page:/settings', group: 'pages', label: m.misc_nav_settings(), keywords: 'preferences account password users sso agents tokens assistant appearance theme about', icon: Settings2, run: go('/settings') },
		{ id: 'page:/wall', group: 'pages', label: m.misc_nav_wall(), detail: m.misc_palette_wall_detail(), keywords: 'kiosk tv screen', icon: Tv, run: go('/wall') },
		{ id: 'page:docs', group: 'pages', label: m.misc_palette_documentation(), keywords: 'docs help manual notifications channels', icon: BookOpen, run: () => window.open('https://dumbmonit.readthedocs.io/en/latest/', '_blank', 'noopener') },
		{ id: 'action:add', group: 'actions', label: m.misc_palette_add_device(), keywords: 'new target create host', icon: Plus, run: go('/targets/new') },
		{ id: 'action:scan', group: 'actions', label: m.misc_palette_scan(), keywords: 'discover cidr snmp', icon: Radar, run: go('/targets/new?scan=1') },
		{ id: 'action:agent', group: 'actions', label: m.misc_palette_agent(), detail: m.misc_palette_agent_detail(), keywords: 'agent install machine server linux windows macos freebsd docker containers services disks backups token enroll', icon: Cpu, run: go('/targets/new?kind=agent') },
		{ id: 'action:relay', group: 'actions', label: m.misc_palette_relay(), detail: m.misc_palette_relay_detail(), keywords: 'relay remote site branch office second site client customer nat firewall vpn vps network agent', icon: RadioTower, run: go('/targets/new?kind=agent&via=relay') },
		{ id: 'action:mcp', group: 'actions', label: m.misc_palette_mcp(), detail: m.misc_palette_mcp_detail(), keywords: 'mcp ai assistant claude chatgpt cursor llm agent model context protocol', icon: Bot, run: go('/settings#assistant') },
		{ id: 'action:api-token', group: 'actions', label: m.misc_palette_api_token(), detail: m.misc_palette_api_token_detail(), keywords: 'api token rest http key script automation bearer prometheus grafana federate scrape', icon: KeyRound, run: go('/settings#assistant') },
		{ id: 'action:maintenance', group: 'actions', label: m.misc_palette_maintenance(), keywords: 'silence window quiet', icon: CalendarClock, run: go('/alerts#scheduled') },
		{ id: 'action:channel', group: 'actions', label: m.misc_palette_channel(), keywords: 'slack discord telegram email webhook', icon: BellPlus, run: go('/alerts#notifications') },
		{ id: 'action:status-page', group: 'actions', label: m.misc_palette_status_page(), keywords: 'public status page create', icon: Globe, run: go('/status/new') },
		{ id: 'action:incident', group: 'actions', label: m.misc_palette_incident(), keywords: 'status page maintenance announcement outage', icon: Megaphone, run: go('/status#incidents') },
		{ id: 'action:theme', group: 'actions', label: m.misc_palette_theme(), keywords: 'dark light night day', icon: SunMoon, run: () => theme.toggle() }
	];

	let query = $state('');
	let active = $state(0);
	let input = $state<HTMLInputElement | null>(null);
	let panel = $state<HTMLDivElement | null>(null);
	let list = $state<HTMLDivElement | null>(null);

	let targets = $state<Target[]>([]);
	let probes = $state<Map<TargetId, ProbeStatus>>(new Map());
	let loadingDevices = $state(false);
	let fetchedAt = 0;

	async function loadDevices() {
		if (Date.now() - fetchedAt < CACHE_MS) return;
		loadingDevices = targets.length === 0;
		try {
			const [nextTargets, nextProbes] = await Promise.all([
				listTargets(),
				loadProbeStatuses().catch(() => new Map<TargetId, ProbeStatus>())
			]);
			targets = nextTargets;
			probes = nextProbes;
			fetchedAt = Date.now();
		} catch {
			// The palette still works for pages and actions.
		} finally {
			loadingDevices = false;
		}
	}

	const deviceEntries = $derived<Entry[]>(
		targets.map((target) => {
			const state = displayState(target, probes.get(target.id));
			return {
				id: `device:${target.id}`,
				group: 'devices',
				label: target.name,
				detail: `${target.kind} · ${target.address}`,
				keywords: STATE_LABEL[state],
				icon: kindIcon(target.kind),
				led: { tone: STATE_TONE[state], label: STATE_LABEL[state] },
				run: go(`/targets/${target.id}`)
			};
		})
	);

	/** 2 = name starts with the query, 1 = every word found, 0 = no match. */
	function score(entry: Entry, words: string[]): number {
		if (words.length === 0) return 1;
		const name = entry.label.toLowerCase();
		const hay = `${name} ${entry.detail ?? ''} ${entry.keywords ?? ''}`.toLowerCase();
		if (!words.every((word) => hay.includes(word))) return 0;
		return name.startsWith(words[0]) ? 2 : 1;
	}

	const results = $derived.by(() => {
		const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
		const groups: { name: Group; entries: Entry[] }[] = [];
		for (const name of ['pages', 'actions', 'devices'] as Group[]) {
			const pool = name === 'devices' ? deviceEntries : staticEntries().filter((entry) => entry.group === name);
			const scored = pool
				.map((entry) => ({ entry, score: score(entry, words) }))
				.filter((item) => item.score > 0)
				.sort((a, b) => b.score - a.score);
			// Pages and actions are a short fixed list, all worth seeing; only devices are capped.
			const entries = (name === 'devices' ? scored.slice(0, MAX_PER_GROUP) : scored).map((item) => item.entry);
			if (entries.length > 0) groups.push({ name, entries });
		}
		return groups;
	});
	const flat = $derived(results.flatMap((group) => group.entries));

	// A new query resets the cursor; the cursor never points past the list.
	$effect(() => {
		query;
		active = 0;
	});
	$effect(() => {
		if (active >= flat.length) active = Math.max(0, flat.length - 1);
	});

	// Opening: reset, focus the input, fetch devices. The layout is locked
	// behind the backdrop so the page does not scroll under it.
	$effect(() => {
		if (!palette.isOpen) return;
		query = '';
		active = 0;
		void loadDevices();
		void tick().then(() => input?.focus());
		const previous = document.body.style.overflow;
		document.body.style.overflow = 'hidden';
		return () => {
			document.body.style.overflow = previous;
		};
	});

	// The active option stays in view while arrowing through a long list.
	$effect(() => {
		if (!palette.isOpen) return;
		const option = list?.querySelector<HTMLElement>(`[data-index="${active}"]`);
		option?.scrollIntoView({ block: 'nearest' });
	});

	function run(entry: Entry) {
		palette.close();
		entry.run();
	}

	// The Ctrl/⌘ K shortcut itself is handled once, in the root layout (which
	// mounts this component dynamically on first use); this listener only
	// handles keys that matter while the panel is already open.
	function onWindowKeydown(event: KeyboardEvent) {
		if (!palette.isOpen) return;
		if (event.key === 'Escape') {
			event.preventDefault();
			palette.close();
			return;
		}
		// Focus trap: Tab stays inside the panel.
		if (event.key === 'Tab' && panel) {
			const focusable = [...panel.querySelectorAll<HTMLElement>('input, button, [tabindex="0"]')].filter(
				(el) => !el.hasAttribute('disabled')
			);
			if (focusable.length === 0) return;
			const first = focusable[0];
			const last = focusable[focusable.length - 1];
			const current = document.activeElement;
			if (event.shiftKey && (current === first || !panel.contains(current))) {
				event.preventDefault();
				last.focus();
			} else if (!event.shiftKey && (current === last || !panel.contains(current))) {
				event.preventDefault();
				first.focus();
			}
		}
	}

	function onInputKeydown(event: KeyboardEvent) {
		switch (event.key) {
			case 'ArrowDown':
				event.preventDefault();
				if (flat.length) active = (active + 1) % flat.length;
				break;
			case 'ArrowUp':
				event.preventDefault();
				if (flat.length) active = (active - 1 + flat.length) % flat.length;
				break;
			case 'Home':
				event.preventDefault();
				active = 0;
				break;
			case 'End':
				event.preventDefault();
				active = Math.max(0, flat.length - 1);
				break;
			case 'Enter': {
				event.preventDefault();
				const entry = flat[active];
				if (entry) run(entry);
				break;
			}
		}
	}

	const activeId = $derived(flat[active] ? `palette-option-${flat[active].id}` : undefined);
	const placeholder = $derived(m.misc_palette_placeholder());
</script>

<svelte:window onkeydown={onWindowKeydown} />

{#if palette.isOpen}
	<!-- Backdrop: a click outside the panel closes. -->
	<div
		class="palette-backdrop fixed inset-0 z-50 flex items-start justify-center bg-canvas/70 px-4 pt-[12vh] backdrop-blur-sm sm:pt-[16vh]"
		onclick={(event) => {
			if (event.target === event.currentTarget) palette.close();
		}}
		role="presentation"
	>
		<div
			bind:this={panel}
			class="palette-panel flex w-full max-w-xl flex-col overflow-hidden rounded-[var(--radius-card)] border border-line bg-surface shadow-float ring-1 ring-signal/40"
			role="dialog"
			aria-modal="true"
			aria-label={m.misc_palette_label()}
		>
			<div class="flex items-center gap-3 border-b border-line px-4">
				<Search class="size-[18px] shrink-0 text-ink-3" aria-hidden="true" />
				<input
					bind:this={input}
					bind:value={query}
					type="text"
					class="h-14 min-w-0 flex-1 bg-transparent text-base text-ink placeholder:text-ink-3"
					{placeholder}
					aria-label={placeholder}
					role="combobox"
					aria-expanded="true"
					aria-controls="palette-results"
					aria-autocomplete="list"
					aria-activedescendant={activeId}
					autocomplete="off"
					autocorrect="off"
					autocapitalize="off"
					spellcheck="false"
					onkeydown={onInputKeydown}
				/>
				<kbd
					class="hidden rounded-md border border-line bg-surface-2 px-1.5 py-0.5 text-[0.6875rem] font-semibold text-ink-3 sm:inline-block"
					>esc</kbd
				>
			</div>

			<div
				bind:this={list}
				id="palette-results"
				role="listbox"
				aria-label={m.misc_palette_results()}
				class="max-h-[min(60vh,26rem)] overflow-y-auto overscroll-contain py-2"
			>
				{#if flat.length === 0}
					<p class="px-4 py-8 text-center text-sm text-ink-2" aria-live="polite">
						{#if loadingDevices}
							{m.misc_palette_loading()}
						{:else}
							{m.misc_palette_empty({ query })}
						{/if}
					</p>
				{:else}
					{#each results as group (group.name)}
						<div class="px-2 pt-2 first:pt-0">
							<div class="label-tape px-2 pb-1.5">{GROUP_LABEL[group.name]()}</div>
							{#each group.entries as entry (entry.id)}
								{@const index = flat.indexOf(entry)}
								{@const selected = index === active}
								<button
									type="button"
									id={`palette-option-${entry.id}`}
									role="option"
									aria-selected={selected}
									data-index={index}
									tabindex="-1"
									class={`flex w-full items-center gap-3 rounded-lg px-2 py-2 text-left transition-colors ${selected ? 'bg-surface-2 text-ink' : 'text-ink-2 hover:bg-surface-2 hover:text-ink'}`}
									onmousemove={() => (active = index)}
									onclick={() => run(entry)}
								>
									<span class="flex size-8 shrink-0 items-center justify-center rounded-lg border border-line bg-surface text-ink-2">
										{#if entry.icon}
											<entry.icon class="size-4" aria-hidden="true" />
										{/if}
									</span>
									<span class="min-w-0 flex-1">
										<span class="flex items-center gap-2">
											<span class="truncate text-sm font-semibold text-ink">{entry.label}</span>
											{#if entry.led}
												<span class="inline-flex items-center gap-1.5 text-[0.75rem] text-ink-2">
													<Led tone={entry.led.tone} size="sm" />
													{entry.led.label}
												</span>
											{/if}
										</span>
										{#if entry.detail}
											<span class="block truncate text-[0.8125rem] text-ink-2">{entry.detail}</span>
										{/if}
									</span>
									{#if selected}
										<CornerDownLeft class="size-4 shrink-0 text-ink-3" aria-hidden="true" />
									{/if}
								</button>
							{/each}
						</div>
					{/each}
				{/if}
			</div>

			<div class="flex items-center gap-4 border-t border-line bg-canvas px-4 py-2 text-[0.75rem] text-ink-2">
				<span><kbd class="palette-key">↑</kbd><kbd class="palette-key">↓</kbd> navigate</span>
				<span><kbd class="palette-key">↵</kbd> open</span>
				<span><kbd class="palette-key">esc</kbd> close</span>
			</div>
		</div>
	</div>
{/if}

<style>
	/* The panel is the control: it carries the focus ring, the input does not.
	   (The global :focus-visible rule is unlayered, so a utility cannot beat it.) */
	.palette-panel input:focus-visible {
		outline: none;
	}
	/* 16px on phones so iOS does not zoom into the field. */
	@media (max-width: 639px) {
		.palette-panel input {
			font-size: 16px;
		}
	}

	.palette-key {
		display: inline-block;
		margin-right: 0.25rem;
		min-width: 1.25rem;
		border: 1px solid var(--c-line);
		border-radius: 0.375rem;
		background: var(--c-surface-2);
		padding: 0 0.3rem;
		text-align: center;
		font-family: inherit;
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--c-ink-3);
	}

	@keyframes palette-backdrop-in {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}
	@keyframes palette-panel-in {
		from {
			opacity: 0;
			transform: scale(0.97) translateY(-6px);
		}
		to {
			opacity: 1;
			transform: none;
		}
	}
	.palette-backdrop {
		animation: palette-backdrop-in 160ms var(--ease-out-expo) both;
	}
	.palette-panel {
		animation: palette-panel-in 160ms var(--ease-out-expo) both;
	}
	@media (prefers-reduced-motion: reduce) {
		.palette-backdrop,
		.palette-panel {
			animation: none;
		}
	}
</style>
