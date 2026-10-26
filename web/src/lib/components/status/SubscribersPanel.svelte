<script lang="ts">
	/**
	 * Subscribers of a page, by email, SMS, signed webhook or Slack: the channels
	 * the page opens, who is confirmed, who is still pending, a CSV export and a
	 * way to remove one. Phone numbers and URLs are masked by the server. Pending
	 * requests disappear by themselves after two days.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import {
		deleteStatusSubscriber,
		getStatusSubscriptionSettings,
		listStatusSubscribers,
		saveStatusSubscriptionSettings,
		type Channel,
		type StatusPage,
		type StatusSubscriber,
		type StatusSubscriptionKind,
		type StatusSubscriptionSettings
	} from '#lib/api/index.js';
	import { formatDateTime, parseServerDate } from '#lib/format.js';
	import { Button, Confirm, EmptyState, ErrorNotice, Field, Panel, Plate, Skeleton, Toggle } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Download } from 'lucide-svelte';

	interface Props {
		page: StatusPage;
		/** Notification channels; Vonage, Telnyx and ASPSMS ones can text subscribers. */
		channels?: Channel[];
	}

	let { page, channels = [] }: Props = $props();

	const SMS_KINDS = ['vonage', 'telnyx', 'aspsms'];
	const smsChannels = $derived(channels.filter((channel) => SMS_KINDS.includes(channel.kind)));
	const KIND_LABEL: Record<StatusSubscriptionKind, () => string> = {
		email: () => m.status_sub_kind_email(),
		sms: () => m.status_sub_kind_sms(),
		webhook: () => m.status_sub_kind_webhook(),
		slack: () => m.status_sub_kind_slack()
	};

	// Channels besides email.
	let settings = $state<StatusSubscriptionSettings>({ sms_channel_id: null, webhook: false, slack: false });
	let savingSettings = $state(false);
	let savedSettings = $state(false);
	let settingsError = $state<unknown>(null);

	$effect(() => {
		const controller = new AbortController();
		getStatusSubscriptionSettings(page.id, controller.signal)
			.then((loaded) => (settings = loaded))
			.catch((cause) => {
				if (!(cause instanceof DOMException && cause.name === 'AbortError')) settingsError = cause;
			});
		return () => controller.abort();
	});

	async function saveSettings() {
		savingSettings = true;
		savedSettings = false;
		settingsError = null;
		try {
			settings = await saveStatusSubscriptionSettings(page.id, settings);
			savedSettings = true;
		} catch (cause) {
			settingsError = cause;
		} finally {
			savingSettings = false;
		}
	}

	let filter = $state<StatusSubscriptionKind | 'all'>('all');
	const shown = $derived(filter === 'all' ? subscribers : subscribers.filter((s) => s.kind === filter));

	let subscribers = $state<StatusSubscriber[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		loading = true;
		try {
			subscribers = await listStatusSubscribers(page.id, signal);
			error = null;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});

	async function remove(subscriber: StatusSubscriber) {
		try {
			await deleteStatusSubscriber(page.id, subscriber.id);
			subscribers = subscribers.filter((s) => s.id !== subscriber.id);
		} catch (cause) {
			error = cause;
		}
	}

	const confirmed = $derived(subscribers.filter((s) => s.confirmed_at !== null).length);
	function when(stamp: string): string {
		const date = parseServerDate(stamp);
		return date ? formatDateTime(date) : stamp;
	}
</script>

