<script lang="ts">
	/**
	 * Alerts → Notifications → Notification policy: the global knobs that keep notifications
	 * quiet — batching window, hourly cap per channel, flap detection, and the
	 * public URL used for device links in messages. Per-channel choices (what a
	 * channel hears, quiet hours) live on each channel above; this panel only
	 * summarises them.
	 */
	import { SlidersHorizontal } from 'lucide-svelte';
	import {
		getNotificationPolicy,
		listChannels,
		updateNotificationPolicy,
		type Channel,
		type NotificationPolicy
	} from '#lib/api/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, ClickSpark, ErrorNotice, Field, Panel, Plate, Skeleton, Toggle } from '#lib/ui/index.js';
	import { matcherIsEmpty, matcherSentence } from '#lib/components/alerts/helpers.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';

	let policy = $state<NotificationPolicy | null>(null);
	let channels = $state<Channel[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			const [current, list] = await Promise.all([getNotificationPolicy(signal), listChannels(signal)]);
			policy = current;
			channels = Array.isArray(list) ? list : [];
			batchWindow = current.batch_window_secs;
			maxPerHour = current.max_per_hour === 0 ? '' : String(current.max_per_hour);
			flapOn = current.flap_events > 0;
			flapEvents = current.flap_events > 0 ? String(current.flap_events) : '4';
			flapWindow = current.flap_window_secs;
			flapHold = current.flap_hold_secs;
			publicUrl = current.public_url;
			escalateAfter = current.escalate_channel === null ? 0 : current.escalate_after_secs;
			escalateChannel = current.escalate_channel === null ? '' : String(current.escalate_channel);
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

	// --- Form -----------------------------------------------------------------

	const WINDOWS = $derived([
		{ value: 0, label: m.notifications_policy_send_now() },
		{ value: 30, label: m.alerts_span_s({ seconds: 30 }) },
		{ value: 60, label: m.alerts_span_min({ minutes: 1 }) },
		{ value: 120, label: m.alerts_span_min({ minutes: 2 }) },
		{ value: 300, label: m.alerts_span_min({ minutes: 5 }) }
	]);
	const MINUTES = $derived([
		{ value: 300, label: m.alerts_span_min({ minutes: 5 }) },
		{ value: 900, label: m.alerts_span_min({ minutes: 15 }) },
		{ value: 1800, label: m.alerts_span_min({ minutes: 30 }) },
		{ value: 3600, label: m.alerts_span_h({ hours: 1 }) }
	]);

	let batchWindow = $state(60);
	let maxPerHour = $state('20');
	let flapOn = $state(true);
	let flapEvents = $state('4');
	let flapWindow = $state(1800);
	let flapHold = $state(1800);
	let publicUrl = $state('');
	let escalateAfter = $state(0);
	let escalateChannel = $state('');
	let showMore = $state(false);

	/** Delays offered for the escalation hop. 0 switches it off. */
	const ESCALATIONS = $derived([
		{ value: 0, label: m.notifications_policy_escalate_never() },
		{ value: 300, label: m.notifications_policy_escalate_after({ span: m.alerts_span_min({ minutes: 5 }) }) },
		{ value: 900, label: m.notifications_policy_escalate_after({ span: m.alerts_span_min({ minutes: 15 }) }) },
		{ value: 1800, label: m.notifications_policy_escalate_after({ span: m.alerts_span_min({ minutes: 30 }) }) },
		{ value: 3600, label: m.notifications_policy_escalate_after({ span: m.alerts_span_h({ hours: 1 }) }) }
	]);

	/** The escalation, said in one sentence — or why it is doing nothing. */
	const escalationSentence = $derived.by(() => {
		if (escalateAfter === 0 || escalateChannel === '') {
			return m.notifications_policy_escalate_off();
		}
		const name =
			channels.find((c) => String(c.id) === escalateChannel)?.name ??
			m.notifications_policy_that_channel();
		const minutes = Math.round(escalateAfter / 60);
		return m.notifications_policy_escalate_on({ minutes, name });
	});

	let saving = $state(false);
	let saveError = $state<unknown>(null);
	let saved = $state(false);

	function withCurrent(options: { value: number; label: string }[], current: number) {
		if (options.some((o) => o.value === current)) return options;
		return [...options, { value: current, label: m.notifications_form_interval_current({ seconds: current }) }].sort((a, b) => a.value - b.value);
	}

	const dirty = $derived.by(() => {
		if (!policy) return false;
		return (
			batchWindow !== policy.batch_window_secs ||
			Number(maxPerHour || 0) !== policy.max_per_hour ||
			(flapOn ? Number(flapEvents) : 0) !== policy.flap_events ||
			flapWindow !== policy.flap_window_secs ||
			flapHold !== policy.flap_hold_secs ||
			publicUrl.trim() !== policy.public_url ||
			escalateAfter !== (policy.escalate_channel === null ? 0 : policy.escalate_after_secs) ||
			escalateChannel !== (policy.escalate_channel === null ? '' : String(policy.escalate_channel))
		);
	});

	let formError = $state<string | null>(null);

	async function save(event: SubmitEvent) {
		event.preventDefault();
		formError = null;
		saveError = null;
		saved = false;
		const cap = maxPerHour.trim() === '' ? 0 : Number(maxPerHour);
		if (!Number.isInteger(cap) || cap < 0) {
			formError = m.notifications_policy_error_cap();
			return;
		}
		const events = flapOn ? Number(flapEvents) : 0;
		if (flapOn && (!Number.isInteger(events) || events < 2)) {
			formError = m.notifications_policy_error_flap();
			return;
		}
		const url = publicUrl.trim();
		if (url && !/^https?:\/\//.test(url)) {
			formError = m.notifications_policy_error_url();
			return;
		}
		const escalating = escalateAfter > 0 && escalateChannel !== '';
		if (escalateAfter > 0 && escalateChannel === '') {
			formError = m.notifications_policy_error_escalate();
			return;
		}
		saving = true;
		try {
			policy = await updateNotificationPolicy({
				batch_window_secs: batchWindow,
				max_per_hour: cap,
				flap_events: events,
				flap_window_secs: flapWindow,
				flap_hold_secs: flapHold,
				public_url: url,
				escalate_after_secs: escalating ? escalateAfter : 0,
				escalate_channel: escalating ? Number(escalateChannel) : null
			});
			publicUrl = policy.public_url;
			escalateAfter = policy.escalate_channel === null ? 0 : policy.escalate_after_secs;
			escalateChannel = policy.escalate_channel === null ? '' : String(policy.escalate_channel);
			saved = true;
			setTimeout(() => (saved = false), 2500);
		} catch (cause) {
			saveError = cause;
		} finally {
			saving = false;
		}
	}

	function severityWord(severity: Channel['policy']['min_severity']): string {
		if (severity === 'critical') return m.notifications_channels_policy_warning_only();
		if (severity === 'warning') return m.notifications_channels_policy_advisory_up();
		return m.notifications_policy_everything();
	}

	function clock(minute: number): string {
		const pad = (n: number) => String(n).padStart(2, '0');
		return `${pad(Math.floor(minute / 60))}:${pad(minute % 60)}`;
	}

	/** Short weekday name in the UI language, 0 = Monday (2024-01-01). */
	function dayName(index: number): string {
		return new Intl.DateTimeFormat(getLocale(), { weekday: 'short', timeZone: 'UTC' }).format(
			new Date(Date.UTC(2024, 0, 1 + index))
		);
	}

	function channelSummary(channel: Channel): string {
		const p = channel.policy;
		const parts = [severityWord(p.min_severity)];
		if (!p.notify_resolved) parts.push(m.notifications_channels_policy_no_recoveries());
		if (p.min_interval_secs > 0) {
			parts.push(m.notifications_channels_policy_interval({ minutes: Math.round(p.min_interval_secs / 60) }));
		}
		if (p.quiet_hours) {
			const days =
				p.quiet_hours.days.length === 7
					? m.notifications_policy_daily()
					: p.quiet_hours.days.map((d) => dayName(d)).join(' ');
			parts.push(
				m.notifications_policy_quiet({
					days,
					start: clock(p.quiet_hours.start_minute),
					end: clock(p.quiet_hours.end_minute)
				})
			);
		}
		return parts.join(' · ');
	}
</script>

<Panel id="notifications-policy" title={m.notifications_policy_title()} description={m.notifications_policy_description()}>
	{#snippet aside()}
		{#if !auth.isAdmin}
			<Plate tone="ghost" label={auth.readOnlyLabel} />
		{/if}
	{/snippet}

	{#if error}
		<ErrorNotice {error} title={m.notifications_policy_error_load()} onretry={() => void load()} />
	{:else if loading || !policy}
		<Skeleton class="h-10 w-full" rows={3} />
	{:else}
		<form class="grid gap-5" onsubmit={save} aria-label={m.notifications_policy_title()}>
			<div class="grid gap-4 sm:grid-cols-2">
				<Field label={m.notifications_policy_group()} for="policy-window" help={m.notifications_policy_group_help()}>
					<select id="policy-window" class="input" bind:value={batchWindow} disabled={saving || !auth.isAdmin}>
						{#each withCurrent(WINDOWS, batchWindow) as option (option.value)}
							<option value={option.value}>{option.label}</option>
						{/each}
					</select>
				</Field>
				<Field label={m.notifications_policy_cap()} for="policy-cap" help={m.notifications_policy_cap_help()}>
					<input id="policy-cap" type="number" min="0" step="1" class="input tnum" bind:value={maxPerHour} placeholder={m.notifications_policy_cap_placeholder()} disabled={saving || !auth.isAdmin} />
				</Field>
			</div>

			<Field label={m.notifications_policy_url()} for="policy-url" help={m.notifications_policy_url_help()}>
				<input id="policy-url" type="url" class="input" bind:value={publicUrl} placeholder="https://monit.example.lan" autocomplete="off" disabled={saving || !auth.isAdmin} />
			</Field>

			<div class="grid gap-2">
				<div class="grid gap-4 sm:grid-cols-2">
					<Field label={m.notifications_policy_escalate_label()} for="policy-escalate-after" help={m.notifications_policy_escalate_help()}>
						<select id="policy-escalate-after" class="input" bind:value={escalateAfter} disabled={saving || !auth.isAdmin}>
							{#each withCurrent(ESCALATIONS, escalateAfter) as option (option.value)}
								<option value={option.value}>{option.label}</option>
							{/each}
						</select>
					</Field>
					<Field label={m.notifications_policy_also_tell()} for="policy-escalate-channel" help={m.notifications_policy_also_tell_help()}>
						<select id="policy-escalate-channel" class="input" bind:value={escalateChannel} disabled={saving || !auth.isAdmin || escalateAfter === 0}>
							<option value="">{m.notifications_policy_no_channel()}</option>
							{#each channels as channel (channel.id)}
								<option value={String(channel.id)}>{channel.name}</option>
							{/each}
						</select>
					</Field>
				</div>
				<p class="text-[0.8125rem] text-ink-2">{escalationSentence}</p>
			</div>

			<div class="grid gap-3">
				<button
					type="button"
					class="inline-flex min-h-10 w-fit flex-wrap items-center gap-x-1.5 text-left text-sm font-semibold text-ink sm:min-h-0"
					aria-expanded={showMore}
					aria-controls="policy-more"
					onclick={() => (showMore = !showMore)}
				>
					<SlidersHorizontal class="size-4" aria-hidden="true" />
					{showMore ? m.notifications_policy_fewer() : m.notifications_policy_more()}
					<span class="font-normal text-ink-2">{m.notifications_policy_flap_hint()}</span>
				</button>
				{#if showMore}
					<div id="policy-more" class="grid gap-4 rounded-lg border border-line bg-surface-2 p-4">
						<Field label={m.notifications_policy_flap()} for="policy-flap" inline help={m.notifications_policy_flap_help()}>
							<Toggle id="policy-flap" bind:checked={flapOn} disabled={saving || !auth.isAdmin} label={m.notifications_policy_flap()} />
						</Field>
						{#if flapOn}
							<div class="grid gap-4 sm:grid-cols-3">
								<Field label={m.notifications_policy_flap_changes()} for="policy-flap-events" help={m.notifications_policy_flap_changes_help()}>
									<input id="policy-flap-events" type="number" min="2" step="1" class="input tnum" bind:value={flapEvents} disabled={saving || !auth.isAdmin} />
								</Field>
								<Field label={m.notifications_policy_flap_within()} for="policy-flap-window">
									<select id="policy-flap-window" class="input" bind:value={flapWindow} disabled={saving || !auth.isAdmin}>
										{#each withCurrent(MINUTES, flapWindow) as option (option.value)}
											<option value={option.value}>{option.label}</option>
										{/each}
									</select>
								</Field>
								<Field label={m.notifications_policy_flap_hold()} for="policy-flap-hold">
									<select id="policy-flap-hold" class="input" bind:value={flapHold} disabled={saving || !auth.isAdmin}>
										{#each withCurrent(MINUTES, flapHold) as option (option.value)}
											<option value={option.value}>{option.label}</option>
										{/each}
									</select>
								</Field>
							</div>
						{/if}
					</div>
				{/if}
			</div>

			{#if formError}
				<p class="text-[0.8125rem] font-medium text-warning-ink" role="alert">{formError}</p>
			{/if}
			{#if saveError}
				<ErrorNotice error={saveError} title={m.notifications_policy_error_save()} />
			{/if}

			{#if auth.isAdmin}
				<div class="flex flex-wrap items-center gap-3" aria-live="polite">
					<ClickSpark>
						<Button type="submit" variant="primary" loading={saving} disabled={!dirty}>{m.alerts_rules_edit_save()}</Button>
					</ClickSpark>
					{#if saved}
						<Plate tone="signal" label={m.notifications_policy_saved()} draw />
					{/if}
				</div>
			{/if}
		</form>

		<div class="mt-6 border-t border-line pt-4">
			<h3 class="text-sm font-semibold text-ink">{m.notifications_policy_per_channel()}</h3>
			<p class="mt-0.5 text-[0.8125rem] text-ink-2">
				{m.notifications_policy_per_channel_note()}
			</p>
			{#if channels.length === 0}
				<p class="mt-2 text-sm text-ink-2">{m.notifications_channels_empty_title()}</p>
			{:else}
				<ul class="mt-2 grid gap-1.5" role="list">
					{#each channels as channel (channel.id)}
						<li class="text-sm">
							<div class="flex flex-wrap items-baseline gap-x-2">
								<span class="font-medium text-ink">{channel.name}</span>
								<span class="text-ink-2">{channelSummary(channel)}</span>
								{#if !channel.enabled}
									<Plate tone="ghost" label={m.notifications_channels_disabled()} bare />
								{/if}
							</div>
							{#if !matcherIsEmpty(channel.policy?.matcher)}
								<p class="text-[0.8125rem] text-ink-2">
									{matcherSentence(channel.policy.matcher, channel.policy.min_severity)}
								</p>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}
</Panel>
