<script lang="ts">
	/**
	 * Settings → Wall music → Speaker: is the wall really a Spotify speaker?
	 *
	 * Everything the phone's device list depends on, read from the server
	 * (`GET /api/music/speaker`): the connected account, Premium (when Spotify
	 * or a wall said so), whether Spotify lists the speaker among the
	 * account's devices right now, and what each wall reported in the last
	 * fifteen minutes — its browser, whether it is ready, and why not. A wall
	 * on a TV says it on its own screen too, but nobody reads a TV up close.
	 *
	 * "Test sound" moves the account's playback to the speaker (it resumes
	 * what played last); the name is shared by every wall (admins rename it).
	 */
	import { Play, RefreshCw } from 'lucide-svelte';
	import {
		getWallSpeaker,
		playOnSpeaker,
		setSpeakerName,
		type WallReport,
		type WallSpeaker
	} from '#lib/api/music.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, ErrorNotice, Field, Led, Plate, Skeleton } from '#lib/ui/index.js';
	import { formatRelative } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		/** Called with the new name after a rename, so the section can say it. */
		onrenamed?: (name: string) => void;
	}

	let { onrenamed }: Props = $props();

	let status = $state<WallSpeaker | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);
	let missing = $state(false);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			status = await getWallSpeaker(signal);
			if (!editing) draftName = status.speaker_name;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			if (cause && typeof cause === 'object' && 'missing' in cause && cause.missing) missing = true;
			else error = cause;
		} finally {
			loading = false;
		}
	}

	// Walls report every minute: refreshed every 20 s while the page is open.
	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => {
			if (document.visibilityState === 'visible') void load(controller.signal);
		}, 20_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	// --- Name -----------------------------------------------------------------

	let draftName = $state('');
	let editing = $state(false);
	let saving = $state(false);
	let nameError = $state<string | null>(null);

	async function rename(event: SubmitEvent) {
		event.preventDefault();
		const name = draftName.trim();
		if (name.length > 64) {
			nameError = m.settings_speaker_err_name();
			return;
		}
		saving = true;
		nameError = null;
		try {
			const saved = await setSpeakerName(name || null);
			draftName = saved.speaker_name;
			editing = false;
			if (status) status = { ...status, speaker_name: saved.speaker_name };
			onrenamed?.(saved.speaker_name);
		} catch (cause) {
			nameError = cause instanceof Error ? cause.message : m.settings_speaker_rename_error();
		} finally {
			saving = false;
		}
	}

	// --- Test sound -------------------------------------------------------------

	let testing = $state<string | null>(null);
	let testResult = $state<{ ok: boolean; message: string } | null>(null);

	async function test(deviceId: string | null, key: string) {
		testing = key;
		testResult = null;
		try {
			const played = await playOnSpeaker(deviceId);
			testResult = {
				ok: true,
				message: m.settings_speaker_test_ok({ name: played.speaker_name })
			};
			void load();
		} catch (cause) {
			testResult = { ok: false, message: cause instanceof Error ? cause.message : m.settings_speaker_test_fail() };
		} finally {
			testing = null;
		}
	}

	// --- Reading ----------------------------------------------------------------

	type Tone = 'signal' | 'advisory' | 'warning' | 'ghost';

	function wallLine(wall: WallReport): { tone: Tone; word: string } {
		switch (wall.phase) {
			case 'ready':
				if (wall.listed === false) return { tone: 'advisory', word: m.settings_speaker_phase_not_listed() };
				return wall.activated ? { tone: 'signal', word: m.settings_speaker_phase_ready() } : { tone: 'advisory', word: m.settings_speaker_phase_locked() };
			case 'starting':
				return { tone: 'ghost', word: m.settings_speaker_phase_connecting() };
			case 'unsupported':
				return { tone: 'warning', word: m.settings_speaker_phase_unsupported() };
			case 'error':
				return { tone: 'warning', word: m.settings_speaker_phase_error() };
			default:
				return { tone: 'ghost', word: m.settings_speaker_phase_off() };
		}
	}

	const premium = $derived.by((): { tone: Tone; word: string } => {
		if (status?.premium === true) return { tone: 'signal', word: m.settings_speaker_premium_yes() };
		if (status?.premium === false) return { tone: 'warning', word: m.settings_speaker_premium_no() };
		return { tone: 'ghost', word: m.settings_speaker_premium_unknown() };
	});
</script>

