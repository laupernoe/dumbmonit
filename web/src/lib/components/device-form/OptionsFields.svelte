<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * Settings specific to a kind, as described by the server.
	 *
	 * Each option becomes a typed control; values stay strings because they end
	 * up in `Target.tags`. A blank value is never sent: the server then applies
	 * its own default. Two list-shaped inputs are drawn generically from the
	 * description: `checklist` (tick boxes from `items`) and `rows` (lines
	 * with the `columns` the server names, stored as JSON).
	 */
	import { Plus, X } from 'lucide-svelte';
	import type { CollectorOption } from '#lib/api/index.js';
	import { Button, Field, Toggle } from '#lib/ui/index.js';
	import { checkedValues, emptyRow, encodeChecked, parseRows, rowErrors, serializeRows, type Row } from './option-values';

	interface Props {
		options: CollectorOption[];
		/** Current values by option key. A missing key means "server default". */
		values: Record<string, string>;
		errors?: Record<string, string>;
		onchange: (values: Record<string, string>) => void;
	}

	let { options, values, errors = {}, onchange }: Props = $props();

	function set(key: string, value: string) {
		const next = { ...values };
		if (value.trim()) next[key] = value;
		else delete next[key];
		onchange(next);
	}

	function checked(option: CollectorOption): boolean {
		return (values[option.key] ?? option.default).trim().toLowerCase() === 'true';
	}

	/** Text inputs get the full row: they often hold lists or long values. */
	function span(option: CollectorOption): string {
		return option.input === 'text' || option.input === 'boolean' || option.input === 'checklist' || option.input === 'rows'
			? 'sm:col-span-2'
			: '';
	}

	// --- checklist -------------------------------------------------------------
	function ticked(option: CollectorOption): string[] {
		return checkedValues(option, values[option.key]);
	}

	function tick(option: CollectorOption, value: string, on: boolean) {
		const current = new Set(ticked(option));
		if (on) current.add(value);
		else current.delete(value);
		set(option.key, encodeChecked(option, [...current]));
	}

	function tickAll(option: CollectorOption, on: boolean) {
		set(option.key, encodeChecked(option, on ? option.items.map((item) => item.value) : []));
	}

	// --- rows ------------------------------------------------------------------
	// Rows being typed live here: an unfilled row is not stored in the tag, so
	// the tag alone could not hold "a line the user just added".
	let draftRows = $state<Record<string, Row[]>>({});

	function rowsOf(option: CollectorOption): Row[] {
		return draftRows[option.key] ?? parseRows(option, values[option.key]);
	}

	function commitRows(option: CollectorOption, rows: Row[]) {
		draftRows = { ...draftRows, [option.key]: rows };
		set(option.key, serializeRows(option, rows));
	}

	function editRow(option: CollectorOption, index: number, key: string, value: string | boolean) {
		commitRows(
			option,
			rowsOf(option).map((row, i) => (i === index ? { ...row, [key]: value } : row))
		);
	}

	const rowMessages = $derived({ invalid: m.deviceform_options_rows_invalid(), duplicate: m.deviceform_options_rows_duplicate() });
</script>

