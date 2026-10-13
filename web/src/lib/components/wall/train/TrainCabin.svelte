<script lang="ts">
	/**
	 * The inside of a Glacier Express panoramic carriage, as a fragment of an
	 * SVG of W x H units: curved panoramic windows onto the scrolling landscape
	 * (TrainLandscape), cream pillars with reading lights, a tablecloth with a
	 * cup of tea, a table lamp that takes the status colour (`--scene-tone`),
	 * the DumbMonit pigeon and, on the wall, a little count card.
	 *
	 * Gestures (`pulse`): down, the train brakes, everything on the table
	 * lurches forward and the tea sloshes; up, it pulls away and the table
	 * leans back; resolved, a tunnel exit floods the windows with light and
	 * the lamp flares. `startle` is a small jolt. Colours come from the parent.
	 */
	import { untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import TrainLandscape from './TrainLandscape.svelte';
	import { moodLevel, windowPath, type Pulse } from './geometry.js';

	interface Props {
		W: number;
		H: number;
		windows?: number;
		mood?: string;
		pulse?: Pulse;
		startle?: number;
		paused?: boolean;
		/** In this fragment's coordinates. */
		sun?: { x: number; y: number } | null;
		moon?: { x: number; y: number } | null;
		/** Wall only: the number on the card. */
		problems?: number | null;
		idp?: string;
	}

	let {
		W,
		H,
		windows = 3,
		mood = 'calm',
		pulse = null,
		startle = 0,
		paused = false,
		sun = null,
		moon = null,
		problems = null,
		idp = 'tr'
	}: Props = $props();

	const motion = !reducedMotion();
	const level = $derived(moodLevel(mood));

	// --- Layout ------------------------------------------------------------------------

	const u = $derived(H / 540);
	const top = $derived(H * 0.056);
	const sill = $derived(H * 0.74);
	const pillar = $derived(W * 0.04);
	const winW = $derived((W - (windows + 1) * pillar) / windows);
	const rt = $derived(Math.min(winW * 0.3, (sill - top) * 0.3));
	const openings = $derived(
		Array.from({ length: windows }, (_, i) => windowPath(pillar + i * (winW + pillar), top, winW, sill - top, rt, 8 * u))
	);
	const cabin = $derived(`M0 0H${W}V${H}H0Z${openings.join('')}`);
	const pillarsX = $derived(Array.from({ length: windows + 1 }, (_, i) => i * (winW + pillar) + pillar / 2));
	const baseY = $derived(sill + (H - sill) * 0.52);
	const mascotK = $derived(0.17 * u);

	// --- Table reactions ------------------------------------------------------------------

	let anim = $state('none');
	let animKey = $state(0);
	let seenPulse: number | null | undefined = untrack(() => pulse?.at);
	let seenStartle = untrack(() => startle);
	$effect(() => {
		const p = pulse;
		if (!p || p.at === seenPulse) return;
		seenPulse = p.at;
		anim = p.kind;
		animKey += 1;
	});
	$effect(() => {
		const s = startle;
		if (s === seenStartle) return;
		seenStartle = s;
		anim = 'jolt';
		animKey += 1;
	});
</script>

<defs>
	<clipPath id="{idp}-glass"><path d={openings.join('')} /></clipPath>
	<linearGradient id="{idp}-cloth" x1="0" y1="0" x2="0" y2="1">
		<stop offset="0" style="stop-color:var(--tr-cloth)" />
		<stop offset="1" style="stop-color:var(--tr-cloth-2)" />
	</linearGradient>
	<radialGradient id="{idp}-lamp">
		<stop offset="0" style="stop-color:var(--scene-tone, #e8a33a)" stop-opacity="0.9" />
		<stop offset="1" style="stop-color:var(--scene-tone, #e8a33a)" stop-opacity="0" />
	</radialGradient>
	<radialGradient id="{idp}-read">
		<stop offset="0" stop-color="#ffd98a" stop-opacity="0.8" />
		<stop offset="1" stop-color="#ffd98a" stop-opacity="0" />
	</radialGradient>
</defs>

<TrainLandscape
	x={0}
	y={top}
	{W}
	H={sill - top}
	{level}
	{pulse}
	{paused}
	sun={sun ? { x: sun.x, y: sun.y - top } : null}
	moon={moon ? { x: moon.x, y: moon.y - top } : null}
	{idp}
/>

<!-- Walls and pillars: everything but the glass. -->
<path d={cabin} fill="var(--tr-int)" fill-rule="evenodd" />
<rect width={W} height={top} fill="var(--tr-int-2)" />
<rect y={top - 2 * u} width={W} height={2 * u} fill="var(--tr-frame)" opacity="0.5" />
{#each pillarsX as px, i (i)}
	<rect x={px - 1.5 * u} y={top + rt * 0.9} width={3 * u} height={sill - top - rt * 0.9 - 8 * u} fill="var(--tr-brass)" opacity="0.45" />
	<circle class="read" cx={px} cy={top + (sill - top) * 0.3} r={26 * u} fill="url(#{idp}-read)" style:opacity="var(--tr-glow-o)" />
	<circle cx={px} cy={top + (sill - top) * 0.3} r={4 * u} fill="var(--tr-brass)" />
{/each}
{#each openings as d, i (i)}
	<path {d} fill="none" stroke="var(--tr-frame)" stroke-width={5 * u} />
{/each}
<g clip-path="url(#{idp}-glass)" fill="#fff">
	<path d="M{W * 0.12} 0L{W * 0.2} 0L{W * 0.12 - H * 0.5} {H}L{W * 0.04 - H * 0.5} {H}Z" opacity="0.07" />
	<path d="M{W * 0.52} 0L{W * 0.56} 0L{W * 0.52 - H * 0.5} {H}L{W * 0.48 - H * 0.5} {H}Z" opacity="0.05" />
	<path d="M{W * 0.86} 0L{W * 0.93} 0L{W * 0.86 - H * 0.5} {H}L{W * 0.79 - H * 0.5} {H}Z" opacity="0.06" />
</g>

<!-- Sill and table. -->
<rect y={sill} width={W} height={9 * u} fill="var(--tr-wood)" />
<rect y={sill + 9 * u} width={W} height={2 * u} fill="var(--tr-wood-2)" />
<rect y={sill + 11 * u} width={W} height={H - sill} fill="url(#{idp}-cloth)" />
<rect y={sill + 26 * u} width={W} height={3 * u} fill="var(--tr-stripe)" opacity="0.85" />

{#key animKey}
	<g class="inner inner--{anim}">
		<!-- The pigeon, back to the window for once. -->
		<use href="#dm-mascot" transform="translate({W * 0.2 - 140 * mascotK} {baseY + 8 * u - 300 * mascotK}) scale({mascotK})" />

		{#if problems !== null}
			<g transform="translate({W * 0.5} {baseY + 2 * u}) scale({u})">
				<rect x="-26" y="-34" width="52" height="34" rx="4" fill="#1d2433" stroke="var(--tr-brass)" stroke-width="2" />
				<text x="0" y="-9" text-anchor="middle" font-family="ui-monospace, monospace" font-weight="700" font-size="24" fill={problems > 0 ? '#ffb454' : '#7fd6a2'}>
					{problems}
				</text>
			</g>
		{/if}

		<!-- Tea. -->
		<g class="cup" transform="translate({W * 0.7} {baseY}) scale({u})">
			<ellipse cx="0" cy="0" rx="38" ry="8" fill="var(--tr-cup)" stroke="var(--tr-frame)" stroke-width="1.4" />
			<path d="M-34 1L-14 -2" stroke="var(--tr-brass)" stroke-width="2.2" stroke-linecap="round" />
			<path d="M24-30C42-32 42-10 19-10" fill="none" stroke="var(--tr-frame)" stroke-width="7" stroke-linecap="round" />
			<path d="M24-30C42-32 42-10 19-10" fill="none" stroke="var(--tr-cup)" stroke-width="4.4" stroke-linecap="round" />
			<path d="M-24-34H24C24-12 14-3 0-3S-24-12-24-34Z" fill="var(--tr-cup)" stroke="var(--tr-frame)" stroke-width="1.4" stroke-linejoin="round" />
			<path d="M-23.2-24H23.2" stroke="var(--tr-stripe)" stroke-width="4" />
			<ellipse cx="0" cy="-34" rx="24" ry="5.5" fill="var(--tr-cup)" stroke="var(--tr-frame)" stroke-width="1.4" />
			<ellipse class="tea" cx="0" cy="-33.4" rx="20" ry="4" fill="var(--tr-tea)" />
			<g class="steam" fill="none" stroke="var(--tr-frame)" stroke-width="2.4" stroke-linecap="round">
				<path d="M-8-42C-16-54-2-60-9-74" />
				<path d="M2-42C-6-56 8-62 1-78" style="animation-delay:-1.1s" />
				<path d="M11-42C4-52 16-58 10-70" style="animation-delay:-2.2s" />
			</g>
		</g>

		<!-- The lamp: its shade is the status colour. -->
		<g transform="translate({W * 0.9} {baseY + 8 * u}) scale({u})">
			<circle class="lampglow" cx="0" cy="-58" r="78" fill="url(#{idp}-lamp)" style:opacity="var(--tr-glow-o)" />
			<ellipse cx="0" cy="0" rx="17" ry="4.5" fill="var(--tr-brass)" />
			<rect x="-2" y="-46" width="4" height="46" fill="var(--tr-brass)" />
			<path d="M-19-44L-11-74H11L19-44Z" fill="var(--scene-tone, var(--tr-brass))" stroke="var(--tr-frame)" stroke-width="1.2" stroke-linejoin="round" />
		</g>
	</g>
{/key}

<style>
	.inner--down,
	.inner--up,
	.inner--jolt,
	.inner--resolved {
		animation-fill-mode: both;
	}
	.inner--down {
		animation: lurch 2.4s cubic-bezier(0.2, 0.7, 0.3, 1);
	}
	.inner--up {
		animation: pull 1.8s cubic-bezier(0.2, 0.7, 0.3, 1);
	}
	.inner--jolt {
		animation: jolt 0.7s ease-out;
	}
	.inner--down .tea,
	.inner--up .tea {
		transform-box: fill-box;
		transform-origin: center;
		animation: slosh 2.2s ease-out;
	}
	.inner--resolved .lampglow {
		transform-box: fill-box;
		transform-origin: center;
		animation: flare 2.4s ease-out;
	}
	.steam path {
		opacity: 0;
		animation: steam 3.3s ease-out infinite;
	}
	.read {
		transition: opacity 2s ease;
	}

	@media (prefers-reduced-motion: reduce) {
		.inner--down,
		.inner--up,
		.inner--jolt,
		.inner--down .tea,
		.inner--up .tea,
		.inner--resolved .lampglow,
		.steam path {
			animation: none;
		}
		.steam path {
			opacity: 0.35;
		}
	}

	/* Braking: the table's contents slide forward, then settle. */
	@keyframes lurch {
		0% {
			transform: translateX(0);
		}
		18% {
			transform: translateX(9px);
		}
		45% {
			transform: translateX(-3px);
		}
		70% {
			transform: translateX(1px);
		}
		100% {
			transform: translateX(0);
		}
	}
	@keyframes pull {
		0% {
			transform: translateX(0);
		}
		20% {
			transform: translateX(-6px);
		}
		55% {
			transform: translateX(2px);
		}
		100% {
			transform: translateX(0);
		}
	}
	@keyframes jolt {
		0%,
		100% {
			transform: translateY(0);
		}
		20% {
			transform: translateY(-3px);
		}
		45% {
			transform: translateY(1.5px);
		}
		70% {
			transform: translateY(-1px);
		}
	}
	@keyframes slosh {
		0% {
			transform: rotate(0);
		}
		20% {
			transform: rotate(-7deg) translateX(2px);
		}
		50% {
			transform: rotate(5deg);
		}
		75% {
			transform: rotate(-2deg);
		}
		100% {
			transform: rotate(0);
		}
	}
	@keyframes flare {
		0%,
		100% {
			transform: scale(1);
		}
		25% {
			transform: scale(1.5);
			opacity: 1;
		}
	}
	@keyframes steam {
		0% {
			opacity: 0;
			transform: translateY(6px);
		}
		30% {
			opacity: 0.45;
		}
		100% {
			opacity: 0;
			transform: translateY(-12px);
		}
	}
</style>
