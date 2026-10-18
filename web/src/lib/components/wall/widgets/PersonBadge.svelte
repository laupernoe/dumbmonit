<script lang="ts">
	/**
	 * A small round badge with a person's initials. Its colour comes from the
	 * name (stable everywhere, dark enough for white letters at every hue), but
	 * it never carries the information alone: the name is always written next
	 * to it, so the badge is hidden from screen readers. No name (usage the
	 * provider attributes to nobody): a neutral badge with a question mark.
	 */
	import { hueOf, initialsOf } from '#lib/wall/widgets.js';

	interface Props {
		name: string | null;
	}

	let { name }: Props = $props();

	const initials = $derived(name ? initialsOf(name) : '?');
	const hue = $derived(name ? hueOf(name) : null);
</script>

<span class="badge" class:badge--none={hue === null} style:--hue={hue ?? undefined} aria-hidden="true">{initials}</span>

<style>
	.badge {
		display: inline-grid;
		place-items: center;
		flex-shrink: 0;
		width: 2.1em;
		aspect-ratio: 1;
		border-radius: 50%;
		background: hsl(var(--hue) 52% 30%);
		box-shadow: inset 0 0 0 1.5px hsl(var(--hue) 60% 62% / 0.7);
		color: #fff;
		font-size: 0.62em;
		font-weight: 750;
		letter-spacing: 0.02em;
		line-height: 1;
	}
	.badge--none {
		background: var(--c-surface-2);
		box-shadow: inset 0 0 0 1.5px var(--c-line-strong);
		color: var(--c-ink-2);
	}
</style>
