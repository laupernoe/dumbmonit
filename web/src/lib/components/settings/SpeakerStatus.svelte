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
			nameError = 'A speaker name is at most 64 characters.';
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
			nameError = cause instanceof Error ? cause.message : 'Could not rename the speaker.';
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
				message: `Spotify now plays on “${played.speaker_name}”: listen to the wall. Nothing to hear? Tap the wall once to unlock its sound.`
			};
			void load();
		} catch (cause) {
			testResult = { ok: false, message: cause instanceof Error ? cause.message : 'Spotify did not start playing.' };
		} finally {
			testing = null;
		}
	}

	// --- Reading ----------------------------------------------------------------

	type Tone = 'signal' | 'advisory' | 'warning' | 'ghost';

	function wallLine(wall: WallReport): { tone: Tone; word: string } {
		switch (wall.phase) {
			case 'ready':
				if (wall.listed === false) return { tone: 'advisory', word: 'Not listed by Spotify' };
				return wall.activated ? { tone: 'signal', word: 'Ready' } : { tone: 'advisory', word: 'Ready, sound locked' };
			case 'starting':
				return { tone: 'ghost', word: 'Connecting' };
			case 'unsupported':
				return { tone: 'warning', word: 'Cannot play here' };
			case 'error':
				return { tone: 'warning', word: 'Stopped' };
			default:
				return { tone: 'ghost', word: 'Off on this display' };
		}
	}

	const premium = $derived.by((): { tone: Tone; word: string } => {
		if (status?.premium === true) return { tone: 'signal', word: 'Yes' };
		if (status?.premium === false) return { tone: 'warning', word: 'No — the wall cannot be a speaker' };
		return { tone: 'ghost', word: 'Not known yet (a wall tells when it connects)' };
	});
</script>

<section aria-labelledby="music-speaker" class="mt-5 rounded-[var(--radius-card)] border border-line p-4">
	<div class="flex flex-wrap items-center justify-between gap-2">
		<h4 id="music-speaker" class="text-sm font-semibold text-ink">Speaker</h4>
		<Button variant="ghost" size="sm" onclick={() => void load()} aria-label="Refresh the speaker status">
			<RefreshCw class="size-3.5" aria-hidden="true" />
			Refresh
		</Button>
	</div>

	{#if missing}
		<p class="mt-2 text-sm text-ink-2">This server does not report the speaker yet: update DumbMonit.</p>
	{:else if error && !status}
		<ErrorNotice class="mt-2" {error} title="Could not read the speaker" onretry={() => void load()} />
	{:else if loading || !status}
		<Skeleton class="mt-2 h-8 w-full" rows={3} />
	{:else}
		{#if auth.isAdmin}
			<form class="mt-2 flex flex-col gap-2 sm:flex-row sm:items-end" onsubmit={rename} novalidate>
				<div class="min-w-0 flex-1">
					<Field
						label="Name in Spotify"
						for="music-speaker-name"
						error={nameError}
						help="What phones list under Devices. Every wall uses it; add ?speaker=Kitchen to one wall's address to name that one alone. Empty: “DumbMonit Wall”."
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
				<Button type="submit" variant="secondary" loading={saving} disabled={!editing}>Rename</Button>
			</form>
		{:else}
			<p class="mt-2 text-sm text-ink">Listed in Spotify as <strong class="font-semibold">{status.speaker_name}</strong>.</p>
		{/if}

		<dl class="mt-3 grid gap-x-4 gap-y-1.5 text-sm sm:grid-cols-[auto_minmax(0,1fr)]">
			<dt class="text-ink-2">Account</dt>
			<dd class="text-ink">
				{status.account_name ?? 'Connected'}
				<span class="text-ink-2">— the phone must be signed in to this Spotify account to see the wall.</span>
			</dd>
			<dt class="text-ink-2">Premium</dt>
			<dd class="flex items-center gap-2 text-ink"><Led tone={premium.tone} size="sm" />{premium.word}</dd>
			<dt class="text-ink-2">In Spotify’s devices</dt>
			<dd class="min-w-0 text-ink">
				{#if status.error}
					<span class="text-advisory-ink">{status.error}</span>
				{:else}
					<span class="flex items-center gap-2">
						<Led tone={status.listed ? 'signal' : 'advisory'} size="sm" />
						{status.listed ? 'Yes — phones on this account can pick it' : 'Not right now'}
					</span>
					{#if status.devices.length > 0}
						<span class="mt-0.5 block text-[0.8125rem] text-ink-2">
							Spotify lists: {status.devices.map((d) => `${d.name}${d.is_active ? ' (playing)' : ''}`).join(', ')}
						</span>
					{:else}
						<span class="mt-0.5 block text-[0.8125rem] text-ink-2">Spotify lists no device on this account right now.</span>
					{/if}
				{/if}
			</dd>
		</dl>

		<h5 class="mt-4 text-[0.8125rem] font-semibold text-ink">Walls</h5>
		{#if status.walls.length === 0}
			<p class="mt-1 text-[0.8125rem] text-ink-2">
				No wall has reported in the last fifteen minutes. Open <a class="font-semibold text-ink hover:underline" href="/wall">/wall</a> on the
				display (signed in, viewer accounts included) and it shows up here within a minute.
			</p>
		{:else}
			<ul class="mt-1 grid gap-2">
				{#each status.walls as wall (wall.display)}
					{@const line = wallLine(wall)}
					<li class="grid gap-1 rounded-[var(--radius-card)] border border-line px-3 py-2 text-[0.8125rem]">
						<div class="flex flex-wrap items-center gap-2">
							<Plate tone={line.tone} label={line.word} />
							<span class="font-semibold text-ink">{wall.browser ?? 'A browser'}</span>
							<span class="text-ink-2">· as “{wall.name}” · {formatRelative(wall.seen_at)}</span>
							{#if wall.phase === 'ready' && wall.device_id}
								<Button
									variant="ghost"
									size="sm"
									class="ml-auto"
									loading={testing === wall.display}
									onclick={() => void test(wall.device_id, wall.display)}
								>
									<Play class="size-3.5" aria-hidden="true" />
									Test sound
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
				Test sound
			</Button>
			<span class="text-[0.8125rem] text-ink-2">Plays the account on “{status.speaker_name}”, resuming what played last.</span>
		</div>
		{#if testResult}
			<p class="mt-2 text-[0.8125rem] font-medium {testResult.ok ? 'text-ink' : 'text-warning-ink'}" role="status">{testResult.message}</p>
		{/if}
	{/if}
</section>