<div class="grid gap-4">
<Panel title={m.status_sub_channels_title()} description={m.status_sub_channels_help()}>
	<div class="grid gap-4 sm:grid-cols-2">
		<Field label={m.status_sub_sms_label()} for="sub-sms" help={smsChannels.length === 0 && settings.sms_channel_id === null ? m.status_sub_sms_no_channel() : m.status_sub_sms_help()}>
			<select
				id="sub-sms"
				class="input"
				value={settings.sms_channel_id === null ? '' : String(settings.sms_channel_id)}
				onchange={(event) => (settings.sms_channel_id = event.currentTarget.value === '' ? null : Number(event.currentTarget.value))}
				disabled={savingSettings || (smsChannels.length === 0 && settings.sms_channel_id === null)}
			>
				<option value="">{m.status_form_subscribers_off()}</option>
				{#each smsChannels as channel (channel.id)}
					<option value={String(channel.id)}>{channel.enabled ? m.status_sub_sms_via({ name: channel.name }) : m.status_sub_sms_via_disabled({ name: channel.name })}</option>
				{/each}
			</select>
		</Field>
		<div class="grid content-start gap-3">
			<div class="flex items-start justify-between gap-3">
				<div class="min-w-0">
					<label for="sub-webhook" class="text-sm font-semibold text-ink">{m.status_sub_webhook_toggle()}</label>
					<p class="text-[0.8125rem] text-ink-2">{m.status_sub_webhook_toggle_help()}</p>
				</div>
				<Toggle id="sub-webhook" bind:checked={settings.webhook} disabled={savingSettings} label={m.status_sub_webhook_toggle()} />
			</div>
			<div class="flex items-start justify-between gap-3">
				<div class="min-w-0">
					<label for="sub-slack" class="text-sm font-semibold text-ink">{m.status_sub_slack_toggle()}</label>
					<p class="text-[0.8125rem] text-ink-2">{m.status_sub_slack_toggle_help()}</p>
				</div>
				<Toggle id="sub-slack" bind:checked={settings.slack} disabled={savingSettings} label={m.status_sub_slack_toggle()} />
			</div>
		</div>
	</div>
	{#if settingsError}
		<div class="mt-3"><ErrorNotice error={settingsError} title={m.status_sub_save_error()} /></div>
	{/if}
	<div class="mt-4 flex flex-wrap items-center gap-3">
		<Button onclick={saveSettings} loading={savingSettings}>{m.status_sub_save()}</Button>
		{#if savedSettings}<span class="text-sm text-ink-2" role="status">{m.status_sub_saved()}</span>{/if}
	</div>
</Panel>

<Panel title={m.status_subscribers_title()} description={m.status_subscribers_summary({ confirmed, pending: subscribers.length - confirmed })}>
	{#if subscribers.length > 0}
		<div class="mb-3 flex flex-wrap items-center justify-between gap-2">
			<div class="flex flex-wrap gap-1.5" role="group" aria-label={m.status_sub_filter_label()}>
				{#each ['all', 'email', 'sms', 'webhook', 'slack'] as const as option (option)}
					<button
						type="button"
						class={`inline-flex min-h-10 items-center rounded-full border px-3 text-sm ${filter === option ? 'border-ink bg-ink text-surface' : 'border-line text-ink-2 hover:text-ink'}`}
						aria-pressed={filter === option}
						onclick={() => (filter = option)}
					>
						{option === 'all' ? m.status_sub_filter_all() : KIND_LABEL[option]()}
					</button>
				{/each}
			</div>
			<a
				class="inline-flex min-h-10 items-center gap-1.5 text-sm font-semibold text-ink underline decoration-line underline-offset-4 hover:decoration-ink"
				href={`/api/status-pages/${page.id}/subscribers.csv`}
				download
			>
				<Download class="size-4" aria-hidden="true" />
				{m.status_sub_export()}
			</a>
		</div>
	{/if}
	{#if error}
		<ErrorNotice {error} title={m.status_subscribers_load_error()} onretry={() => void load()} />
	{:else if loading}
		<Skeleton class="h-16 w-full" />
	{:else if subscribers.length === 0}
		<EmptyState title={m.status_subscribers_empty_title()} description={m.status_subscribers_empty_description()} />
	{:else}
		<ul class="divide-y divide-line" role="list">
			{#each shown as subscriber (subscriber.id)}
				<li class="flex flex-wrap items-center justify-between gap-2 py-2.5">
					<div class="flex min-w-0 items-center gap-3">
						<Plate tone={subscriber.confirmed_at ? 'signal' : 'ghost'} label={subscriber.confirmed_at ? m.status_subscribers_confirmed() : m.status_subscribers_pending()} />
						<span class="shrink-0 rounded border border-line px-1.5 py-0.5 text-xs font-semibold text-ink-2">{KIND_LABEL[subscriber.kind]()}</span>
						<span class="truncate text-sm text-ink">{subscriber.destination}</span>
						{#if subscriber.components}
							<span class="shrink-0 text-xs text-ink-3">{m.status_sub_followed_some({ count: subscriber.components.length })}</span>
						{/if}
					</div>
					<div class="flex items-center gap-3">
						<span class="text-[0.8125rem] text-ink-3">{m.status_subscribers_since({ date: when(subscriber.confirmed_at ?? subscriber.created_at) })}</span>
						{#if auth.isAdmin}
							<Confirm confirmLabel={m.status_subscribers_remove_confirm()} onconfirm={() => remove(subscriber)}>{m.status_subscribers_remove()}</Confirm>
						{/if}
					</div>
				</li>
			{/each}
		</ul>
	{/if}
</Panel>
</div>
