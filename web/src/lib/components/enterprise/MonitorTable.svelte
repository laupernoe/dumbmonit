<script lang="ts" module>
	import type { Target } from '#lib/api/index.js';
	import type { ConsoleStatus } from './status.js';

	/** One line of the table, computed by the page from the shared stores. */
	export interface MonitorRow {
		target: Target;
		status: ConsoleStatus;
		kind: string;
		latency: number[];
		/** Milliseconds. */
		p95: number | null;
		/** Percent over 30 days. */
		uptime: number | null;
	}
</script>

<script lang="ts">
	/**
	 * The console's monitor table: dense, sorted worst first, filterable by
	 * state, type and text, with optional columns. Selecting a row (click,
	 * Enter, or j / k then Enter) fills the context panel; `o` or the name's
	 * link opens the monitor's page.
	 *
	 * The table never scrolls sideways: narrow screens drop the secondary
	 * columns instead.
	 */
	import { goto } from '$app/navigation';
	import { Check, Columns3, Search } from 'lucide-svelte';
	import { formatPercent, formatRelative } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { consoleState } from '#lib/stores/console.svelte.js';
	import { Menu } from '#lib/ui/index.js';
	import { isTypingTarget } from '#lib/palette/logic.js';
	import Sparkline from './Sparkline.svelte';
	import StatusMark from './StatusMark.svelte';
	import { STATUS_RANK, STATUS_STROKE } from './status.js';

	interface Props {
		rows: MonitorRow[];
	}
	let { rows }: Props = $props();

	type Filter = 'all' | 'down' | 'degraded' | 'paused';
	type Column = 'type' | 'latency' | 'p95' | 'uptime' | 'checked' | 'address' | 'folder';
	type SortKey = 'status' | 'name' | 'p95' | 'uptime';

	const COLUMNS_KEY = 'dumbmonit-console-columns';
	const DEFAULT_COLUMNS: Column[] = ['type', 'latency', 'p95', 'uptime', 'checked'];
	const ALL_COLUMNS: Column[] = ['type', 'latency', 'p95', 'uptime', 'checked', 'address', 'folder'];

	function readColumns(): Column[] {
		try {
			const raw = JSON.parse(localStorage.getItem(COLUMNS_KEY) ?? 'null');
			if (Array.isArray(raw)) return ALL_COLUMNS.filter((c) => raw.includes(c));
		} catch {
			/* storage blocked or garbled: defaults */
		}
		return DEFAULT_COLUMNS;
	}

	let filter = $state<Filter>('all');
	let kind = $state('');
	let text = $state('');
	let columns = $state<Column[]>(readColumns());
	let sortKey = $state<SortKey>('status');
	let sortDesc = $state(false);
	let focused = $state(0);
	let body = $state<HTMLTableSectionElement | null>(null);

	function toggleColumn(column: Column) {
		columns = columns.includes(column) ? columns.filter((c) => c !== column) : ALL_COLUMNS.filter((c) => c === column || columns.includes(c));
		try {
			localStorage.setItem(COLUMNS_KEY, JSON.stringify(columns));
		} catch {
			/* not remembered */
		}
	}
	const shows = (column: Column) => columns.includes(column);

	function inFilter(row: MonitorRow, f: Filter): boolean {
		switch (f) {
			case 'all':
				return true;
			case 'down':
				return row.status === 'down';
			case 'degraded':
				return row.status === 'degraded' || row.status === 'config';
			case 'paused':
				return row.status === 'paused';
		}
	}

	const FILTERS: { id: Filter; label: () => string }[] = [
		{ id: 'all', label: () => m.console_filter_all() },
		{ id: 'down', label: () => m.console_filter_down() },
		{ id: 'degraded', label: () => m.console_filter_degraded() },
		{ id: 'paused', label: () => m.console_filter_paused() }
	];

	const kinds = $derived([...new Map(rows.map((r) => [r.target.kind, r.kind])).entries()].sort((a, b) => a[1].localeCompare(b[1])));

	const visible = $derived.by(() => {
		const needle = text.trim().toLowerCase();
		const list = rows.filter(
			(row) =>
				inFilter(row, filter) &&
				(!kind || row.target.kind === kind) &&
				(!needle || `${row.target.name} ${row.target.address} ${row.kind} ${row.target.group_name}`.toLowerCase().includes(needle))
		);
		const dir = sortDesc ? -1 : 1;
		const byName = (a: MonitorRow, b: MonitorRow) => a.target.name.localeCompare(b.target.name);
		const num = (v: number | null) => (v === null ? Number.POSITIVE_INFINITY : v);
		return list.sort((a, b) => {
			let diff = 0;
			if (sortKey === 'status') diff = STATUS_RANK[a.status] - STATUS_RANK[b.status];
			else if (sortKey === 'name') diff = byName(a, b);
			else if (sortKey === 'p95') diff = num(a.p95) - num(b.p95);
			else diff = num(a.uptime) - num(b.uptime);
			return diff * dir || byName(a, b);
		});
	});

	function sortBy(key: SortKey) {
		if (sortKey === key) sortDesc = !sortDesc;
		else {
			sortKey = key;
			sortDesc = false;
		}
	}
	const ariaSort = (key: SortKey) => (sortKey === key ? (sortDesc ? 'descending' : 'ascending') : 'none');

	// The first row is selected on arrival, so the panel is never empty.
	$effect(() => {
		if (consoleState.selected === null && visible.length > 0) consoleState.select(visible[0].target.id);
	});

	function focusRow(index: number) {
		const rowsEls = body?.querySelectorAll<HTMLTableRowElement>('tr[data-row]');
		if (!rowsEls || rowsEls.length === 0) return;
		focused = Math.max(0, Math.min(rowsEls.length - 1, index));
		rowsEls[focused]?.focus();
	}

	function onRowKey(event: KeyboardEvent, row: MonitorRow, index: number) {
		if (isTypingTarget(event.target as HTMLElement | null) && event.target !== event.currentTarget) return;
		switch (event.key) {
			case 'ArrowDown':
			case 'j':
				event.preventDefault();
				focusRow(index + 1);
				break;
			case 'ArrowUp':
			case 'k':
				event.preventDefault();
				focusRow(index - 1);
				break;
			case 'Enter':
			case ' ':
				event.preventDefault();
				consoleState.select(row.target.id);
				break;
			case 'o':
				event.preventDefault();
				void goto(`/targets/${row.target.id}`);
				break;
		}
	}

	const COLUMN_LABEL: Record<Column, () => string> = {
		type: () => m.console_col_type(),
		latency: () => m.console_col_latency_short(),
		p95: () => m.console_col_p95(),
		uptime: () => m.console_col_uptime(),
		checked: () => m.console_col_checked(),
		address: () => m.console_col_address(),
		folder: () => m.console_col_folder()
	};
	/** Secondary columns leave on narrow screens rather than push the table sideways. */
	const HIDE: Record<Column, string> = {
		type: 'max-md:hidden',
		latency: 'max-lg:hidden',
		p95: 'max-sm:hidden',
		uptime: '',
		checked: 'max-md:hidden',
		address: 'max-xl:hidden',
		folder: 'max-xl:hidden'
	};
