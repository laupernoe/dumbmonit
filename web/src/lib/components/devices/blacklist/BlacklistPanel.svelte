<script lang="ts">
	/**
	 * Mail server reputation and IP range blocklist: the last check as the
	 * probe stored it, list by list, then every listing with its lookup and
	 * delisting pages. For a mail server, also what Gmail, Outlook.com and
	 * Yahoo weigh (reverse DNS, SPF, DMARC) and their sender tools, since
	 * they publish no blocklist that can be queried. Refreshed every minute;
	 * opening the page never queries a blocklist.
	 */
	import { untrack } from 'svelte';
	import { ExternalLink } from 'lucide-svelte';
	import { getBlacklistOverview } from '#lib/api/blacklist.js';
	import type { BlacklistAddress, BlacklistCheck, BlacklistList, BlacklistOverview, Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import type { Tone } from '#lib/ui/index.js';
	import Figure from '../Figure.svelte';
	import { formatAgo, formatSpan, formatUnix } from '../pbs/format';
	import { m } from '#lib/paraglide/messages.js';
	import { authVerdict, checkTone, checkWord, linkLabel, overallVerdict, reverseVerdict, sourceLabel } from './format';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let view = $state<BlacklistOverview | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			view = await getBlacklistOverview(target.id, signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void target.id;
		loading = true;
		view = null;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const isMail = $derived(target.kind === 'mail_blacklist');
	const hasResult = $derived(view !== null && view.addresses_checked !== null);
	const overall = $derived(view ? overallVerdict(view) : null);
	/** A range only stores listings and unanswered checks; a mail server stores them all. */
	const details = $derived((view?.checks ?? []).filter((c: BlacklistCheck) => c.state !== 'not_applicable'));
	const listTone = (list: BlacklistList): Tone =>
		list.listed > 0 ? (list.major ? 'warning' : 'advisory') : list.unknown > 0 ? 'ghost' : 'signal';
	const listWord = (list: BlacklistList): string =>
		list.listed > 0
			? m.devicesb_blacklist_state_listed()
			: list.unknown > 0
				? m.devicesb_blacklist_state_unknown()
				: m.devicesb_blacklist_state_clean();
	const reverse = (address: BlacklistAddress) => reverseVerdict(address);
</script>

{#if error}
	<Panel title={m.devicesb_blacklist_title()} class="rise-in">
		<ErrorNotice {error} title={m.devicesb_blacklist_error_title()} onretry={() => void load()} />
	</Panel>
{:else if loading}
	<Panel title={m.devicesb_blacklist_title()} padded={false} class="rise-in">
		<div class="flex flex-col gap-3 px-5 py-4" aria-busy="true" aria-label={m.devicesb_blacklist_loading()}>
			<Skeleton class="h-10 w-full" rows={3} />
		</div>
	</Panel>
{:else if view}
	<div class="flex flex-col gap-6">
		<Panel title={m.devicesb_blacklist_title()} padded={false} class="rise-in">
			{#snippet aside()}
				{#if overall}
					<span class="flex flex-wrap items-center justify-end gap-2">
						<Plate tone={overall.tone} label={overall.word} size="md" />
						{#if view?.scan_age_seconds !== null && view?.sampled_at !== null}
							<span class="tnum text-[0.75rem] text-ink-3" title={formatUnix((view?.sampled_at ?? 0) - (view?.scan_age_seconds ?? 0))}>
								{m.devicesb_blacklist_checked_ago({ ago: formatAgo((view?.sampled_at ?? 0) - (view?.scan_age_seconds ?? 0)) })}
							</span>
						{/if}
					</span>
				{/if}
			{/snippet}
			{#if view.sampled_at === null}
				<p class="px-5 py-4 text-sm text-ink-2">{m.devicesb_blacklist_waiting()}</p>
			{:else if !hasResult}
				<p class="px-5 py-4 text-sm text-ink-2">{m.devicesb_blacklist_running_first()}</p>
			{:else}
				<div class="flex flex-col divide-y divide-line">
					{#if view.scan_running}
						<p class="px-4 py-3 text-sm text-ink-2 sm:px-5">{m.devicesb_blacklist_running_again()}</p>
					{/if}
					<div class="grid grid-cols-2 gap-x-4 gap-y-3 px-4 pt-4 pb-2 sm:grid-cols-4 sm:gap-x-6 sm:px-5">
						<Figure label={m.devicesb_blacklist_figure_addresses()} value={String(view.addresses_checked ?? '—')} />
						<Figure
							label={m.devicesb_blacklist_figure_listed()}
							value={String(view.addresses_listed ?? '—')}
							tone={(view.addresses_listed ?? 0) > 0 ? 'warning' : 'ink'}
						/>
						<Figure
							label={m.devicesb_blacklist_figure_lists()}
							value={String(view.listed_lists ?? '—')}
							tone={(view.listed_major_lists ?? 0) > 0 ? 'warning' : (view.listed_lists ?? 0) > 0 ? 'advisory' : 'ink'}
						/>
						<Figure label={m.devicesb_blacklist_figure_unknown()} value={String(view.unknown_checks ?? '—')} />
					</div>
					{#if view.scan_duration_seconds !== null}
						<p class="px-4 py-3 text-[0.8125rem] text-ink-3 sm:px-5">
							{m.devicesb_blacklist_duration({ span: formatSpan(Math.max(1, Math.round(view.scan_duration_seconds))) })}
						</p>
					{/if}
				</div>
			{/if}
		</Panel>

		{#if hasResult && view.lists.length > 0}
			<Panel title={m.devicesb_blacklist_lists_title()} description={m.devicesb_blacklist_lists_note()} padded={false} class="rise-in">
				<ul class="divide-y divide-line">
					{#each view.lists as list (list.id)}
						<li class="flex min-h-10 flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5 sm:px-5">
							<span class="min-w-0 flex-1">
								<span class="text-sm font-semibold text-ink">{list.name}</span>
								{#if list.major}
									<span class="ml-1.5 text-[0.75rem] text-ink-3">{m.devicesb_blacklist_list_major()}</span>
								{/if}
								<span class="block truncate font-mono text-[0.75rem] text-ink-3" title={list.zone}>{list.zone}</span>
							</span>
							<span class="tnum text-[0.8125rem] text-ink-2">
								{m.devicesb_blacklist_list_counts({ listed: String(list.listed), checked: String(list.checked) })}
							</span>
							<Plate tone={listTone(list)} label={listWord(list)} />
						</li>
					{/each}
				</ul>
			</Panel>
		{/if}

		{#if hasResult}
			<Panel title={isMail ? m.devicesb_blacklist_details_title_mail() : m.devicesb_blacklist_details_title_range()} padded={false} class="rise-in">
				{#if details.length === 0}
					<p class="px-5 py-4 text-sm text-ink-2">{m.devicesb_blacklist_details_none()}</p>
				{:else}
					<ul class="divide-y divide-line">
						{#each details as check (`${check.list}|${check.subject}`)}
							<li class="flex flex-col gap-1.5 px-4 py-3 sm:px-5">
								<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
									<Plate tone={checkTone(check)} label={checkWord(check)} />
									<span class="min-w-0 font-mono text-sm break-all text-ink">{check.subject}</span>
									<span class="text-sm text-ink-2">{check.list_name}</span>
									{#if check.meaning}
										<span class="text-[0.8125rem] text-ink-2">{check.meaning}</span>
									{/if}
									{#if check.codes.length > 0}
										<span class="tnum font-mono text-[0.75rem] text-ink-3">{check.codes.join(', ')}</span>
									{/if}
								</div>
								{#if check.reason}
									<p class="text-[0.8125rem] break-words text-ink-2">{check.reason}</p>
								{/if}
								{#if check.lookup_url || check.delist_url}
									<div class="flex flex-wrap gap-x-4">
										{#if check.lookup_url}
											<a class="inline-flex min-h-10 items-center gap-1 text-sm text-info-ink underline-offset-2 hover:underline" href={check.lookup_url} target="_blank" rel="noopener noreferrer">
												{m.devicesb_blacklist_lookup()}<ExternalLink class="size-3.5" aria-hidden="true" />
											</a>
										{/if}
										{#if check.delist_url && check.delist_url !== check.lookup_url}
											<a class="inline-flex min-h-10 items-center gap-1 text-sm font-semibold text-info-ink underline-offset-2 hover:underline" href={check.delist_url} target="_blank" rel="noopener noreferrer">
												{m.devicesb_blacklist_delist()}<ExternalLink class="size-3.5" aria-hidden="true" />
											</a>
										{:else if check.delist_url}
											<span class="inline-flex min-h-10 items-center text-[0.8125rem] text-ink-3">{m.devicesb_blacklist_delist_same_page()}</span>
										{/if}
									</div>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</Panel>
		{/if}

		{#if hasResult && isMail && (view.addresses.length > 0 || view.auth)}
			<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
				{#if view.addresses.length > 0}
					<Panel title={m.devicesb_blacklist_reverse_title()} description={m.devicesb_blacklist_reverse_note()} padded={false} class="rise-in">
						<ul class="divide-y divide-line">
							{#each view.addresses as address (address.address)}
								{@const verdict = reverse(address)}
								<li class="flex min-h-10 flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5 sm:px-5">
									<span class="min-w-0 flex-1">
										<span class="block font-mono text-sm break-all text-ink">{address.address}</span>
										<span class="block text-[0.75rem] break-all text-ink-3">
											{sourceLabel(address)}{#if address.ptr} · {address.ptr}{/if}
										</span>
									</span>
									<Plate tone={verdict.tone} label={verdict.word} />
								</li>
							{/each}
						</ul>
					</Panel>
				{/if}
				{#if view.auth}
					{@const auth = view.auth}
					<Panel title={m.devicesb_blacklist_auth_title()} description={m.devicesb_blacklist_auth_note()} padded={false} class="rise-in">
						<ul class="divide-y divide-line">
							{#each [{ name: 'SPF', verdict: authVerdict(auth.spf_records, null) }, { name: 'DMARC', verdict: authVerdict(auth.dmarc_records, auth.dmarc_policy) }] as row (row.name)}
								<li class="flex min-h-10 flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5 sm:px-5">
									<span class="min-w-0 flex-1 text-sm font-semibold text-ink">{row.name}</span>
									{#if row.verdict.detail}
										<span class="text-[0.8125rem] text-ink-2">{row.verdict.detail}</span>
									{/if}
									<Plate tone={row.verdict.tone} label={row.verdict.word} />
								</li>
							{/each}
							{#if auth.mx_hosts !== null}
								<li class="flex min-h-10 items-center gap-3 px-4 py-2.5 sm:px-5">
									<span class="min-w-0 flex-1 text-sm font-semibold text-ink">MX</span>
									<span class="tnum text-sm text-ink">{m.devicesb_blacklist_auth_mx({ count: String(auth.mx_hosts) })}</span>
								</li>
							{/if}
						</ul>
					</Panel>
				{/if}
			</div>
		{/if}

		{#if hasResult && view.providers.length > 0}
			<Panel title={m.devicesb_blacklist_providers_title()} description={m.devicesb_blacklist_providers_note()} padded={false} class="rise-in">
				<ul class="divide-y divide-line">
					{#each view.providers as provider (provider.id)}
						<li class="flex flex-col gap-1 px-4 py-3 sm:flex-row sm:items-baseline sm:gap-4 sm:px-5">
							<span class="shrink-0 text-sm font-semibold text-ink sm:w-44">{provider.name}</span>
							<span class="flex flex-wrap gap-x-4">
								{#each provider.links as link (link.url)}
									<a class="inline-flex min-h-10 items-center gap-1 text-sm text-info-ink underline-offset-2 hover:underline" href={link.url} target="_blank" rel="noopener noreferrer">
										{linkLabel(link)}<ExternalLink class="size-3.5" aria-hidden="true" />
									</a>
								{/each}
							</span>
						</li>
					{/each}
				</ul>
			</Panel>
		{/if}
	</div>
{/if}
