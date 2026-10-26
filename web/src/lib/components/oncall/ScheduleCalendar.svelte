<script lang="ts">
	/**
	 * A schedule laid out in time: the final line (who is really on call, with
	 * replacements applied) and one line per layer. Week or month, in the
	 * browser's own time zone. Each person has one look and their initials sit
	 * on the bar, so a colour never carries the meaning alone. On a phone the
	 * timeline gives way to an agenda: one line per shift, day by day.
	 */
	import { ChevronLeft, ChevronRight, Repeat2 } from 'lucide-svelte';
	import { getScheduleShifts, type Person, type Schedule, type Shift, type ShiftsResponse } from '#lib/api/index.js';
	import { Button, ErrorNotice, Skeleton } from '#lib/ui/index.js';
	import {
		addDays,
		addMonths,
		browserTimezone,
		formatClock,
		formatDay,
		initials,
		parseIso,
		personLook,
		personName,
		startOfMonth,
		startOfWeek
	} from './helpers';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		schedule: Schedule;
		people: Person[];
		/** Bumped by the parent when shifts may have changed (a replacement, a save). */
		version?: number;
	}
	let { schedule, people, version = 0 }: Props = $props();

	let view = $state<'week' | 'month'>('week');
	let anchor = $state(new Date());
	let data = $state<ShiftsResponse | null>(null);
	let error = $state<unknown>(null);
	let loading = $state(true);

	const from = $derived(view === 'week' ? startOfWeek(anchor) : startOfMonth(anchor));
	const to = $derived(view === 'week' ? addDays(from, 7) : addMonths(from, 1));
	const span = $derived(to.getTime() - from.getTime());
	const dayStarts = $derived.by(() => {
		const out: Date[] = [];
		for (let d = from; d < to; d = addDays(d, 1)) out.push(d);
		return out;
	});

	$effect(() => {
		void version;
		void schedule.updated_at;
		const controller = new AbortController();
		loading = true;
		error = null;
		getScheduleShifts(schedule.id, from.toISOString(), to.toISOString(), controller.signal)
			.then((result) => (data = result))
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				error = cause;
			})
			.finally(() => {
				if (!controller.signal.aborted) loading = false;
			});
		return () => controller.abort();
	});

	function step(by: -1 | 1) {
		anchor = view === 'week' ? addDays(from, 7 * by) : addMonths(from, by);
	}

	const title = $derived(
		view === 'week'
			? m.oncall_cal_week_range({ from: formatDay(from), to: formatDay(addDays(to, -1)) })
			: formatDay(from, { month: 'long', year: 'numeric' })
	);

	interface Bar {
		left: number;
		width: number;
		shift: Shift;
		name: string;
		text: string;
	}

	function bars(shifts: Shift[]): Bar[] {
		const out: Bar[] = [];
		for (const shift of shifts) {
			const start = parseIso(shift.start);
			const end = parseIso(shift.end);
			if (!start || !end) continue;
			const a = Math.max(start.getTime(), from.getTime());
			const b = Math.min(end.getTime(), to.getTime());
			if (b <= a) continue;
			const name = personName(people, shift.user_id);
			out.push({
				left: ((a - from.getTime()) / span) * 100,
				width: ((b - a) / span) * 100,
				shift,
				name,
				text: `${name}: ${formatDay(start)} ${formatClock(start)} – ${formatDay(end)} ${formatClock(end)}`
			});
		}
		return out;
	}

	const rows = $derived.by(() => {
		if (!data) return [];
		return [
			{ key: 'final', label: m.oncall_cal_final(), bars: bars(data.final), final: true },
			...data.layers.map((shifts, i) => ({
				key: `layer-${i}`,
				label: schedule.layers[i]?.name || m.oncall_layer_default_name({ n: i + 1 }),
				bars: bars(shifts),
				final: false
			}))
		];
	});

	const nowPct = $derived.by(() => {
		const t = Date.now();
		return t >= from.getTime() && t < to.getTime() ? ((t - from.getTime()) / span) * 100 : null;
	});

	/** Final shifts clipped to the range and grouped by day, for the agenda. */
	const agenda = $derived.by(() => {
		if (!data) return [];
		return bars(data.final).sort((a, b) => a.left - b.left);
	});

	const browserZone = browserTimezone();
	const wide = $derived(view === 'month');
</script>