</script>

<section class="flex min-h-0 flex-col overflow-hidden rounded-xl border border-line bg-surface" aria-labelledby="monitors-title">
	<div class="flex flex-wrap items-center gap-2.5 border-b border-line px-4 py-2.5">
		<h2 id="monitors-title" class="m-0 text-[0.8125rem] font-semibold text-ink">{m.console_table_title()}</h2>
		<span class="text-xs text-ink-2 max-md:hidden">{m.console_table_hint()}</span>
		<div role="group" aria-label={m.console_filter_label()} class="ml-auto flex flex-wrap gap-1">
			{#each FILTERS as f (f.id)}
				{@const count = rows.filter((r) => inFilter(r, f.id)).length}
				<button
					type="button"
					aria-pressed={filter === f.id}
					onclick={() => (filter = f.id)}
					class={`inline-flex h-10 items-center gap-1.5 rounded-md border border-line px-2.5 text-xs font-medium transition-colors sm:h-8 ${filter === f.id ? 'bg-line text-ink' : 'text-ink-2 hover:bg-surface-2 hover:text-ink'}`}
				>
					{f.label()} <span class="font-mono text-ink-3">{count}</span>
				</button>
			{/each}
		</div>
	</div>

	<div class="flex flex-wrap items-center gap-2 border-b border-line px-4 py-2">
		<label class="flex h-10 min-w-0 flex-1 items-center gap-2 rounded-md border border-line bg-canvas px-2.5 text-ink-2 sm:h-8 sm:max-w-64">
			<Search class="size-3.5 shrink-0" aria-hidden="true" />
			<span class="sr-only">{m.console_filter_search()}</span>
			<input type="search" bind:value={text} placeholder={m.console_filter_search()} class="min-w-0 flex-1 bg-transparent text-[0.8125rem] text-ink outline-none placeholder:text-ink-3" />
		</label>
		<label class="flex items-center gap-2 text-xs text-ink-2">
			<span class="sr-only">{m.console_filter_kind()}</span>
			<select bind:value={kind} class="h-10 max-w-44 min-w-0 rounded-md border border-line bg-canvas px-2 text-[0.8125rem] text-ink sm:h-8">
				<option value="">{m.console_filter_kind_all()}</option>
				{#each kinds as [id, label] (id)}<option value={id}>{label}</option>{/each}
			</select>
		</label>
		<Menu label={m.console_columns()} align="right" class="ml-auto">
			{#snippet trigger({ toggle, open })}
				<button type="button" onclick={toggle} aria-expanded={open} class="inline-flex h-10 items-center gap-1.5 rounded-md border border-line px-2.5 text-xs font-medium text-ink-2 hover:bg-surface-2 hover:text-ink sm:h-8">
					<Columns3 class="size-3.5" aria-hidden="true" />{m.console_columns()}
				</button>
			{/snippet}
			{#snippet children()}
				<div class="grid gap-0.5 p-1">
					{#each ALL_COLUMNS as column (column)}
						<button
							type="button"
							role="menuitemcheckbox"
							aria-checked={shows(column)}
							onclick={() => toggleColumn(column)}
							class="flex min-h-10 w-full items-center gap-2.5 rounded-md px-2.5 text-left text-[0.8125rem] text-ink hover:bg-surface-2"
						>
							<span aria-hidden="true" class={`grid size-4 place-items-center rounded-[3px] border ${shows(column) ? 'border-info bg-info text-canvas' : 'border-line-strong'}`}>
								{#if shows(column)}<Check class="size-3" />{/if}
							</span>
							{COLUMN_LABEL[column]()}
						</button>
					{/each}
				</div>
			{/snippet}
		</Menu>
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto">
		<table class="w-full table-fixed border-collapse text-[0.8125rem]">
			<thead class="sticky top-0 z-[1] bg-surface">
				<tr class="text-left text-[0.6875rem] text-ink-2">
					<th scope="col" class="w-28 py-2 pl-4 font-medium max-sm:w-24" aria-sort={ariaSort('status')}>
						<button type="button" class="hover:text-ink" onclick={() => sortBy('status')}>{m.console_col_status()}</button>
					</th>
					<th scope="col" class="py-2 font-medium" aria-sort={ariaSort('name')}>
						<button type="button" class="hover:text-ink" onclick={() => sortBy('name')}>{m.console_col_name()}</button>
					</th>
					{#if shows('type')}<th scope="col" class={`w-28 py-2 font-medium ${HIDE.type}`}>{m.console_col_type()}</th>{/if}
					{#if shows('address')}<th scope="col" class={`w-40 py-2 font-medium ${HIDE.address}`}>{m.console_col_address()}</th>{/if}
					{#if shows('folder')}<th scope="col" class={`w-28 py-2 font-medium ${HIDE.folder}`}>{m.console_col_folder()}</th>{/if}
					{#if shows('latency')}<th scope="col" class={`w-36 py-2 font-medium ${HIDE.latency}`}>{m.console_col_latency({ range: consoleState.range })}</th>{/if}
					{#if shows('p95')}
						<th scope="col" class={`w-20 py-2 text-right font-medium ${HIDE.p95}`} aria-sort={ariaSort('p95')}>
							<button type="button" class="hover:text-ink" onclick={() => sortBy('p95')}>{m.console_col_p95()}</button>
						</th>
					{/if}
					{#if shows('uptime')}
						<th scope="col" class={`w-24 py-2 text-right font-medium max-sm:w-20 ${HIDE.uptime}`} aria-sort={ariaSort('uptime')}>
							<button type="button" class="hover:text-ink" onclick={() => sortBy('uptime')}>{m.console_col_uptime()}</button>
						</th>
					{/if}
					{#if shows('checked')}<th scope="col" class={`w-28 py-2 pr-4 text-right font-medium ${HIDE.checked}`}>{m.console_col_checked()}</th>{/if}
				</tr>
			</thead>
			<tbody bind:this={body}>
				{#each visible as row, i (row.target.id)}
					{@const selected = consoleState.selected === row.target.id}
					<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
					<tr
						data-row
						tabindex={i === focused ? 0 : -1}
						aria-selected={selected}
						class={`h-10 cursor-pointer border-t border-line/70 transition-colors hover:bg-surface-2 focus-visible:outline-offset-[-2px] ${selected ? 'bg-surface-2' : ''}`}
						onclick={() => {
							focused = i;
							consoleState.select(row.target.id);
						}}
						ondblclick={() => goto(`/targets/${row.target.id}`)}
						onkeydown={(event) => onRowKey(event, row, i)}
					>
						<td class="pl-4"><StatusMark status={row.status} /></td>
						<td class="truncate pr-2 font-medium text-ink">
							<a href={`/targets/${row.target.id}`} class="hover:underline" onclick={(e) => e.stopPropagation()}>{row.target.name}</a>
							{#if !shows('address')}<span class="ml-1 text-xs font-normal text-ink-3 max-sm:hidden">{row.target.address}</span>{/if}
						</td>
						{#if shows('type')}<td class={`truncate text-ink-2 ${HIDE.type}`}>{row.kind}</td>{/if}
						{#if shows('address')}<td class={`truncate font-mono text-xs text-ink-2 ${HIDE.address}`}>{row.target.address}</td>{/if}
						{#if shows('folder')}<td class={`truncate text-ink-2 ${HIDE.folder}`}>{row.target.group_name || '—'}</td>{/if}
						{#if shows('latency')}<td class={HIDE.latency}><Sparkline values={row.latency} stroke={STATUS_STROKE[row.status]} /></td>{/if}
						{#if shows('p95')}<td class={`text-right font-mono ${HIDE.p95}`}>{row.p95 === null ? '—' : `${Math.round(row.p95)} ms`}</td>{/if}
						{#if shows('uptime')}<td class={`text-right font-mono ${row.status === 'down' ? 'text-warning-ink' : 'text-ink'} ${HIDE.uptime}`}>{formatPercent(row.uptime)}</td>{/if}
						{#if shows('checked')}<td class={`truncate pr-4 text-right font-mono text-ink-2 ${HIDE.checked}`}>{row.target.last_probe_at ? formatRelative(row.target.last_probe_at) : '—'}</td>{/if}
					</tr>
				{:else}
					<tr><td colspan="9" class="px-4 py-6 text-center text-xs text-ink-2">{m.console_table_empty()}</td></tr>
				{/each}
			</tbody>
		</table>
	</div>
	<p class="border-t border-line px-4 py-1.5 font-mono text-[0.625rem] text-ink-3 max-md:hidden">{m.console_table_keys()}</p>
</section>
