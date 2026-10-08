<script lang="ts">
	/**
	 * Inline panel to schedule a maintenance window (a silence).
	 *
	 * Not a modal: it opens in place, above the list it will add to. A window is
	 * one-off (absolute start/end), weekly (day chips + a daily span) or monthly
	 * (days of the month, or "the first Sunday"). Recurring windows carry a time
	 * zone rather than a fixed offset, so "every Sunday 02:00" stays at 02:00 on
	 * both sides of a daylight-saving change. The server validates the shape; we
	 * validate the obvious mistakes here so the operator hears them without a
	 * round trip.
	 */
	import type { NthWeekday, Target, SilencePayload } from '#lib/api/index.js';
	import { Button, Field, Panel } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { dayName } from './helpers';

	interface Props {
		targets: Target[];
		/** Creates the silence and resolves when the list has been refreshed. */
		oncreate: (payload: SilencePayload) => Promise<void>;
		oncancel: () => void;
	}

	let { targets, oncreate, oncancel }: Props = $props();

	const DAYS = $derived([0, 1, 2, 3, 4, 5, 6].map((index) => ({ index, label: dayName(index) })));

	const RANKS = $derived([
		{ value: 1, label: m.alerts_form_rank_first() },
		{ value: 2, label: m.alerts_form_rank_second() },
		{ value: 3, label: m.alerts_form_rank_third() },
		{ value: 4, label: m.alerts_form_rank_fourth() },
		{ value: 5, label: m.alerts_form_rank_fifth() },
		{ value: -1, label: m.alerts_form_rank_last() }
	]);

	const DURATIONS = $derived([
		{ value: 30, label: m.alerts_span_min({ minutes: 30 }) },
		{ value: 60, label: m.alerts_span_h({ hours: 1 }) },
		{ value: 120, label: m.alerts_span_h({ hours: 2 }) },
		{ value: 240, label: m.alerts_span_h({ hours: 4 }) },
		{ value: 480, label: m.alerts_span_h({ hours: 8 }) },
		{ value: 1440, label: m.alerts_span_h({ hours: 24 }) }
	]);

	/** "2026-09-14T22:00" in local time, for a datetime-local default. */
	function localInput(date: Date): string {
		const pad = (n: number) => String(n).padStart(2, '0');
		return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
	}

	/** The browser's own zone, which is the one the operator is thinking in. */
	function browserZone(): string {
		try {
			return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
		} catch {
			return 'UTC';
		}
	}

	const now = new Date();
	const inTwoHours = new Date(now.getTime() + 2 * 3600_000);

	let name = $state('');
	let comment = $state('');
	let targetId = $state<string>('');
	let mode = $state<'once' | 'weekly' | 'monthly'>('once');
	let startAt = $state(localInput(now));
	let endAt = $state(localInput(inTwoHours));
	let days = $state<number[]>([6]); // Sunday, the usual maintenance night.
	let weeklyStart = $state('02:00');
	let weeklyEnd = $state('04:00');
	let timezone = $state(browserZone());

	// Monthly: either days of the month, or "the first Sunday".
	let monthlyBy = $state<'dates' | 'weekday'>('weekday');
	let monthlyDates = $state('1');
	let monthlyRank = $state(1);
	let monthlyWeekday = $state(6);
	let monthlyStart = $state('02:00');
	let monthlyDuration = $state(120);

	let saving = $state(false);
	let error = $state<string | null>(null);

	function toggleDay(index: number) {
		days = days.includes(index) ? days.filter((d) => d !== index) : [...days, index];
	}

	/** "HH:MM" → minutes since midnight, or null if unreadable. */
	function clockToMinutes(value: string): number | null {
		const match = /^(\d{1,2}):(\d{2})$/.exec(value.trim());
		if (!match) return null;
		const minutes = Number(match[1]) * 60 + Number(match[2]);
		return minutes >= 0 && minutes < 1440 ? minutes : null;
	}

	/** "1, 15" → [1, 15]; null when a token is not a day of the month. */
	function parseDates(text: string): number[] | null {
		const parts = text
			.split(/[,\s]+/)
			.map((part) => part.trim())
			.filter(Boolean);
		if (parts.length === 0) return null;
		const days: number[] = [];
		for (const part of parts) {
			const value = Number(part);
			if (!Number.isInteger(value) || value < 1 || value > 31) return null;
			if (!days.includes(value)) days.push(value);
		}
		return days.sort((a, b) => a - b);
	}

	/** Fixed offset carried alongside the zone, for a server that cannot read it. */
	const offsetMinutes = -new Date().getTimezoneOffset();

	function build(): SilencePayload | null {
		const trimmed = name.trim();
		if (!trimmed) {
			error = m.alerts_form_error_name();
			return null;
		}
		const target_id = targetId === '' ? null : Number(targetId);
		const base = { name: trimmed, comment: comment.trim() || undefined, target_id };

		if (mode === 'once') {
			const start = new Date(startAt);
			const end = new Date(endAt);
			if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) {
				error = m.alerts_form_error_dates();
				return null;
			}
			if (end <= start) {
				error = m.alerts_form_error_order();
				return null;
			}
			return {
				...base,
				schedule: { kind: 'once', starts_at: start.toISOString(), ends_at: end.toISOString() }
			};
		}

		if (mode === 'weekly') {
			if (days.length === 0) {
				error = m.alerts_form_error_days();
				return null;
			}
			const start_minute = clockToMinutes(weeklyStart);
			const end_minute = clockToMinutes(weeklyEnd);
			if (start_minute === null || end_minute === null) {
				error = m.alerts_form_error_daily();
				return null;
			}
			if (start_minute === end_minute) {
				error = m.alerts_form_error_daily_differ();
				return null;
			}
			return {
				...base,
				schedule: {
					kind: 'weekly',
					days: [...days].sort((a, b) => a - b),
					start_minute,
					end_minute,
					utc_offset_minutes: offsetMinutes,
					timezone
				}
			};
		}

		const start_minute = clockToMinutes(monthlyStart);
		if (start_minute === null) {
			error = m.alerts_form_error_start();
			return null;
		}
		let monthDays: number[] = [];
		let nth_weekdays: NthWeekday[] = [];
		if (monthlyBy === 'dates') {
			const parsed = parseDates(monthlyDates);
			if (parsed === null) {
				error = m.alerts_form_error_month_days();
				return null;
			}
			monthDays = parsed;
		} else {
			nth_weekdays = [{ nth: monthlyRank, weekday: monthlyWeekday }];
		}
		return {
			...base,
			schedule: {
				kind: 'monthly',
				days: monthDays,
				nth_weekdays,
				start_minute,
				duration_minutes: monthlyDuration,
				utc_offset_minutes: offsetMinutes,
				timezone
			}
		};
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		error = null;
		const payload = build();
		if (!payload) return;
		saving = true;
		try {
			await oncreate(payload);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : m.alerts_form_error_schedule();
		} finally {
			saving = false;
		}
	}
