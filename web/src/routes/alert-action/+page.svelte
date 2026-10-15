<script lang="ts">
	/**
	 * `/alert-action#<token>` — the Acknowledge / Snooze link of a notification.
	 *
	 * Works without a session: the signed token is the authorisation, for one
	 * gesture on the alerts it names, once, for 24 hours. It sits in the URL
	 * fragment, which browsers never send to a server, and is posted in a JSON
	 * body. Opening the page only describes the link; nothing happens until the
	 * button is pressed, so a link scanner (mail, Slack, antivirus) acts on nothing.
	 */
	import { onMount } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { BellOff, Check, Clock } from 'lucide-svelte';
	import { inspectAlertLink, runAlertLink, toApiError } from '#lib/api/index.js';
	import type { AlertLinkResult, AlertLinkView } from '#lib/api/types.js';
	import { formatDateTime } from '#lib/format.js';
	import { Plate } from '#lib/ui/index.js';
	import { severityTone, severityWord } from '#lib/components/alerts/helpers.js';

	let token = $state('');
	let loading = $state(true);
	let view = $state<AlertLinkView | null>(null);
	let result = $state<AlertLinkResult | null>(null);
	let busy = $state(false);
	let error = $state<string | null>(null);

	/** One sentence per refusal the server can give; anything else is shown as sent. */
	function explain(cause: unknown): string {
		const apiError = toApiError(cause);
		if (apiError.status === 404 && /no longer active/i.test(apiError.message)) return m.alert_link_inactive();
		if (apiError.status === 404) return m.alert_link_invalid();
		if (apiError.status === 409) return m.alert_link_used();
		if (apiError.status === 429) return m.alert_link_too_many();
		return apiError.message;
	}

	onMount(() => {
		token = window.location.hash.replace(/^#/, '').trim();
		if (!token) {
			loading = false;
			return;
		}
		void (async () => {
			try {
				view = await inspectAlertLink(token);
			} catch (cause) {
				error = explain(cause);
			} finally {
				loading = false;
			}
		})();
	});

	async function run() {
		error = null;
		busy = true;
		try {
			result = await runAlertLink(token);
		} catch (cause) {
			error = explain(cause);
		} finally {
			busy = false;
		}
	}

	const snooze = $derived(view?.action === 'snooze');
	const title = $derived(snooze ? m.alert_link_snooze_title() : m.alert_link_ack_title());
	const doneSentence = $derived.by(() => {
		if (!result) return '';
		if (result.acked === 0) return m.alert_link_nothing_changed();
		return result.action === 'snooze'
			? m.alert_link_snooze_done({ time: formatDateTime(result.until) })
			: m.alert_link_ack_done();
	});
</script>

<svelte:head>
	<title>{m.alert_link_page_title({ title })}</title>
	<meta name="robots" content="noindex" />
	<meta name="referrer" content="no-referrer" />
</svelte:head>

<main class="flex min-h-full items-center justify-center bg-canvas px-4 py-16 text-ink">
	<div class="w-full max-w-md rounded-[var(--radius-card)] border border-line bg-surface p-6 shadow-lift">
		<h1 class="display text-2xl text-ink">{title}</h1>

		{#if loading}
			<p class="mt-3 text-sm text-ink-2" role="status">{m.alert_link_loading()}</p>
		{:else if !token}
			<p class="mt-3 text-sm text-ink-2" role="alert">{m.alert_link_incomplete()}</p>
		{:else if result}
			<p class="mt-3 flex items-start gap-2 text-sm text-ink" role="status">
				<Check class="mt-0.5 size-4 shrink-0 text-signal" aria-hidden="true" />
				{doneSentence}
			</p>
		{:else if view}
			<p class="mt-3 text-sm text-ink-2">{snooze ? m.alert_link_snooze_lead() : m.alert_link_ack_lead()}</p>

			{#if view.alerts.length > 0}
				<ul class="mt-4 grid gap-2" aria-label={m.alert_link_alerts_label()}>
					{#each view.alerts as alert, index (index)}
						<li class="flex flex-wrap items-center gap-x-2 gap-y-1 rounded-lg border border-line bg-surface-2 px-3 py-2 text-sm">
							<Plate tone={severityTone(alert.severity)} label={severityWord(alert.severity)} />
							<span class="min-w-0 break-words font-semibold">{alert.target_name ?? m.alert_link_unknown_device()}</span>
							<span class="min-w-0 break-words text-ink-2">{alert.rule_name}</span>
							{#if alert.acked}
								<span class="text-[0.8125rem] text-ink-2">{m.alert_link_already_acked({ time: formatDateTime(alert.acked_until) })}</span>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}

			{#if view.used}
				<p class="mt-4 text-sm text-ink-2" role="alert">{m.alert_link_used()}</p>
			{:else if view.alerts.length === 0}
				<p class="mt-4 text-sm text-ink-2" role="alert">{m.alert_link_inactive()}</p>
			{:else}
				<button
					type="button"
					class="mt-4 inline-flex min-h-10 items-center gap-2 rounded-lg bg-ink px-4 text-sm font-semibold text-surface transition hover:opacity-90 disabled:opacity-60"
					onclick={run}
					disabled={busy}
				>
					{#if snooze}<Clock class="size-4" aria-hidden="true" />{:else}<BellOff class="size-4" aria-hidden="true" />{/if}
					{busy ? m.alert_link_working() : snooze ? m.alert_link_snooze_button() : m.alert_link_ack_button()}
				</button>
				<p class="mt-2 text-[0.8125rem] text-ink-2">{m.alert_link_expires({ time: formatDateTime(view.expires_at) })}</p>
			{/if}
		{/if}

		{#if error}
			<p class="mt-3 text-sm text-warning-ink" role="alert">{error}</p>
		{/if}

		<p class="mt-6 text-sm">
			<a class="inline-flex min-h-10 items-center font-semibold text-ink underline decoration-line underline-offset-4 hover:decoration-ink" href="/alerts">{m.alert_link_open_app()}</a>
		</p>
	</div>
</main>
