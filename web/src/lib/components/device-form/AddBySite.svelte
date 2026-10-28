<script lang="ts">
	/**
	 * "Where is it?" — first step of adding a monitor: pick a site (a location
	 * of the world map, or none), then add what fits there in one click.
	 *
	 * Two proposals per site: devices a network scan found there that nobody
	 * monitors yet (created at once with the site's relay, location and
	 * folder, except those that need credentials), and the kinds that fit the
	 * site (they open the form already filled in). The full list of kinds stays
	 * on the page, below.
	 *
	 * Nothing is rendered while the map has no location: the page then works
	 * exactly as before.
	 */
	import { ArrowUpRight, Check, MapPin, Plus, Router } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { createTarget, listTargets } from '#lib/api/index.js';
	import { listLocations } from '#lib/api/locations.js';
	import { listRelays } from '#lib/api/relay.js';
	import { listNetscanInventory, listNetscanSchedules } from '#lib/api/discovery.js';
	import type { CollectorInfo, MapLocation, NetscanDevice, NetscanSchedule, RelayAgent, Target } from '#lib/api/types.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, ErrorNotice, Plate } from '#lib/ui/index.js';
	import KindIcon from '#lib/components/KindIcon.svelte';
	import { kindLabel } from '#lib/kind-label.js';
	import {
		NO_DEFAULTS,
		NO_SITE,
		defaultsFor,
		defaultsParams,
		discoveredFor,
		payloadFor,
		relaysOf,
		suggestedKinds,
		visibleLocations,
		type Discovered,
		type SiteDefaults
	} from './add-by-site.js';

	interface Props {
		collectors: CollectorInfo[];
		/** Chosen site: a location id as text, `none`, or `null` (not asked yet). */
		site?: string | null;
		/** What the chosen site pre-fills; the page passes it on to the form. */
		defaults?: SiteDefaults;
		/** A kind was picked: the page opens its form. Absent: a plain link (new tab in the wizard). */
		onkind?: (kind: string, defaults: SiteDefaults) => void;
		/** A device was created. */
		onadded?: (target: Target) => void;
		/** Link target of the kind buttons. */
		formHref?: string;
		/** Link target opened in a new tab (the wizard keeps its place). */
		newTab?: boolean;
	}

	let {
		collectors,
		site = $bindable(null),
		defaults = $bindable(NO_DEFAULTS),
		onkind,
		onadded,
		formHref = '/targets/new',
		newTab = false
	}: Props = $props();

	let locations = $state<MapLocation[]>([]);
	let relays = $state<RelayAgent[]>([]);
	let targets = $state<Target[]>([]);
	let schedules = $state<NetscanSchedule[]>([]);
	let inventory = $state<NetscanDevice[]>([]);
	let ready = $state(false);

	async function loadAll(signal?: AbortSignal) {
		const [locs, rel, tgt, sch, inv] = await Promise.all([
			listLocations(signal).catch(() => null),
			listRelays(signal).catch(() => [] as RelayAgent[]),
			listTargets(signal).catch(() => [] as Target[]),
			// The scan inventory is for administrators: others simply see no scan results.
			listNetscanSchedules(signal).catch(() => [] as NetscanSchedule[]),
			listNetscanInventory(signal).catch(() => [] as NetscanDevice[])
		]);
		if (signal?.aborted) return;
		locations = locs?.locations ?? [];
		relays = rel;
		targets = tgt;
		schedules = sch;
		inventory = inv;
		ready = true;
	}

	$effect(() => {
		const controller = new AbortController();
		void loadAll(controller.signal);
		return () => controller.abort();
	});

	const shown = $derived(visibleLocations(locations, targets, auth.mspOn ? auth.currentClient : null));
	const current = $derived(site === null || site === NO_SITE ? null : (shown.find((l) => String(l.id) === site) ?? null));
	/** A site that is asked for but unknown (deleted, other client) counts as not asked. */
	const asked = $derived(site === NO_SITE || current !== null);
	const siteRelays = $derived(relaysOf(current, relays, targets));
	const anyRelays = $derived(new Set(shown.flatMap((l) => relaysOf(l, relays, targets).map((r) => r.id))));
	const computed = $derived(asked ? defaultsFor(current, relays, targets) : NO_DEFAULTS);
	$effect(() => {
		defaults = computed;
	});

	let added = $state<{ id: number; name: string }[]>([]);
	const found = $derived(
		asked
			? discoveredFor(current, inventory, schedules, siteRelays, anyRelays, targets, collectors)
			: []
	);
	const kinds = $derived(asked ? suggestedKinds(current, targets, collectors) : []);
	const LIMIT = 4;
	let showAll = $state(false);
	const visible = $derived(showAll ? found : found.slice(0, LIMIT));

	const relayName = $derived(relays.find((r) => r.id === computed.via_agent)?.name ?? null);

	let busyIp = $state<string | null>(null);
	let error = $state<unknown>(null);

	function params(extra: Record<string, string>) {
		return new URLSearchParams({ ...extra, ...defaultsParams(computed) }).toString();
	}

	async function add(item: Discovered) {
		if (busyIp) return;
		busyIp = item.device.ip;
		error = null;
		try {
			const created = await createTarget(payloadFor(item, computed));
			added = [...added, { id: created.id, name: item.label }];
			targets = [...targets, created];
			onadded?.(created);
		} catch (cause) {
			error = cause;
		} finally {
			busyIp = null;
		}
	}

	function setupHref(item: Discovered) {
		const prefill = JSON.stringify({ address: item.address, name: item.label });
		return `${formHref}?${params({ kind: item.kind, prefill })}`;
	}

	const chip =
		'flex min-h-10 cursor-pointer items-center gap-2 rounded-lg border border-line bg-surface px-3 text-sm text-ink transition-colors hover:border-line-strong has-[:checked]:border-ink has-[:checked]:bg-surface-2 has-[:checked]:font-semibold has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-signal';
	const target = $derived(newTab ? '_blank' : undefined);