<section aria-labelledby="music-speaker" class="mt-5 rounded-[var(--radius-card)] border border-line p-4">
	<div class="flex flex-wrap items-center justify-between gap-2">
		<h4 id="music-speaker" class="text-sm font-semibold text-ink">{m.settings_speaker_heading()}</h4>
		<Button variant="ghost" size="sm" onclick={() => void load()} aria-label={m.settings_speaker_refresh_aria()}>
			<RefreshCw class="size-3.5" aria-hidden="true" />
			{m.settings_speaker_refresh()}
		</Button>
	</div>

	{#if missing}
		<p class="mt-2 text-sm text-ink-2">{m.settings_speaker_missing()}</p>
	{:else if error && !status}
		<ErrorNotice class="mt-2" {error} title={m.settings_speaker_read_error()} onretry={() => void load()} />
	{:else if loading || !status}
		<Skeleton class="mt-2 h-8 w-full" rows={3} />
	{:else}
		{#if auth.isAdmin}
			<form class="mt-2 flex flex-col gap-2 sm:flex-row sm:items-end" onsubmit={rename} novalidate>
				<div class="min-w-0 flex-1">
					<Field
						label={m.settings_speaker_name_label()}
						for="music-speaker-name"
						error={nameError}
						help={m.settings_speaker_name_help()}
					>
						<input
							id="music-speaker-name"
							class="input"
							bind:value={draftName}
							maxlength="64"
							autocomplete="off"
							disabled={saving}
							oninput={() => {
								editing = true;
								nameError = null;
							}}
						/>
					</Field>
				</div>
				<Button type="submit" variant="secondary" loading={saving} disabled={!editing}>{m.settings_speaker_rename()}</Button>
			</form>
		{:else}
			<p class="mt-2 text-sm text-ink">{m.settings_speaker_listed_as({ name: status.speaker_name })}</p>
		{/if}

		<dl class="mt-3 grid gap-x-4 gap-y-1.5 text-sm sm:grid-cols-[auto_minmax(0,1fr)]">
			<dt class="text-ink-2">{m.settings_speaker_account()}</dt>
			<dd class="text-ink">
				{status.account_name ?? m.settings_speaker_connected()}
				<span class="text-ink-2">{m.settings_speaker_account_note()}</span>
			</dd>
			<dt class="text-ink-2">{m.settings_speaker_premium()}</dt>
			<dd class="flex items-center gap-2 text-ink"><Led tone={premium.tone} size="sm" />{premium.word}</dd>
			<dt class="text-ink-2">{m.settings_speaker_in_devices()}</dt>
			<dd class="min-w-0 text-ink">
				{#if status.error}
					<span class="text-advisory-ink">{status.error}</span>
				{:else}
					<span class="flex items-center gap-2">
						<Led tone={status.listed ? 'signal' : 'advisory'} size="sm" />
						{status.listed ? m.settings_speaker_listed_yes() : m.settings_speaker_listed_no()}
					</span>
					{#if status.devices.length > 0}
						<span class="mt-0.5 block text-[0.8125rem] text-ink-2">
							{m.settings_speaker_lists({ devices: status.devices.map((d) => (d.is_active ? m.settings_speaker_device_playing({ name: d.name }) : d.name)).join(', ') })}
						</span>
					{:else}
						<span class="mt-0.5 block text-[0.8125rem] text-ink-2">{m.settings_speaker_lists_none()}</span>
					{/if}
				{/if}
			</dd>
		</dl>

		<h5 class="mt-4 text-[0.8125rem] font-semibold text-ink">{m.settings_speaker_walls()}</h5>
		{#if status.walls.length === 0}
			<p class="mt-1 text-[0.8125rem] text-ink-2">
				{m.settings_speaker_walls_none()}
			</p>
		{:else}
			<ul class="mt-1 grid gap-2">
				{#each status.walls as wall (wall.display)}
					{@const line = wallLine(wall)}
					<li class="grid gap-1 rounded-[var(--radius-card)] border border-line px-3 py-2 text-[0.8125rem]">
						<div class="flex flex-wrap items-center gap-2">
							<Plate tone={line.tone} label={line.word} />
							<span class="font-semibold text-ink">{wall.browser ?? m.settings_speaker_browser_default()}</span>
							<span class="text-ink-2">{m.settings_speaker_wall_meta({ name: wall.name, when: formatRelative(wall.seen_at) })}</span>
							{#if wall.phase === 'ready' && wall.device_id}
								<Button
									variant="ghost"
									size="sm"
									class="ml-auto"
									loading={testing === wall.display}
									onclick={() => void test(wall.device_id, wall.display)}
								>
									<Play class="size-3.5" aria-hidden="true" />
									{m.settings_speaker_test_sound()}
								</Button>
							{/if}
						</div>
						{#if wall.problem}
							<p class="text-ink">{wall.problem}</p>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}

		<div class="mt-4 flex flex-wrap items-center gap-2">
			<Button variant="secondary" size="md" loading={testing === 'speaker'} onclick={() => void test(null, 'speaker')}>
				<Play class="size-4" aria-hidden="true" />
				{m.settings_speaker_test_sound()}
			</Button>
			<span class="text-[0.8125rem] text-ink-2">{m.settings_speaker_test_hint({ name: status.speaker_name })}</span>
		</div>
		{#if testResult}
			<p class="mt-2 text-[0.8125rem] font-medium {testResult.ok ? 'text-ink' : 'text-warning-ink'}" role="status">{testResult.message}</p>
		{/if}
	{/if}
</section>
