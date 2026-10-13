<script lang="ts">
	/**
	 * Cats run the server room: naps on warm racks, a box, a dangling cable.
	 * `mood` warn: a cat sits on the keyboard (gibberish on screen); critical:
	 * puffed tails, a chewed cable spitting sparks, a storm outside.
	 * `pulse` replays one gesture: down = a cat slowly pushes a server off its
	 * shelf, staring at you (clonk); up = it puts it back, very proud, ears up;
	 * resolved = every cat curls up in a box and naps.
	 */
	import CatsRoom from '#lib/components/wall/cats/CatsRoom.svelte';

	interface Props {
		mood?: 'ok' | 'warn' | 'critical';
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
	}
	let { mood = 'ok', pulse = null }: Props = $props();
</script>

<CatsRoom {mood} {pulse} fade />
