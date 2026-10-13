<script lang="ts">
	/**
	 * Optional 90-day calendar of a public status page: one small cell per day,
	 * filled column by column over 7 rows, the way a contribution graph reads.
	 * Status is never colour alone: operational is solid, degraded is striped,
	 * outage carries a cross, no data is an empty dashed cell; a legend names all
	 * four. Hover, focus or tap a day for its date, uptime and incidents.
	 *
	 * One tab stop (roving tabindex): up/down move by a day, left/right by a week.
	 * 14 flexible columns keep the grid inside a phone's width.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import type { PublicCalendarDay, PublicCalendarState } from '#lib/api/index.js';
	import { formatPercent } from '#lib/format.js';

	interface Props {
		days: PublicCalendarDay[];
	}

	let { days }: Props = $props();

	const STYLE: Record<PublicCalendarState, { cell: string; mark: string }> = {
		operational: { cell: 'bg-signal', mark: '' },
		degraded: { cell: 'bg-advisory calendar-striped', mark: '' },
		outage: { cell: 'bg-warning text-surface', mark: '×' },
		nodata: { cell: 'border border-dashed border-line-strong bg-transparent', mark: '' }
	};

	function label(state: PublicCalendarState): string {
		switch (state) {
			case 'operational':
				return m.status_calendar_operational();
			case 'degraded':
				return m.status_calendar_degraded();
			case 'outage':
				return m.status_calendar_outage();
			default:
				return m.status_calendar_nodata();
		}
	}

	function formatDay(date: string): string {
		const parsed = new Date(`${date}T00:00:00Z`);
		if (Number.isNaN(parsed.getTime())) return date;
		return new Intl.DateTimeFormat(getLocale(), { day: 'numeric', month: 'short', year: 'numeric', timeZone: 'UTC' }).format(parsed);
	}

	function describe(day: PublicCalendarDay): string {
		const incidents = day.incidents > 0 ? ` · ${m.status_uptime_bar_incidents({ count: day.incidents })}` : '';
		if (day.uptime_pct === null) return `${m.status_calendar_day_no_data({ date: formatDay(day.date) })}${incidents}`;
		return `${m.status_calendar_day({ date: formatDay(day.date), state: label(day.state), uptime: formatPercent(day.uptime_pct) })}${incidents}`;
	}

	// Plain 7-row grid filled column by column: a clean rectangle, only the last column can be short.
	const columns = $derived(Math.ceil(days.length / 7));

	let active = $state<number | null>(null);
	let focusIndex = $state<number | null>(null);
	const tabIndex = $derived(focusIndex ?? days.length - 1);
	let buttons: HTMLButtonElement[] = $state([]);

	function move(to: number) {
		const index = Math.max(0, Math.min(days.length - 1, to));
		focusIndex = index;
		active = index;
		buttons[index]?.focus();
	}

	function onkeydown(event: KeyboardEvent, index: number) {
		const step: Record<string, number> = { ArrowUp: -1, ArrowDown: 1, ArrowLeft: -7, ArrowRight: 7 };
		if (event.key in step) {
			event.preventDefault();
			move(index + step[event.key]);
		} else if (event.key === 'Home') {
			event.preventDefault();
			move(0);
		} else if (event.key === 'End') {
			event.preventDefault();
			move(days.length - 1);
		} else if (event.key === 'Escape') {
			active = null;
		}
	}

	const legend: PublicCalendarState[] = ['operational', 'degraded', 'outage', 'nodata'];
</script>

<div class="rounded-[var(--radius-card)] border border-line bg-surface p-4 shadow-lift sm:p-5">
	<div
		class="mx-auto grid w-full max-w-lg gap-[3px]"
		style={`grid-template-columns: repeat(${columns}, minmax(0, 1fr)); grid-template-rows: repeat(7, auto); grid-auto-flow: column;`}
		role="list"
		aria-label={m.status_calendar_label({ days: days.length })}
		onmouseleave={() => (active = null)}
	>
		{#each days as day, index (day.date)}
			<div role="listitem" class="aspect-square min-w-0">
				<button
					type="button"
					bind:this={buttons[index]}
					tabindex={index === tabIndex ? 0 : -1}
					class={`flex h-full w-full items-center justify-center rounded-[3px] text-[0.625rem] font-bold leading-none transition-opacity focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink ${STYLE[day.state].cell} ${active !== null && active !== index ? 'opacity-50' : ''}`}
					aria-label={describe(day)}
					onmouseenter={() => (active = index)}
					onfocus={() => {
						active = index;
						focusIndex = index;
					}}
					onclick={() => {
						active = index;
						focusIndex = index;
					}}
					onblur={() => (active = null)}
					onkeydown={(event) => onkeydown(event, index)}
				>
					<span aria-hidden="true">{STYLE[day.state].mark}</span>
				</button>
			</div>
		{/each}
	</div>

	<p class="tnum mx-auto mt-3 min-h-5 max-w-lg text-center text-[0.8125rem] text-ink-2" aria-live="polite">
		{#if active !== null && days[active]}{describe(days[active])}{:else}{m.status_calendar_hint()}{/if}
	</p>

	<ul class="mx-auto mt-2 flex max-w-lg flex-wrap justify-center gap-x-4 gap-y-1.5 text-[0.75rem] text-ink-2" aria-hidden="true">
		{#each legend as state (state)}
			<li class="flex items-center gap-1.5">
				<span class={`flex h-3.5 w-3.5 items-center justify-center rounded-[3px] text-[0.625rem] font-bold leading-none ${STYLE[state].cell}`}>{STYLE[state].mark}</span>
				{label(state)}
			</li>
		{/each}
	</ul>
</div>

<style>
	.calendar-striped {
		background-image: repeating-linear-gradient(135deg, transparent 0 3px, rgb(0 0 0 / 0.28) 3px 5px);
	}
</style>