</script>

{#if ready && shown.length > 0}
	<section aria-labelledby="add-site-title" class="mb-8 min-w-0">
		<h2 id="add-site-title" class="text-lg font-semibold tracking-tight text-ink">{m.deviceform_site_title()}</h2>
		<p class="mt-1 max-w-[65ch] text-sm text-ink-2">{m.deviceform_site_hint()}</p>

		<div role="radiogroup" aria-labelledby="add-site-title" class="mt-3 flex flex-wrap gap-2">
			{#each shown as location (location.id)}
				<label class={chip}>
					<input type="radio" name="add-site" class="sr-only" value={String(location.id)} bind:group={site} />
					<MapPin class="size-4 shrink-0 text-ink-2" aria-hidden="true" />
					<span class="max-w-[14rem] truncate">{location.name}</span>
					<span class="font-mono text-xs text-ink-3" title={m.deviceform_site_count({ count: location.members.length })}>{location.members.length}</span>
				</label>
			{/each}
			<label class={chip}>
				<input type="radio" name="add-site" class="sr-only" value={NO_SITE} bind:group={site} />
				<span>{m.deviceform_site_none()}</span>
			</label>
		</div>

		{#if asked}
			<div class="mt-4 rounded-[var(--radius-card)] border border-line bg-surface p-4 max-sm:p-3" aria-live="polite">
				<p class="flex flex-wrap items-center gap-x-3 gap-y-1 text-[0.8125rem] text-ink-2">
					{#if current}
						<span class="inline-flex items-center gap-1"><MapPin class="size-3.5" aria-hidden="true" />{current.name}</span>
					{/if}
					{#if relayName}
						<span class="inline-flex items-center gap-1"><Router class="size-3.5" aria-hidden="true" />{m.deviceform_site_via({ relay: relayName })}</span>
					{/if}
					{#if computed.folder}<span>{m.deviceform_site_folder({ folder: computed.folder })}</span>{/if}
					{#if !current}<span>{m.deviceform_site_none_note()}</span>{/if}
				</p>

				{#if found.length > 0}
					<h3 class="mt-4 text-sm font-semibold text-ink">{m.deviceform_site_found()}</h3>
					<ul class="mt-2 grid gap-1.5">
						{#each visible as item (item.device.id)}
							<li class="flex min-h-10 flex-wrap items-center gap-x-3 gap-y-1 rounded-lg border border-line px-3 py-1.5">
								<span class="min-w-0 flex-1">
									<span class="block truncate text-sm font-medium text-ink">{item.label}</span>
									<span class="block truncate font-mono text-xs text-ink-3">{item.device.ip}{item.device.vendor ? ` · ${item.device.vendor}` : ''}</span>
								</span>
								<Plate tone="ghost" bare><KindIcon kind={item.kind} class="size-3.5" />{kindLabel(item.kind, collectors.find((c) => c.kind === item.kind)?.label)}</Plate>
								{#if item.direct}
									<Button size="sm" variant="secondary" loading={busyIp === item.device.ip} disabled={busyIp !== null} onclick={() => add(item)} aria-label={m.deviceform_site_add_named({ name: item.label })}>
										<Plus class="size-4" aria-hidden="true" />{m.deviceform_site_add()}
									</Button>
								{:else}
									<Button size="sm" variant="secondary" href={setupHref(item)} {target} aria-label={m.deviceform_site_setup_named({ name: item.label })}>
										{m.deviceform_site_setup()}<ArrowUpRight class="size-4" aria-hidden="true" />
									</Button>
								{/if}
							</li>
						{/each}
					</ul>
					{#if found.length > LIMIT}
						<Button size="sm" variant="ghost" class="mt-1" onclick={() => (showAll = !showAll)} aria-expanded={showAll}>
							{showAll ? m.deviceform_site_fewer() : m.deviceform_site_more({ count: found.length - LIMIT })}
						</Button>
					{/if}
				{/if}

				{#if added.length > 0}
					<ul class="mt-3 grid gap-1" aria-live="polite">
						{#each added as item (item.id)}
							<li class="flex items-center gap-2 text-sm text-ink">
								<Check class="size-4 text-signal-ink" aria-hidden="true" />
								<span>{m.deviceform_site_added({ name: item.name })}</span>
								<a href={`/targets/${item.id}`} {target} class="text-ink-2 underline hover:text-ink">{m.deviceform_site_open()}</a>
							</li>
						{/each}
					</ul>
				{/if}

				{#if error}<ErrorNotice {error} class="mt-3" />{/if}

				<!-- A server of this site with the agent: installed through the site's relay when it has one. -->
				<div class="mt-4 flex flex-wrap items-center gap-x-3 gap-y-2">
					<Button
						size="sm"
						variant="secondary"
						href={`${formHref}?${params({ kind: 'agent' })}`}
						{target}
						onclick={(event: MouseEvent) => {
							if (!onkind || newTab) return;
							event.preventDefault();
							onkind('agent', computed);
						}}
					>
						<KindIcon kind="agent" class="size-4" />{m.deviceform_site_agent()}
					</Button>
					{#if relayName}
						<span class="text-[0.8125rem] text-ink-2">{m.deviceform_site_agent_via({ relay: relayName })}</span>
					{/if}
				</div>

				{#if kinds.length > 0}
					<h3 class="mt-4 text-sm font-semibold text-ink">{m.deviceform_site_kinds()}</h3>
					<div class="mt-2 flex flex-wrap gap-2">
						{#each kinds as kind (kind)}
							<Button
								size="sm"
								variant="secondary"
								href={`${formHref}?${params({ kind })}`}
								{target}
								onclick={(event: MouseEvent) => {
									if (!onkind || newTab) return;
									event.preventDefault();
									onkind(kind, computed);
								}}
							>
								<KindIcon {kind} class="size-4" />{kindLabel(kind, collectors.find((c) => c.kind === kind)?.label)}
							</Button>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
	</section>
{/if}
