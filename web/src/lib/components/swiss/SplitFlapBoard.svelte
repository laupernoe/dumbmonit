<script lang="ts">
	/**
	 * Station departure board with split-flap letters, as an SVG group to drop
	 * inside a scene's <svg>. Two lines: how the network runs (on time, delayed,
	 * disrupted — words, not only a colour) and the detail: the last event of
	 * `pulse` for a minute after it happens, else the number of open alerts when
	 * the scene knows it (`problems`), else "no alerts". Letters flip through the
	 * alphabet when the text changes. Reduced motion: letters just change.
	 * Drawn from (x, y), `w` scene units wide.
	 */
	import { untrack } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { reducedMotion } from '#lib/ui/index.js';

	type Kind = 'down' | 'up' | 'resolved';
	interface Props {
		x: number;
		y: number;
		w?: number;
		mood?: 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';
		/** Open alerts, when the scene knows the count (the wall does). */
		problems?: number | null;
		pulse?: { kind: Kind; at: number } | null;
		paused?: boolean;
		/** Scene y of the ground: two posts reach down to it. */
		ground?: number | null;
	}
	let {
		x,
		y,
		w = 200,
		mood = 'ok',
		problems = null,
		pulse = null,
		paused = false,
		ground = null
	}: Props = $props();

	const motion = !reducedMotion();
	const level = $derived(
		mood === 'warn' || mood === 'clouded' ? 1 : mood === 'critical' || mood === 'storm' ? 2 : 0
	);

	// The pulse is news for a minute; the one already present at mount is old news.
	let news = $state<Kind | null>(null);
	let seenAt: number | null = untrack(() => pulse?.at ?? null);
	$effect(() => {
		const p = pulse;
		if (!p || p.at === seenAt) return;
		seenAt = p.at;
		news = p.kind;
		const t = setTimeout(() => (news = null), 60_000);
		return () => clearTimeout(t);
	});

	const upper = (s: string) => s.toLocaleUpperCase(getLocale());
	const line1 = $derived(
		upper(level === 2 ? m.scene_swiss_board_critical() : level === 1 ? m.scene_swiss_board_warn() : m.scene_swiss_board_ok())
	);
	const line2 = $derived.by(() => {
		if (news) {
			return upper(
				news === 'down'
					? m.scene_swiss_board_down()
					: news === 'up'
						? m.scene_swiss_board_up()
						: m.scene_swiss_board_resolved()
			);
		}
		if (problems != null)
			return upper(
				problems > 0 ? m.scene_swiss_board_alerts({ count: problems }) : m.scene_swiss_board_none()
			);
		return level === 0 ? upper(m.scene_swiss_board_none()) : '';
	});

	const cols = $derived(Math.min(18, Math.max(12, [...line1].length, [...line2].length)));
	const cw = $derived(w / cols);
	const ch = $derived(cw * 1.55);
	const pad = $derived(cw * 0.35);
	const height = $derived(ch * 2 + pad * 3 + cw * 0.12);

	const pad2 = (s: string, n: number) => {
		const cs = [...s].slice(0, n);
		while (cs.length < n) cs.push(' ');
		return cs;
	};
	const targets = $derived([pad2(line1, cols), pad2(line2, cols)]);

	const FLIPS = ' ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
	let shown = $state<string[][]>([[], []]);
	$effect(() => {
		const goal = targets;
		if (!motion || paused) {
			shown = goal.map((r) => [...r]);
			return;
		}
		// Each letter flips a few times through the alphabet before it settles.
		const before = untrack(() => shown);
		const left = goal.map((row, ri) =>
			row.map((c, ci) => (before[ri]?.[ci] === c ? 0 : 3 + ((ci * 7 + ri * 3) % 5)))
		);
		const cur = goal.map((row, ri) => row.map((c, ci) => before[ri]?.[ci] ?? ' '));
		shown = cur.map((r) => [...r]);
		const timer = setInterval(() => {
			let busy = false;
			const next = shown.map((r) => [...r]);
			for (let ri = 0; ri < goal.length; ri++)
				for (let ci = 0; ci < goal[ri].length; ci++) {
					if (left[ri][ci] <= 0) continue;
					left[ri][ci]--;
					if (left[ri][ci] === 0) next[ri][ci] = goal[ri][ci];
					else {
						const at = FLIPS.indexOf(next[ri][ci]);
						next[ri][ci] = FLIPS[(at + 1 + ((ci + ri) % 5)) % FLIPS.length];
						busy = true;
					}
				}
			shown = next;
			if (!busy) clearInterval(timer);
		}, 85);
		return () => clearInterval(timer);
	});

	const ink = $derived(level === 2 ? '#ff6b5e' : level === 1 ? '#ffb23d' : '#ffd84d');
</script>

<g class="board" transform="translate({x} {y})" style:--ink={ink}>
	{#if ground != null}
		<g fill="#4a4f58">
			<rect x={w * 0.14} y={height} width={cw * 0.35} height={Math.max(0, ground - y - height)} />
			<rect x={w * 0.84} y={height} width={cw * 0.35} height={Math.max(0, ground - y - height)} />
		</g>
	{/if}
	<rect x={-cw * 0.3} y={-cw * 0.3} width={w + cw * 0.6} height={height + cw * 0.6} rx={cw * 0.35} fill="#1b1d22" stroke="#6b717c" stroke-width={cw * 0.12} />
	<rect x={-cw * 0.3} y={-cw * 0.3} width={cw * 0.28} height={height + cw * 0.6} fill="#eb0000" />
	{#each [0, 1] as ri (ri)}
		{#each targets[ri] as _, ci (ci)}
			{@const ch1 = shown[ri]?.[ci] ?? ' '}
			<g transform="translate({ci * cw + cw * 0.04} {pad + ri * (ch + pad)})">
				<rect width={cw * 0.92} height={ch} rx={cw * 0.1} fill="#0c0d10" />
				{#key ch1}
					<g class="flap" class:flip={motion}>
						<text
							x={cw * 0.46}
							y={ch * 0.54}
							text-anchor="middle"
							dominant-baseline="central"
							font-size={ch * 0.7}
							font-weight="700"
							font-family="Helvetica, Arial, sans-serif"
							fill="var(--ink)"
						>{ch1 === ' ' ? '' : ch1}</text>
					</g>
				{/key}
				<rect y={ch / 2 - 0.4} width={cw * 0.92} height="0.9" fill="#000" opacity=".7" />
			</g>
		{/each}
	{/each}
</g>

<style>
	.flap {
		transform-box: fill-box;
		transform-origin: center;
	}
	@media (prefers-reduced-motion: no-preference) {
		.flip {
			animation: flap-flip 0.14s ease-out both;
		}
	}
	@keyframes flap-flip {
		0% { transform: scaleY(0.05); }
		100% { transform: scaleY(1); }
	}
</style>
