<script lang="ts">
	/**
	 * A retention in days: a preset list (7 … 730) plus "Custom", which shows a
	 * number box within the server's limits. The value is bound as a number.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { RETENTION_PRESETS } from './storage';

	interface Props {
		id: string;
		value: number;
		min: number;
		max: number;
		disabled?: boolean;
		invalid?: boolean;
	}
	let { id, value = $bindable(), min, max, disabled = false, invalid = false }: Props = $props();

	const presets = $derived(RETENTION_PRESETS.filter((d) => d >= min && d <= max));
	let custom = $state(false);
	const isCustom = $derived(custom || !presets.includes(Number(value)));

	function pick(event: Event) {
		const raw = (event.currentTarget as HTMLSelectElement).value;
		if (raw === 'custom') {
			custom = true;
		} else {
			custom = false;
			value = Number(raw);
		}
	}
</script>

<div class="flex flex-wrap items-center gap-2">
	<select {id} class="input w-auto min-w-36" {disabled} value={isCustom ? 'custom' : String(value)} onchange={pick} aria-invalid={invalid ? 'true' : undefined}>
		{#each presets as days (days)}
			<option value={String(days)}>{m.storage_days({ count: days })}</option>
		{/each}
		<option value="custom">{m.storage_custom()}</option>
	</select>
	{#if isCustom}
		<input
			type="number"
			class="input tnum w-28"
			{min}
			{max}
			step="1"
			bind:value
			{disabled}
			aria-label={m.storage_custom_days()}
			aria-invalid={invalid ? 'true' : undefined}
		/>
		<span class="text-sm text-ink-2">{m.storage_days_unit()}</span>
	{/if}
</div>
