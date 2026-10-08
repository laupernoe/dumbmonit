<script lang="ts">
	/**
	 * Weekly quiet-hours editor: day chips and a daily span, in the operator's
	 * own timezone (the offset travels with the schedule, like a maintenance
	 * window). Emits `null` when quiet hours are off.
	 */
	import { untrack } from 'svelte';
	import type { QuietHours } from '#lib/api/index.js';
	import { Field, Toggle } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';

	interface Props {
		value: QuietHours | null;
		onchange: (next: QuietHours | null) => void;
		idPrefix?: string;
		disabled?: boolean;
	}

	let { value, onchange, idPrefix = 'quiet', disabled = false }: Props = $props();

	/** Short weekday names in the UI language (2024-01-01 is a Monday). */
	const DAYS = $derived(
		[0, 1, 2, 3, 4, 5, 6].map((index) => ({
			index,
			label: new Intl.DateTimeFormat(getLocale(), { weekday: 'short', timeZone: 'UTC' }).format(
				new Date(Date.UTC(2024, 0, 1 + index))
			)
		}))
	);

	function toClock(minute: number): string {
		const pad = (n: number) => String(n).padStart(2, '0');
		return `${pad(Math.floor(minute / 60))}:${pad(minute % 60)}`;
	}

	/** "HH:MM" → minutes since midnight, or null if unreadable. */
	function clockToMinutes(text: string): number | null {
		const match = /^(\d{1,2}):(\d{2})$/.exec(text.trim());
		if (!match) return null;
		const minutes = Number(match[1]) * 60 + Number(match[2]);
		return minutes >= 0 && minutes < 1440 ? minutes : null;
	}

	// Local drafts seeded once from the initial value (untrack: this is meant);
	// a sensible night is the default when switching on.
	const initial = untrack(() => value);
	let enabled = $state(initial !== null);
	let days = $state<number[]>(initial?.days ?? [0, 1, 2, 3, 4, 5, 6]);
	let start = $state(toClock(initial?.start_minute ?? 22 * 60));
	let end = $state(toClock(initial?.end_minute ?? 7 * 60));

	function emit() {
		if (!enabled) {
			onchange(null);
			return;
		}
		const start_minute = clockToMinutes(start);
		const end_minute = clockToMinutes(end);
		if (start_minute === null || end_minute === null || days.length === 0) return;
		onchange({
			kind: 'weekly',
			days: [...days].sort((a, b) => a - b),
			start_minute,
			end_minute,
			utc_offset_minutes: -new Date().getTimezoneOffset()
		});
	}

	function toggleDay(index: number) {
		days = days.includes(index) ? days.filter((d) => d !== index) : [...days, index];
		emit();
	}

	const problem = $derived.by(() => {
		if (!enabled) return null;
		if (days.length === 0) return m.notifications_quiet_error_days();
		const s = clockToMinutes(start);
		const e = clockToMinutes(end);
		if (s === null || e === null) return m.notifications_quiet_error_format();
		if (s === e) return m.notifications_quiet_error_differ();
		return null;
	});
</script>

<div class="grid gap-3">
	<Field label={m.notifications_quiet_title()} for={`${idPrefix}-on`} inline help={m.notifications_quiet_help()}>
		<Toggle
			id={`${idPrefix}-on`}
			checked={enabled}
			{disabled}
			label={m.notifications_quiet_title()}
			onchange={(next) => {
				enabled = next;
				emit();
			}}
		/>
	</Field>

	{#if enabled}
		<div class="grid gap-3 pl-1">
			<div>
				<span class="mb-1.5 block text-sm font-semibold text-ink">{m.notifications_quiet_days()}</span>
				<div class="flex flex-wrap gap-2">
					{#each DAYS as day (day.index)}
						<button
							type="button"
							class={`min-h-10 rounded-lg border px-3 py-1.5 text-sm font-medium transition ${days.includes(day.index) ? 'border-signal bg-signal-soft text-signal-ink' : 'border-line-strong bg-surface text-ink-2 hover:text-ink'}`}
							aria-pressed={days.includes(day.index)}
							{disabled}
							onclick={() => toggleDay(day.index)}
						>
							{day.label}
						</button>
					{/each}
				</div>
			</div>
			<div class="grid gap-3 sm:grid-cols-2">
				<Field label={m.notifications_quiet_from()} for={`${idPrefix}-start`} help={m.notifications_quiet_from_help()}>
					<input id={`${idPrefix}-start`} type="time" class="input tnum" bind:value={start} {disabled} onchange={emit} />
				</Field>
				<Field label={m.notifications_quiet_to()} for={`${idPrefix}-end`}>
					<input id={`${idPrefix}-end`} type="time" class="input tnum" bind:value={end} {disabled} onchange={emit} />
				</Field>
			</div>
			{#if problem}
				<p class="text-[0.8125rem] font-medium text-warning-ink" role="alert">{problem}</p>
			{/if}
		</div>
	{/if}
</div>