</script>

<Panel title={m.alerts_silences_schedule()} description={m.alerts_form_description()}>
	<form class="grid gap-4" onsubmit={submit}>
		<Field label={m.alerts_form_name()} for="silence-name" required help={m.alerts_form_name_help()}>
			<input
				id="silence-name"
				class="input"
				bind:value={name}
				placeholder={m.alerts_form_name_placeholder()}
				autocomplete="off"
			/>
		</Field>

		<Field label={m.alerts_history_device()} for="silence-target" help={m.alerts_form_device_help()}>
			<select id="silence-target" class="input" bind:value={targetId}>
				<option value="">{m.alerts_scope_all_devices()}</option>
				{#each targets as target (target.id)}
					<option value={String(target.id)}>{target.name}</option>
				{/each}
			</select>
		</Field>

		<div>
			<span class="mb-1.5 block text-sm font-semibold text-ink">{m.alerts_form_when()}</span>
			<div class="flex flex-wrap gap-2">
				<Button
					size="sm"
					variant={mode === 'once' ? 'secondary' : 'ghost'}
					onclick={() => (mode = 'once')}
				>
					{m.alerts_form_one_off()}
				</Button>
				<Button
					size="sm"
					variant={mode === 'weekly' ? 'secondary' : 'ghost'}
					onclick={() => (mode = 'weekly')}
				>
					{m.alerts_form_weekly()}
				</Button>
				<Button
					size="sm"
					variant={mode === 'monthly' ? 'secondary' : 'ghost'}
					onclick={() => (mode = 'monthly')}
				>
					{m.alerts_form_monthly()}
				</Button>
			</div>
		</div>

		{#if mode === 'once'}
			<div class="grid gap-4 sm:grid-cols-2">
				<Field label={m.alerts_form_start()} for="silence-start">
					<input id="silence-start" type="datetime-local" class="input" bind:value={startAt} />
				</Field>
				<Field label={m.alerts_form_end()} for="silence-end">
					<input id="silence-end" type="datetime-local" class="input" bind:value={endAt} />
				</Field>
			</div>
		{:else if mode === 'weekly'}
			<div>
				<span class="mb-1.5 block text-sm font-semibold text-ink">{m.alerts_form_days()}</span>
				<div class="flex flex-wrap gap-2">
					{#each DAYS as day (day.index)}
						<button
							type="button"
							class={`min-h-10 min-w-11 rounded-lg border px-3 py-1.5 text-sm font-medium transition ${days.includes(day.index) ? 'border-signal bg-signal-soft text-signal-ink' : 'border-line-strong bg-surface text-ink-2 hover:text-ink'}`}
							aria-pressed={days.includes(day.index)}
							onclick={() => toggleDay(day.index)}
						>
							{day.label}
						</button>
					{/each}
				</div>
			</div>
			<div class="grid gap-4 sm:grid-cols-2">
				<Field label={m.alerts_form_from()} for="silence-wstart" help={m.alerts_form_local_time_help()}>
					<input id="silence-wstart" type="time" class="input" bind:value={weeklyStart} />
				</Field>
				<Field label={m.alerts_form_to()} for="silence-wend">
					<input id="silence-wend" type="time" class="input" bind:value={weeklyEnd} />
				</Field>
			</div>
		{:else}
			<div>
				<span class="mb-1.5 block text-sm font-semibold text-ink">{m.alerts_form_each_month()}</span>
				<div class="flex flex-wrap gap-2">
					<Button
						size="sm"
						variant={monthlyBy === 'weekday' ? 'secondary' : 'ghost'}
						onclick={() => (monthlyBy = 'weekday')}
					>
						{m.alerts_form_on_weekday()}
					</Button>
					<Button
						size="sm"
						variant={monthlyBy === 'dates' ? 'secondary' : 'ghost'}
						onclick={() => (monthlyBy = 'dates')}
					>
						{m.alerts_form_on_date()}
					</Button>
				</div>
			</div>
			{#if monthlyBy === 'weekday'}
				<div class="grid gap-4 sm:grid-cols-2">
					<Field label={m.alerts_form_which()} for="silence-rank" help={m.alerts_form_which_help()}>
						<select id="silence-rank" class="input" bind:value={monthlyRank}>
							{#each RANKS as rank (rank.value)}
								<option value={rank.value}>{rank.label}</option>
							{/each}
						</select>
					</Field>
					<Field label={m.alerts_form_day()} for="silence-weekday">
						<select id="silence-weekday" class="input" bind:value={monthlyWeekday}>
							{#each DAYS as day (day.index)}
								<option value={day.index}>{day.label}</option>
							{/each}
						</select>
					</Field>
				</div>
			{:else}
				<Field label={m.alerts_form_month_days()} for="silence-dates" help={m.alerts_form_month_days_help()}>
					<input id="silence-dates" class="input tnum" bind:value={monthlyDates} placeholder="1, 15" autocomplete="off" />
				</Field>
			{/if}
			<div class="grid gap-4 sm:grid-cols-2">
				<Field label={m.alerts_form_from()} for="silence-mstart" help={m.alerts_form_local_time_help()}>
					<input id="silence-mstart" type="time" class="input" bind:value={monthlyStart} />
				</Field>
				<Field label={m.alerts_form_for()} for="silence-mduration" help={m.alerts_form_for_help()}>
					<select id="silence-mduration" class="input" bind:value={monthlyDuration}>
						{#each DURATIONS as option (option.value)}
							<option value={option.value}>{option.label}</option>
						{/each}
					</select>
				</Field>
			</div>
		{/if}

		{#if mode !== 'once'}
			<Field label={m.alerts_form_timezone()} for="silence-tz" help={m.alerts_form_timezone_help()}>
				<input id="silence-tz" class="input" bind:value={timezone} placeholder="Europe/Paris" autocomplete="off" />
			</Field>
		{/if}

		<Field label={m.alerts_form_comment()} for="silence-comment" help={m.alerts_form_comment_help()}>
			<input
				id="silence-comment"
				class="input"
				bind:value={comment}
				placeholder={m.alerts_form_comment_placeholder()}
				autocomplete="off"
			/>
		</Field>

		{#if error}
			<p class="text-[0.8125rem] font-medium text-warning-ink" role="alert">{error}</p>
		{/if}

		<div class="flex flex-wrap items-center gap-2">
			<Button type="submit" variant="primary" loading={saving}>{m.alerts_form_submit()}</Button>
			<Button type="button" variant="ghost" onclick={oncancel} disabled={saving}>{m.alerts_form_cancel()}</Button>
		</div>
	</form>
</Panel>