<div class="rounded-[var(--radius-card)] border border-line bg-surface shadow-lift">
	<div class="flex flex-wrap items-center justify-between gap-2 border-b border-line px-4 py-3 sm:px-5">
		<div class="min-w-0">
			<h3 class="truncate text-base font-semibold tracking-tight text-ink">{schedule.name}</h3>
			<p class="tnum text-sm text-ink-2" aria-live="polite">{title}</p>
		</div>
		<div class="flex flex-wrap items-center gap-2">
			<div class="inline-flex rounded-lg border border-line bg-surface-2 p-0.5" role="group" aria-label={m.oncall_cal_view()}>
				{#each [['week', m.oncall_cal_week()], ['month', m.oncall_cal_month()]] as [id, label] (id)}
					<button
						type="button"
						aria-pressed={view === id}
						class={`inline-flex h-10 items-center rounded-md px-3 text-[0.8125rem] font-semibold transition sm:h-8 ${view === id ? 'bg-surface text-ink shadow-lift' : 'text-ink-2 hover:text-ink'}`}
						onclick={() => (view = id as 'week' | 'month')}
					>
						{label}
					</button>
				{/each}
			</div>
			<div class="flex items-center gap-1">
				<Button variant="secondary" size="sm" onclick={() => step(-1)} aria-label={m.oncall_cal_previous()} title={m.oncall_cal_previous()}>
					<ChevronLeft class="size-4" aria-hidden="true" />
				</Button>
				<Button variant="secondary" size="sm" onclick={() => (anchor = new Date())}>{m.oncall_cal_today()}</Button>
				<Button variant="secondary" size="sm" onclick={() => step(1)} aria-label={m.oncall_cal_next()} title={m.oncall_cal_next()}>
					<ChevronRight class="size-4" aria-hidden="true" />
				</Button>
			</div>
		</div>
	</div>

	<div class="px-4 py-3 sm:px-5 sm:py-4">
		{#if error}
			<ErrorNotice {error} title={m.oncall_cal_error()} />
		{:else if loading && !data}
			<Skeleton class="h-24 w-full" />
		{:else if data}
			<!-- Timeline: tablets and up. -->
			<div class="max-sm:hidden" role="table" aria-label={m.oncall_cal_aria({ name: schedule.name })}>
				<div class="grid grid-cols-[7.5rem_minmax(0,1fr)] gap-x-3">
					<div></div>
					<div class="relative h-6" aria-hidden="true">
						{#each dayStarts as day (day.getTime())}
							{#if !wide || day.getDay() === 1 || day.getDate() === 1}
								<span class="tnum absolute top-0 text-[0.6875rem] text-ink-3" style={`left: ${((day.getTime() - from.getTime()) / span) * 100}%`}>
									{wide ? day.getDate() : formatDay(day, { weekday: 'short', day: 'numeric' })}
								</span>
							{/if}
						{/each}
					</div>
					{#each rows as row (row.key)}
						<div class={`flex items-center truncate py-1 text-[0.8125rem] ${row.final ? 'font-semibold text-ink' : 'text-ink-2'}`} role="rowheader">{row.label}</div>
						<div class={`relative my-1 overflow-hidden rounded-md border border-line bg-canvas-deep ${row.final ? 'h-11' : 'h-8'}`} role="row">
							{#each dayStarts as day (day.getTime())}
								<span class="absolute inset-y-0 w-px bg-line" style={`left: ${((day.getTime() - from.getTime()) / span) * 100}%`} aria-hidden="true"></span>
							{/each}
							{#each row.bars as bar (bar.shift.start + bar.shift.user_id)}
								<span
									role="cell"
									title={bar.text}
									class={`absolute inset-y-0.5 flex items-center gap-1 overflow-hidden rounded border px-1.5 text-[0.75rem] font-semibold whitespace-nowrap ${personLook(people, bar.shift.user_id)} ${bar.shift.override_id !== null ? 'border-dashed !border-ink' : 'border-transparent'}`}
									style={`left: ${bar.left}%; width: ${bar.width}%`}
								>
									{#if bar.shift.override_id !== null}<Repeat2 class="size-3 shrink-0" aria-hidden="true" />{/if}
									<span class="truncate">{bar.width > (wide ? 9 : 5) ? bar.name : initials(bar.name)}</span>
									<span class="sr-only">{bar.text}{bar.shift.override_id !== null ? `, ${m.oncall_now_replacement()}` : ''}</span>
								</span>
							{/each}
							{#if nowPct !== null}
								<span class="absolute inset-y-0 z-10 w-0.5 bg-ink" style={`left: ${nowPct}%`} aria-hidden="true"></span>
							{/if}
						</div>
					{/each}
				</div>
			</div>

			<!-- Agenda: phones. -->
			<div class="sm:hidden">
				{#if agenda.length === 0}
					<p class="text-sm text-ink-2">{m.oncall_cal_empty()}</p>
				{:else}
					<ul class="grid gap-2">
						{#each agenda as bar (bar.shift.start + bar.shift.user_id)}
							{@const start = parseIso(bar.shift.start)}
							{@const end = parseIso(bar.shift.end)}
							<li class="rounded-lg border border-line bg-canvas-deep px-3 py-2">
								<div class="flex items-center gap-2">
									<span class={`inline-flex size-7 shrink-0 items-center justify-center rounded-full border border-transparent text-[0.6875rem] font-semibold ${personLook(people, bar.shift.user_id)}`} aria-hidden="true">{initials(bar.name)}</span>
									<span class="min-w-0 flex-1 truncate font-semibold text-ink">{bar.name}</span>
									{#if bar.shift.override_id !== null}
										<span class="inline-flex items-center gap-1 text-[0.75rem] font-semibold text-info-ink"><Repeat2 class="size-3.5" aria-hidden="true" />{m.oncall_now_replacement()}</span>
									{/if}
								</div>
								{#if start && end}
									<p class="tnum mt-1 text-[0.8125rem] text-ink-2">
										{formatDay(start)} {formatClock(start)} – {formatDay(end)} {formatClock(end)}
									</p>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</div>

			<p class="mt-3 text-[0.75rem] text-ink-3">
				{m.oncall_cal_timezones({ browser: browserZone, schedule: schedule.timezone })}
			</p>
		{/if}
	</div>
</div>
