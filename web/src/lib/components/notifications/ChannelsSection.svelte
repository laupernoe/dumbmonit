<script lang="ts">
	/**
	 * Alerts → Notifications: the channel list, and the add/edit form that
	 * opens above it. The kind catalogue and the list load together: without
	 * the catalogue we could neither name a kind nor build the form.
	 */
	import { BellRing, Plus } from 'lucide-svelte';
	import { deleteChannel, listChannelKinds, listChannels, testChannel, type Channel } from '#lib/api/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, ClickSpark, Confirm, EmptyState, ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import { kindIcon, normalizeKind, type KindInfo } from './kinds';
	import ChannelForm from './ChannelForm.svelte';
	import { m } from '#lib/paraglide/messages.js';

	let kinds = $state<KindInfo[]>([]);
	let channels = $state<Channel[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			const [catalogue, list] = await Promise.all([listChannelKinds(signal), listChannels(signal)]);
			kinds = (Array.isArray(catalogue) ? catalogue : [])
				.filter((k) => k && typeof k.kind === 'string')
				.map(normalizeKind);
			channels = Array.isArray(list) ? list : [];
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

	/** "Last sent <time>" cut around the time, which stays a <time> element. */
	const lastSentParts = $derived(m.notifications_channels_last_sent({ when: '\u0000' }).split('\u0000'));

	function kindLabel(kind: string): string {
		return kinds.find((k) => k.kind === kind)?.label ?? kind;
	}

	/** One line on what the channel hears; empty when it hears everything, always. */
	function policySummary(channel: Channel): string {
		const p = channel.policy;
		if (!p) return '';
		const parts: string[] = [];
		if (p.min_severity === 'critical') parts.push(m.notifications_channels_policy_warning_only());
		else if (p.min_severity === 'warning') parts.push(m.notifications_channels_policy_advisory_up());
		if (!p.notify_resolved) parts.push(m.notifications_channels_policy_no_recoveries());
		if (p.min_interval_secs > 0) {
			parts.push(m.notifications_channels_policy_interval({ minutes: Math.round(p.min_interval_secs / 60) }));
		}
		if (p.quiet_hours) {
			const pad = (n: number) => String(n).padStart(2, '0');
			const clock = (min: number) => `${pad(Math.floor(min / 60))}:${pad(min % 60)}`;
			parts.push(
				m.notifications_channels_policy_quiet({
					start: clock(p.quiet_hours.start_minute),
					end: clock(p.quiet_hours.end_minute)
				})
			);
		}
		return parts.join(' · ');
	}

	// --- Form -----------------------------------------------------------------

	/** `null`: closed; `'new'`: adding; a channel: editing it. */
	let form = $state<null | 'new' | Channel>(null);
	let highlighted = $state<number | null>(null);

	function afterSave(saved: Channel) {
		const index = channels.findIndex((c) => c.id === saved.id);
		if (index === -1) channels = [...channels, saved];
		else channels[index] = saved;
		form = null;
		// A brief glow on the row that just changed, so the eye lands on it.
		highlighted = saved.id;
		setTimeout(() => (highlighted = null), 1800);
	}

	// --- Per-channel actions --------------------------------------------------

	let busy = $state<{ id: number; action: 'test' | 'delete' } | null>(null);
	let actionError = $state<{ id: number; cause: unknown } | null>(null);
	let testResult = $state<{ id: number; ok: boolean; message: string } | null>(null);

	async function sendTest(channel: Channel) {
		busy = { id: channel.id, action: 'test' };
		actionError = null;
		testResult = null;
		try {
			const report = await testChannel(channel.id);
			testResult = { id: channel.id, ok: report.ok, message: report.message };
			// The test updates "last sent" and "last error" on the server.
			channels = await listChannels();
		} catch (cause) {
			actionError = { id: channel.id, cause };
		} finally {
			busy = null;
		}
	}

	async function remove(channel: Channel) {
		busy = { id: channel.id, action: 'delete' };
		actionError = null;
		try {
			await deleteChannel(channel.id);
			channels = channels.filter((c) => c.id !== channel.id);
			if (form !== null && form !== 'new' && form.id === channel.id) form = null;
		} catch (cause) {
			actionError = { id: channel.id, cause };
		} finally {
			busy = null;
		}
	}
</script>

<Panel id="notifications-channels" title={m.notifications_channels_title()} description={m.notifications_channels_description()} padded={false}>
	{#snippet aside()}
		{#if !auth.isAdmin}
			<Plate tone="ghost" label={auth.readOnlyLabel} />
		{:else if !loading && !error && form === null && channels.length > 0}
			<ClickSpark>
				<Button variant="primary" size="sm" onclick={() => (form = 'new')}>
					<Plus class="size-4" aria-hidden="true" />
					{m.notifications_channels_add()}
				</Button>
			</ClickSpark>
		{/if}
	{/snippet}

	<div class="px-5 py-4">
		{#if error}
			<ErrorNotice {error} title={m.notifications_channels_error_load()} onretry={() => void load()} />
		{:else if loading}
			<div class="grid gap-2">
				<Skeleton class="h-16 w-full" rows={2} />
			</div>
		{:else}
			{#if form !== null}
				<div class="mb-4">
					<!-- The key forces a fresh form on every opening: no leftover input. -->
					{#key form}
						<ChannelForm {kinds} channel={form === 'new' ? undefined : form} onsaved={afterSave} oncancel={() => (form = null)} />
					{/key}
				</div>
			{/if}

			{#if channels.length === 0}
				{#if !auth.isAdmin}
					<EmptyState icon={BellRing} title={m.notifications_channels_empty_title()} description={m.notifications_channels_empty_viewer()} />
				{:else if form === null}
					<EmptyState
						icon={BellRing}
						title={m.notifications_channels_empty_title()}
						description={m.notifications_channels_empty_admin()}
					>
						{#snippet action()}
							<ClickSpark>
								<Button variant="primary" onclick={() => (form = 'new')}>
									<Plus class="size-4" aria-hidden="true" />
									{m.notifications_channels_add()}
								</Button>
							</ClickSpark>
						{/snippet}
					</EmptyState>
				{/if}
			{:else}
				<ul class="grid gap-2" role="list">
					{#each channels as channel, i (channel.id)}
						{@const Icon = kindIcon(channel.kind)}
						{@const current = busy?.id === channel.id ? busy.action : null}
						<li
							class={`rise-in rounded-[var(--radius-card)] border px-4 py-3 transition-[background-color,border-color,box-shadow] duration-700 ${highlighted === channel.id ? 'border-signal bg-signal-soft shadow-lift' : 'border-line bg-surface'} ${channel.enabled ? '' : 'ghost-cell'}`}
							style="--rise-delay: {Math.min(i, 8) * 30}ms"
						>
							<div class="flex flex-wrap items-start gap-3">
								<span class={`mt-0.5 flex size-9 shrink-0 items-center justify-center rounded-lg ${channel.enabled ? 'bg-signal-soft text-signal-ink' : 'bg-ghost text-ink-3'}`}>
									<Icon class="size-4" aria-hidden="true" />
								</span>
								<!-- A basis of 14rem: on a phone the actions wrap under the text instead of squeezing it. -->
								<div class="min-w-0 flex-[1_1_14rem]">
									<div class="flex flex-wrap items-center gap-x-2 gap-y-1">
										<span class="truncate font-semibold text-ink">{channel.name}</span>
										<span class="text-sm text-ink-2">{kindLabel(channel.kind)}</span>
										{#if channel.enabled}
											<Plate tone="signal" label={m.notifications_channels_enabled()} />
										{:else}
											<Plate tone="ghost" label={m.notifications_channels_disabled()} />
										{/if}
									</div>
									<p class="mt-1 text-sm text-ink-2">
										{lastSentParts[0]}<time class="tnum" title={formatDateTime(channel.last_sent_at)}>{formatRelative(channel.last_sent_at)}</time>{lastSentParts[1]}
									</p>
									{#if policySummary(channel)}
										<p class="mt-0.5 text-[0.8125rem] text-ink-2">{policySummary(channel)}</p>
									{/if}
									{#if channel.last_error}
										<p class="mt-1 text-sm break-words text-warning-ink">{m.notifications_channels_last_error({ error: channel.last_error })}</p>
									{/if}
								</div>
								{#if auth.isAdmin}
									<div class="flex flex-wrap items-center gap-1.5">
										<Button variant="secondary" size="sm" loading={current === 'test'} disabled={current !== null} onclick={() => void sendTest(channel)}>
											{m.notifications_channels_send_test()}
										</Button>
										<Button variant="ghost" size="sm" disabled={current !== null} onclick={() => (form = channel)}>{m.notifications_channels_edit()}</Button>
										<Confirm confirmLabel={m.notifications_channels_remove_confirm()} loading={current === 'delete'} disabled={current !== null} onconfirm={() => remove(channel)}>
											{m.notifications_channels_remove()}
										</Confirm>
									</div>
								{/if}
							</div>

							<div aria-live="polite">
								{#if testResult?.id === channel.id}
									<div class="mt-3 flex flex-wrap items-center gap-2 text-sm">
										<Plate tone={testResult.ok ? 'signal' : 'warning'} label={testResult.ok ? m.notifications_channels_test_ok() : m.notifications_channels_test_failed()} />
										<span class="min-w-0 break-words text-ink-2">{testResult.message}</span>
									</div>
								{/if}
								{#if actionError?.id === channel.id}
									<ErrorNotice error={actionError.cause} title={m.notifications_channels_error_action()} class="mt-3" />
								{/if}
							</div>
						</li>
					{/each}
				</ul>
			{/if}
		{/if}
	</div>
</Panel>