<div class="grid gap-4 sm:grid-cols-2">
	{#each options as option (option.key)}
		{@const id = `option-${option.key}`}
		<div class={span(option)}>
			{#if option.input === 'boolean'}
				<Field label={option.label} for={id} help={option.help} inline>
					<Toggle {id} checked={checked(option)} onchange={(value) => set(option.key, value ? 'true' : 'false')} />
				</Field>
			{:else if option.input === 'checklist'}
				{@const on = ticked(option)}
				<fieldset class="grid gap-2" aria-describedby={option.help ? `${id}-help` : undefined}>
					<legend class="mb-1 flex w-full flex-wrap items-center justify-between gap-x-4 gap-y-1">
						<span class="text-sm font-semibold text-ink">{option.label}</span>
						<span class="flex flex-wrap items-center gap-x-1">
							<span class="tnum text-[0.8125rem] text-ink-2">
								{m.deviceform_options_checklist_count({ checked: String(on.length), total: String(option.items.length) })}
							</span>
							<Button variant="ghost" size="sm" onclick={() => tickAll(option, true)} disabled={on.length === option.items.length}>
								{m.deviceform_options_checklist_all()}
							</Button>
							<Button variant="ghost" size="sm" onclick={() => tickAll(option, false)} disabled={on.length === 0}>
								{m.deviceform_options_checklist_none()}
							</Button>
						</span>
					</legend>
					{#if option.help}
						<p id={`${id}-help`} class="text-[0.8125rem] text-ink-2">{option.help}</p>
					{/if}
					<ul class="grid gap-1 rounded-lg border border-line p-1.5 sm:grid-cols-2" role="list">
						{#each option.items as item (item.value)}
							<li>
								<label class="flex min-h-10 cursor-pointer items-start gap-3 rounded-md px-2 py-1.5 hover:bg-surface-2">
									<input
										type="checkbox"
										class="mt-0.5 size-5 shrink-0 accent-[var(--c-signal)]"
										checked={on.includes(item.value)}
										onchange={(event) => tick(option, item.value, event.currentTarget.checked)}
									/>
									<span class="min-w-0">
										<span class="flex flex-wrap items-center gap-x-2 gap-y-0.5">
											<span class="text-sm font-semibold text-ink">{item.label}</span>
											{#each item.badges as badge (badge)}
												<span class="rounded border border-line px-1.5 text-[0.6875rem] leading-5 text-ink-2">{badge}</span>
											{/each}
										</span>
										{#if item.help}
											<span class="block text-[0.8125rem] leading-snug text-ink-2">{item.help}</span>
										{/if}
									</span>
								</label>
							</li>
						{/each}
					</ul>
				</fieldset>
			{:else if option.input === 'rows'}
				{@const rows = rowsOf(option)}
				{@const problems = rowErrors(option, rows, rowMessages)}
				<fieldset class="grid gap-2" aria-describedby={option.help ? `${id}-help` : undefined}>
					<legend class="mb-1 flex w-full flex-wrap items-center justify-between gap-x-4 gap-y-1">
						<span class="text-sm font-semibold text-ink">{option.label}</span>
						<Button variant="secondary" size="sm" onclick={() => commitRows(option, [...rows, emptyRow(option.columns)])}>
							<Plus class="size-4" aria-hidden="true" />{m.deviceform_options_rows_add()}
						</Button>
					</legend>
					{#if option.help}
						<p id={`${id}-help`} class="text-[0.8125rem] text-ink-2">{option.help}</p>
					{/if}
					{#if rows.length === 0}
						<p class="text-[0.8125rem] text-ink-3">{m.deviceform_options_rows_empty()}</p>
					{/if}
					{#each rows as row, index (index)}
						<div class="grid gap-1">
							<div class="grid items-center gap-2 sm:grid-cols-[minmax(0,1fr)_7rem_auto_auto]">
								{#each option.columns as column (column.key)}
									{@const cell = `${id}-${index}-${column.key}`}
									{#if column.input === 'boolean'}
										<label class="flex min-h-10 cursor-pointer items-center gap-2 text-sm text-ink">
											<input
												type="checkbox"
												class="size-5 shrink-0 accent-[var(--c-signal)]"
												checked={row[column.key] === true}
												onchange={(event) => editRow(option, index, column.key, event.currentTarget.checked)}
											/>
											{column.label}
										</label>
									{:else if column.input === 'select'}
										<select
											id={cell}
											class="input"
											aria-label={column.label}
											value={String(row[column.key] ?? '')}
											onchange={(event) => editRow(option, index, column.key, event.currentTarget.value)}
										>
											{#each column.choices as choice (choice)}
												<option value={choice}>{choice}</option>
											{/each}
										</select>
									{:else}
										<input
											id={cell}
											class="input font-mono text-[0.8125rem]"
											type="text"
											aria-label={column.label}
											placeholder={column.placeholder}
											autocomplete="off"
											autocapitalize="off"
											spellcheck="false"
											value={String(row[column.key] ?? '')}
											oninput={(event) => editRow(option, index, column.key, event.currentTarget.value)}
											aria-invalid={problems[index]?.[column.key] ? 'true' : undefined}
										/>
									{/if}
								{/each}
								<button
									type="button"
									class="inline-flex size-10 items-center justify-center rounded-md text-ink-2 hover:bg-surface-2 hover:text-ink max-sm:justify-self-end"
									aria-label={m.deviceform_options_rows_remove()}
									onclick={() =>
										commitRows(
											option,
											rows.filter((_, i) => i !== index)
										)}
								>
									<X class="size-4" aria-hidden="true" />
								</button>
							</div>
							{#each Object.values(problems[index] ?? {}) as problem (problem)}
								<p class="text-[0.8125rem] font-medium text-warning-ink" role="alert">{problem}</p>
							{/each}
						</div>
					{/each}
				</fieldset>
			{:else if option.input === 'select'}
				<Field label={option.label} for={id} help={option.help} required={option.required} error={errors[option.key]}>
					<select
						{id}
						class="input"
						value={values[option.key] ?? (option.required ? option.default : '')}
						onchange={(e) => set(option.key, e.currentTarget.value)}
						aria-invalid={errors[option.key] ? 'true' : undefined}
					>
						{#if !option.required}
							<option value="">{option.default ? m.deviceform_options_default_value({ value: option.default }) : m.deviceform_options_default()}</option>
						{/if}
						{#each option.choices as choice (choice)}
							<option value={choice}>{choice}</option>
						{/each}
					</select>
				</Field>
			{:else}
				<Field label={option.label} for={id} help={option.help} required={option.required} error={errors[option.key]}>
					<input
						{id}
						class={`input ${option.input === 'number' ? 'tnum' : ''}`}
						type={option.input === 'number' ? 'number' : 'text'}
						inputmode={option.input === 'number' ? 'decimal' : undefined}
						value={values[option.key] ?? ''}
						placeholder={option.placeholder || option.default}
						autocomplete="off"
						oninput={(e) => set(option.key, e.currentTarget.value)}
						aria-invalid={errors[option.key] ? 'true' : undefined}
					/>
				</Field>
			{/if}
		</div>
	{/each}
</div>
