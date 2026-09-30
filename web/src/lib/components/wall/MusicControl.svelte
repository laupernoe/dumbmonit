<script lang="ts">
	/**
	 * "Music" on the wall: a quiet button next to Exit that opens a small panel
	 * where a Spotify, Deezer or YouTube link is pasted. The link is checked
	 * here (`parseMusicLink`) before it is saved; a link that is refused says
	 * why and never reaches a frame.
	 */
	import { tick } from 'svelte';
	import { Music2 } from 'lucide-svelte';
	import { Button } from '$lib/ui';
	import { parseMusicLink, type MusicEmbed } from '$lib/wall/music';

	interface Props {
		/** The link saved for this display, or null. */
		link: string | null;
		embed: MusicEmbed | null;
		open: boolean;
		onsave: (link: string) => void;
		onclear: () => void;
	}

	let { link, embed, open = $bindable(), onsave, onclear }: Props = $props();

	let draft = $state('');
	let error = $state<string | null>(null);
	let input = $state<HTMLInputElement | null>(null);
	let root = $state<HTMLDivElement | null>(null);

	async function toggle() {
		open = !open;
		if (open) {
			draft = link ?? '';
			error = null;
			await tick();
			input?.focus();
			input?.select();
		}
	}

	function submit(event: SubmitEvent) {
		event.preventDefault();
		const parsed = parseMusicLink(draft);
		if (!parsed.ok) {
			error = parsed.message;
			input?.focus();
			return;
		}
		onsave(draft.trim());
		open = false;
	}

	function clear() {
		onclear();
		draft = '';
		open = false;
	}

	// A click anywhere else closes the panel (Escape is handled by the wall).
	$effect(() => {
		if (!open) return;
		const onPointer = (event: PointerEvent) => {
			if (root && !root.contains(event.target as Node)) open = false;
		};
		document.addEventListener('pointerdown', onPointer);
		return () => document.removeEventListener('pointerdown', onPointer);
	});
</script>

<!-- Not positioned: the panel hangs from the wall's tool pill, so it never runs off a phone. -->
<div bind:this={root}>
	<Button
		variant="ghost"
		size="sm"
		onclick={toggle}
		aria-expanded={open}
		aria-controls="wall-music-panel"
		class="music-toggle {open ? 'is-open' : ''}"
	>
		<Music2 class="size-4" aria-hidden="true" />
		Music
		{#if embed}<span class="sr-only">: {embed.label} {embed.kind}</span>{/if}
	</Button>

	{#if open}
		<div
			id="wall-music-panel"
			class="absolute top-full right-0 z-20 mt-2 w-[min(calc(100vw-2rem),24rem)] rounded-[var(--radius-card)] border border-line bg-surface p-4 text-left shadow-float"
			role="dialog"
			aria-label="Music on this display"
		>
			<form onsubmit={submit} novalidate>
				<label for="wall-music-link" class="block text-sm font-semibold text-ink">Music on this display</label>
				<input
					bind:this={input}
					bind:value={draft}
					id="wall-music-link"
					class="input mt-1.5"
					type="url"
					inputmode="url"
					autocomplete="off"
					spellcheck="false"
					placeholder="https://open.spotify.com/playlist/…"
					aria-invalid={error ? 'true' : undefined}
					aria-describedby="wall-music-help"
					oninput={() => (error = null)}
				/>
				{#if error}
					<p id="wall-music-help" class="mt-1.5 text-[0.8125rem] font-medium text-warning-ink" role="alert">{error}</p>
				{:else}
					<p id="wall-music-help" class="mt-1.5 text-[0.8125rem] text-ink-2">
						A Spotify, Deezer or YouTube (Music) link to a track, album, playlist or video.
						Saved in this browser only; nothing loads from them until a link is set.
					</p>
				{/if}
				<div class="mt-3 flex items-center justify-end gap-2">
					{#if link}
						<Button variant="ghost" size="sm" class="mr-auto" onclick={clear}>Remove</Button>
					{/if}
					<Button variant="ghost" size="sm" onclick={() => (open = false)}>Cancel</Button>
					<Button variant="primary" size="sm" type="submit">Play on the wall</Button>
				</div>
			</form>
		</div>
	{/if}
</div>
