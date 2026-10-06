<script lang="ts">
	/**
	 * Public status page — `/s/<slug>`. No session, no nav, no cookie: one
	 * document from `GET /api/public/status/<slug>`, refreshed every minute.
	 *
	 * Top to bottom: logo, title and the overall banner, the open announcements
	 * (incidents, maintenance) with their timeline, the services by group with
	 * their daily history bar, "Past incidents" by day for 30 days, then how to
	 * get updates (RSS, and email when the owner set up an SMTP channel). The
	 * theme follows the page setting (`light` / `dark`) or the visitor's system;
	 * the accent is one of a closed set of tokens, never free-form CSS.
	 */
	import { page } from '$app/state';
	import { CalendarClock, ExternalLink, Megaphone } from 'lucide-svelte';
	import { getPublicStatus, toApiError, type PublicIncident, type PublicStatus } from '$lib/api';
	import { formatDateTime, formatPercent, formatRelative, parseServerDate } from '$lib/format';
	import { theme } from '$lib/stores/theme.svelte';
	import { DecryptText, EmptyState, ErrorNotice, Plate, Skeleton } from '$lib/ui';
	import Logo from '$lib/components/Logo.svelte';
	import IncidentCard from '$lib/components/status/IncidentCard.svelte';
	import StatusUpdates from '$lib/components/status/StatusUpdates.svelte';
	import ServiceRow from '$lib/components/status/ServiceRow.svelte';
	import StatusMascot from '$lib/components/status/StatusMascot.svelte';
	import SceneBackdrop from '$lib/components/status/scenes/SceneBackdrop.svelte';
	import { knownScenes } from '$lib/components/status/scenes/registry';
	import { accentClass, homepageHost, isClosed, overallBanner } from '$lib/components/status/words';

	const REFRESH_MS = 60_000;

	const slug = $derived(page.params.slug ?? '');

	let status = $state<PublicStatus | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);
	let lastChecked = $state<Date | null>(null);

	async function load(signal?: AbortSignal) {
		try {
			status = await getPublicStatus(slug, signal);
			error = null;
			lastChecked = new Date();
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			// A failed refresh keeps the last good document on screen.
			if (!status) error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const current = slug;
		if (!current) return;
		const controller = new AbortController();
		loading = true;
		status = null;
		void load(controller.signal);
		const timer = setInterval(() => void load(controller.signal), REFRESH_MS);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	// The page decides its light: a fixed theme overrides whatever the visitor
	// (or the admin, on this browser) chose; `auto` follows the system.
	$effect(() => {
		const wanted = status?.page.theme ?? 'auto';
		const dark = wanted === 'auto' ? theme.resolved === 'dark' : wanted === 'dark';
		document.documentElement.classList.toggle('dark', dark);
		return () => theme.apply();
	});

	const notFound = $derived(error !== null && toApiError(error).status === 404);
	const banner = $derived(status ? overallBanner(status) : null);
	const days = $derived(status?.page.show_uptime_days ?? 90);
	const bannerTone = $derived(banner?.tone ?? 'signal');

	// Open announcements sit at the top; everything closed goes to the history.
	const active = $derived<PublicIncident[]>(
		status ? [...status.maintenance, ...status.incidents].filter((i) => !isClosed(i.status)) : []
	);
	const past = $derived<PublicIncident[]>(
		status ? [...status.incidents, ...status.maintenance].filter((i) => isClosed(i.status)) : []
	);

	// "Past incidents" grouped by the day they started, newest day first.
	const dayFormat = new Intl.DateTimeFormat('en-GB', { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' });
	const pastByDay = $derived.by(() => {
		const groups = new Map<string, { label: string; incidents: PublicIncident[] }>();
		for (const incident of past) {
			const date = parseServerDate(incident.starts_at);
			const key = date ? date.toISOString().slice(0, 10) : incident.starts_at;
			const label = date ? dayFormat.format(date) : incident.starts_at;
			const group = groups.get(key) ?? { label, incidents: [] };
			group.incidents.push(incident);
			groups.set(key, group);
		}
		return [...groups.entries()].sort((a, b) => (a[0] < b[0] ? 1 : -1)).map(([, group]) => group);
	});

	const serviceCount = $derived(status ? status.groups.reduce((n, g) => n + g.items.length, 0) : 0);

	// Banner scene: an empty (or unknown) list keeps the plain page.
	const sceneIds = $derived(knownScenes(status?.page.scenes));
	const withScene = $derived(sceneIds.length > 0);

	// The sentence on the scene keeps its last word in the tone's colour.
	const TONE_WORD: Record<string, string> = {
		signal: 'text-signal-ink',
		advisory: 'text-advisory-ink',
		warning: 'text-warning-ink',
		info: 'text-info-ink'
	};
	const TONE_VAR: Record<string, string> = {
		signal: 'var(--c-signal)',
		advisory: 'var(--c-advisory)',
		warning: 'var(--c-warning)',
		info: 'var(--c-info)'
	};
	const BAR_TONE: Record<string, string> = {
		signal: 'border-signal/30 bg-signal-soft',
		advisory: 'border-advisory/35 bg-advisory-soft',
		warning: 'border-warning/35 bg-warning-soft',
		info: 'border-info/30 bg-info-soft'
	};
	const labelHead = $derived(banner ? banner.label.slice(0, banner.label.lastIndexOf(' ') + 1) : '');
	const labelTail = $derived(banner ? banner.label.slice(banner.label.lastIndexOf(' ') + 1) : '');

	// One line above the services: counts by state and the mean uptime.
	const tally = $derived.by(() => {
		const items = status ? status.groups.flatMap((g) => g.items) : [];
		const count = (state: string) => items.filter((i) => i.state === state).length;
		const window = days >= 90 ? 90 : 30;
		const values = items.map((i) => (window === 90 ? i.uptime_90d : i.uptime_30d)).filter((v): v is number => v !== null);
		const up = count('up');
		const degraded = count('degraded');
		const down = count('down');
		const maintenance = count('maintenance');
		const affected = degraded + down + maintenance;
		let headline = `All ${items.length} services operational`;
		if (items.length === 1 && affected === 0) headline = '1 service operational';
		if (affected > 0) {
			const only = [
				[down, 'down'],
				[degraded, 'degraded'],
				[maintenance, 'in maintenance']
			].filter(([n]) => (n as number) > 0);
			headline =
				only.length === 1
					? `${affected} of ${items.length} service${items.length > 1 ? 's' : ''} ${only[0][1]}`
					: `${affected} of ${items.length} services affected`;
		}
		return {
			total: items.length,
			headline,
			detail: `${up} operational · ${down} down${values.length ? ` · ${formatPercent(values.reduce((a, b) => a + b, 0) / values.length)} over ${window} days` : ''}`
		};
	});
</script>

<svelte:head>
	<title>{status ? `${status.page.title} · Status` : 'Status'}</title>
	<meta name="robots" content="noindex" />
</svelte:head>

{#snippet announcements()}
	{#if active.length > 0}
		<section class="rise-in mt-8" style="--rise-delay: 80ms" aria-labelledby="announcements">
			<h2 id="announcements" class="text-base font-semibold tracking-tight text-ink">
				{active.length === 1 ? 'Current announcement' : 'Current announcements'}
			</h2>
			<div class="mt-3 grid gap-3">
				{#each active as incident (incident.kind + incident.starts_at + incident.title)}
					<IncidentCard {incident} />
				{/each}
			</div>
		</section>
	{/if}
{/snippet}

{#snippet services(compact: boolean)}
	{#if status}
		<section class="rise-in mt-8" style="--rise-delay: 120ms" aria-labelledby="services">
			<div class="flex items-baseline justify-between gap-3">
				<h2 id="services" class="text-base font-semibold tracking-tight text-ink">Services</h2>
				<p class="text-[0.8125rem] text-ink-2">Uptime over the last <span class="tnum">{days}</span> days</p>
			</div>
			{#if serviceCount === 0}
				<EmptyState class="mt-3" icon={Megaphone} title="No service listed yet." description="This page has nothing to show for now." />
			{:else}
				<div class={`mt-3 grid ${compact ? 'gap-3' : 'gap-4'}`}>
					{#each status.groups as group (group.name)}
						<div class="rounded-[var(--radius-card)] border border-line bg-surface shadow-lift">
							{#if group.name}
								<h3 class="label-tape border-b border-line px-4 py-2.5 text-ink-2 sm:px-5">{group.name}</h3>
							{/if}
							<ul class="divide-y divide-line" role="list">
								{#each group.items as item (item.label)}
									<ServiceRow {item} {days} {compact} />
								{/each}
							</ul>
						</div>
					{/each}
				</div>
			{/if}
		</section>
	{/if}
{/snippet}

{#snippet pastIncidents()}
	<section class="rise-in mt-10" style="--rise-delay: 160ms" aria-labelledby="past">
		<h2 id="past" class="text-base font-semibold tracking-tight text-ink">Past incidents</h2>
		<p class="mt-0.5 text-[0.8125rem] text-ink-2">Last 30 days.</p>
		{#if pastByDay.length === 0}
			<EmptyState class="mt-3" icon={CalendarClock} title="No incident in the last 30 days." tone="signal" />
		{:else}
			<div class="mt-3 grid gap-5">
				{#each pastByDay as day (day.label)}
					<div>
						<h3 class="graticule pb-1.5 text-sm font-semibold text-ink">{day.label}</h3>
						<div class="mt-2 grid gap-2">
							{#each day.incidents as incident (incident.kind + incident.starts_at + incident.title)}
								<IncidentCard {incident} compact />
							{/each}
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</section>
{/snippet}

{#snippet pageFooter()}
	<footer class="mt-12 grid justify-items-center gap-3 text-center text-[0.8125rem] text-ink-2">
		{#if status?.page.footer_text}
			<p class="max-w-prose whitespace-pre-line">{status.page.footer_text}</p>
		{/if}
		<p class="flex items-center gap-2">
			<Logo class="size-5" />
			<span>Powered by <a class="font-semibold text-ink underline decoration-line underline-offset-2 hover:decoration-ink" href="https://github.com/laupernoe/dumbmonit" rel="noreferrer">DumbMonit</a></span>
		</p>
	</footer>
{/snippet}

<div class={`min-h-full bg-canvas text-ink ${error || loading || !status || !withScene ? 'border-t-4 border-accent' : ''} ${accentClass(status?.page.accent)}`}>
	{#if !error && !loading && status && banner && withScene}
		<!-- Banner scene: the head of the page sits on the city, the column starts on its fade. -->
		<div
			class="relative h-[380px] overflow-hidden sm:h-[570px]"
			style={`--scene-tone: ${TONE_VAR[bannerTone] ?? 'var(--c-signal)'}`}
		>
			<SceneBackdrop scenes={sceneIds} rotation={status.page.scene_rotation} />
			<div class="relative z-[2] mx-auto w-full max-w-3xl px-4 pt-6 sm:px-6">
				<header class="flex flex-wrap items-center gap-x-4 gap-y-2">
					<div class="flex min-w-0 items-center gap-3">
						{#if status.page.logo_url}
							<img src={status.page.logo_url} alt="" class="size-10 shrink-0 rounded-lg object-contain" />
						{/if}
						<div class="min-w-0">
							<h1 class="text-lg leading-tight font-semibold tracking-tight break-words text-ink">{status.page.title}</h1>
							{#if status.page.description}
								<p class="max-w-prose text-xs text-ink-2">{status.page.description}</p>
							{/if}
						</div>
					</div>
					<div class="ml-auto flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-ink-2">
						{#if lastChecked}
							<span class="hidden sm:inline">Checked <time class="tnum" datetime={lastChecked.toISOString()} title={formatDateTime(lastChecked)}>{formatRelative(lastChecked)}</time></span>
						{/if}
						{#if status.page.homepage_url}
							<a class="inline-flex items-center gap-1 font-semibold text-ink underline decoration-ink/30 underline-offset-4 hover:decoration-ink" href={status.page.homepage_url} rel="noopener noreferrer nofollow">
								{homepageHost(status.page.homepage_url)}
								<ExternalLink class="size-3" aria-hidden="true" />
							</a>
						{/if}
						<a class="font-semibold text-ink underline decoration-ink/30 underline-offset-4 hover:decoration-ink" href={`/api/public/status/${encodeURIComponent(status.page.slug)}/rss`}>RSS</a>
						{#if status.page.subscribe}
							<a class="font-semibold text-ink underline decoration-ink/30 underline-offset-4 hover:decoration-ink" href="#updates">Email</a>
						{/if}
					</div>
				</header>

				<section class="rise-in mt-8 max-w-[21rem] sm:mt-12 sm:max-w-md" aria-live="polite" aria-label="Current status">
					<Plate tone={bannerTone} size="md" label={banner.plate} />
					<p class="display mt-3 text-3xl text-balance text-ink sm:text-5xl">
						{labelHead}<span class={TONE_WORD[bannerTone] ?? 'text-ink'}>{labelTail}</span>
					</p>
				</section>
			</div>
		</div>

		<main class="relative z-[3] mx-auto -mt-[34px] w-full max-w-3xl px-4 pb-16 sm:px-6">
			{#if tally.total > 0}
				<section
					class={`rise-in flex flex-wrap items-center justify-between gap-x-4 gap-y-1 rounded-[var(--radius-card)] border px-4 py-3.5 font-semibold shadow-lift sm:px-5 ${BAR_TONE[bannerTone] ?? BAR_TONE.signal}`}
					style="--rise-delay: 40ms"
				>
					<p class="text-base text-ink sm:text-lg">{tally.headline}</p>
					<p class="tnum text-xs font-medium text-ink-2">{tally.detail}</p>
				</section>
			{/if}
			{@render announcements()}
			{@render services(true)}
			{@render pastIncidents()}
			<StatusUpdates slug={status.page.slug} subscribe={status.page.subscribe} />
			{@render pageFooter()}
		</main>
	{:else}
		<main class="mx-auto w-full max-w-3xl px-4 pt-8 pb-16 sm:px-6 sm:pt-12">
			{#if error && notFound}
				<EmptyState mascot="dizzy" title="This status page does not exist." description="Check the link you were given, or ask whoever runs this DumbMonit for the right one." />
			{:else if error}
				<ErrorNotice {error} title="Could not load the status page" onretry={() => void load()} />
			{:else if loading || !status || !banner}
				<div class="grid gap-6" aria-busy="true" aria-label="Loading">
					<Skeleton class="h-9 w-2/3" />
					<Skeleton class="h-20 w-full" />
					<Skeleton class="h-28 w-full" />
					<Skeleton class="h-28 w-full" />
				</div>
			{:else}
				<header class="rise-in flex flex-wrap items-start justify-between gap-x-6 gap-y-3">
					<div class="flex min-w-0 items-center gap-4">
						{#if status.page.logo_url}
							<img src={status.page.logo_url} alt="" class="size-12 shrink-0 rounded-lg object-contain sm:size-14" />
						{/if}
						<div class="min-w-0">
							<h1 class="display text-3xl break-words text-ink sm:text-4xl">{status.page.title}</h1>
							{#if status.page.description}
								<p class="mt-1.5 max-w-prose text-base text-ink-2">{status.page.description}</p>
							{/if}
						</div>
					</div>
					{#if status.page.homepage_url}
						<a
							class="inline-flex items-center gap-1.5 rounded-md py-1 text-sm font-semibold text-accent underline decoration-accent/40 underline-offset-4 hover:decoration-accent"
							href={status.page.homepage_url}
							rel="noopener noreferrer nofollow"
						>
							{homepageHost(status.page.homepage_url)}
							<ExternalLink class="size-3.5" aria-hidden="true" />
						</a>
					{/if}
				</header>

				<!-- Overall banner: the one-second answer. -->
				<section
					class={`rise-in mt-6 flex flex-wrap items-center justify-between gap-3 rounded-[var(--radius-card)] border px-4 py-4 shadow-lift sm:px-5 ${bannerTone === 'signal' ? 'border-signal/30 bg-signal-soft' : bannerTone === 'advisory' ? 'border-advisory/35 bg-advisory-soft' : bannerTone === 'warning' ? 'border-warning/35 bg-warning-soft' : 'border-info/30 bg-info-soft'}`}
					style="--rise-delay: 40ms"
					aria-live="polite"
				>
					<div class="flex min-w-0 items-center gap-3">
						<Plate tone={bannerTone} size="md" label={banner.plate} />
						<p class="display text-xl text-ink sm:text-2xl">
							<DecryptText text={banner.label} tag="span" />
						</p>
					</div>
					{#if lastChecked}
						<p class="text-[0.8125rem] text-ink-2">
							Checked <time class="tnum" datetime={lastChecked.toISOString()} title={formatDateTime(lastChecked)}>{formatRelative(lastChecked)}</time>
						</p>
					{/if}
				</section>

				<StatusMascot tone={bannerTone} />

				{@render announcements()}
				{@render services(false)}
				{@render pastIncidents()}
				<StatusUpdates slug={status.page.slug} subscribe={status.page.subscribe} />
			{/if}

			{@render pageFooter()}
		</main>
	{/if}
</div>
