<script lang="ts">
	/**
	 * The next 90 days on one axis: a marker per deadline, banded by the alert
	 * tiers (7, 30 and 60 days), neighbours stacked on lanes so none hides
	 * another. Each marker carries a glyph and a full text label, and links to
	 * its monitor. Not shown under the `sm` breakpoint: phones get the list.
	 */
	import type { Deadline } from '#lib/api/types.js';
	import { m } from '#lib/paraglide/messages.js';
	import { TONE_COLOR, TONE_GLYPH, TONE_INK, TONE_SOFT } from '#lib/viz/index.js';
	import type { Tone } from '#lib/viz/index.js';
	import { HORIZON_DAYS, TIER_DAYS, deadlineTone, placeOnLanes } from './geometry.js';
	import { whenLabel } from './labels.js';

	interface Props {
		items: Deadline[];
		class?: string;
	}
	let { items, class: cls = '' }: Props = $props();

	const LANE_REM = 2.5;
	const placed = $derived(placeOnLanes(items, (item) => item.days_left));
	const lanes = $derived(Math.max(1, ...placed.map((p) => p.lane + 1)));

	/** Tier bands, from the left: urgent, soon, notice, calm. */
	const BANDS: { from: number; to: number; tone: Tone }[] = [
		{ from: 0, to: TIER_DAYS[0], tone: 'warning' },
		{ from: TIER_DAYS[0], to: 30, tone: 'advisory' },
		{ from: 30, to: TIER_DAYS[3], tone: 'info' },
		{ from: TIER_DAYS[3], to: HORIZON_DAYS, tone: 'signal' }
	];
	const TICKS = [0, ...TIER_DAYS, HORIZON_DAYS];
	const pct = (days: number) => `${(days / HORIZON_DAYS) * 100}%`;
</script>

<div class={`max-sm:hidden ${cls}`} role="group" aria-label={m.deadlines_timeline_label({ days: HORIZON_DAYS })}>
	<div class="relative mx-5" style:height={`${lanes * LANE_REM + 1.75}rem`}>
		<div class="absolute inset-x-0 top-0 overflow-hidden rounded-[var(--radius-plate)]" style:height={`${lanes * LANE_REM}rem`}>
			{#each BANDS as band (band.from)}
				<div
					class="absolute inset-y-0"
					style:left={pct(band.from)}
					style:width={pct(band.to - band.from)}
					style:background={TONE_SOFT[band.tone]}
					style:opacity="0.55"
				></div>
			{/each}
		</div>
		{#each TICKS as tick (tick)}
			<div class="absolute top-0 w-px bg-line" style:left={pct(tick)} style:height={`${lanes * LANE_REM}rem`}></div>
			<span
				class="tnum absolute text-[0.75rem] text-ink-3"
				style:left={pct(tick)}
				style:top={`${lanes * LANE_REM + 0.25}rem`}
				style:transform={tick === 0 ? 'none' : tick === HORIZON_DAYS ? 'translateX(-100%)' : 'translateX(-50%)'}
			>
				{tick === 0 ? m.deadlines_today() : m.misc_dur_d({ n: tick })}
			</span>
		{/each}
		{#each placed as { item, at, lane } (`${item.target}|${item.kind}|${item.what}|${item.expires_at}`)}
			{@const tone = deadlineTone(item.days_left)}
			<a
				href={`/targets/${item.target}`}
				class="absolute grid size-10 -translate-x-1/2 place-items-center"
				style:left={`${at * 100}%`}
				style:top={`${lane * LANE_REM - 0.25}rem`}
				title={`${item.what} · ${item.target_name} · ${whenLabel(item.days_left)}`}
				aria-label={m.deadlines_marker_label({
					what: item.what,
					target: item.target_name,
					when: whenLabel(item.days_left)
				})}
			>
				<span
					class="grid size-6 place-items-center rounded-full border-2 bg-surface text-[0.75rem] font-semibold"
					style:border-color={TONE_COLOR[tone]}
					style:color={TONE_INK[tone]}
					aria-hidden="true">{TONE_GLYPH[tone]}</span
				>
			</a>
		{/each}
	</div>
</div>
