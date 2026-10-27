<script lang="ts">
	/**
	 * A miniature of each mode, drawn with boxes: Homelab's bar, sky band and
	 * rack; Enterprise's rail, figures, river and context panel. Decorative —
	 * the picker's words say what each mode is.
	 */
	import type { ProductMode } from '#lib/api/index.js';

	let { mode, class: cls = '' }: { mode: ProductMode; class?: string } = $props();

	// A fixed, made-up river: two bad hours on one line, a wobble on another.
	const river = [0, 1, 2, 3].map((row) =>
		Array.from({ length: 24 }, (_, c) => (row === 0 && c > 20 ? 'down' : row === 1 && c > 16 ? 'warn' : (c * 7 + row * 5) % 23 === 0 ? 'warn' : 'up'))
	);
	const CELL: Record<string, string> = { up: 'bg-pv-up/45', warn: 'bg-pv-warn', down: 'bg-pv-down' };
</script>

{#if mode === 'homelab'}
	<div aria-hidden="true" class={`mode-preview-homelab overflow-hidden rounded-lg border border-pv-line bg-pv-canvas ${cls}`}>
		<div class="flex items-center gap-1.5 border-b border-pv-line px-2.5 py-1.5">
			<span class="size-2.5 rounded-[3px] bg-pv-ink-2"></span>
			<span class="h-1.5 w-8 rounded-full bg-pv-ink/70"></span>
			<span class="ml-2 h-1.5 w-6 rounded-full bg-pv-ink-2/40"></span>
			<span class="h-1.5 w-6 rounded-full bg-pv-ink-2/40"></span>
			<span class="h-1.5 w-6 rounded-full bg-pv-ink-2/40"></span>
		</div>
		<div class="px-2.5 pt-2.5">
			<div class="rounded-md bg-pv-surface px-2.5 py-2.5">
				<span class="block h-2.5 w-3/4 rounded-full bg-pv-ink"></span>
				<span class="mt-1.5 block h-1.5 w-1/2 rounded-full bg-pv-ink-2/50"></span>
				<span class="mt-2 inline-flex items-center gap-1 rounded-full border border-pv-line px-1.5 py-0.5">
					<span class="size-1.5 rounded-full bg-pv-up"></span><span class="h-1 w-6 rounded-full bg-pv-ink-2/50"></span>
				</span>
			</div>
		</div>
		<div class="grid grid-cols-3 gap-1.5 p-2.5">
			{#each [0, 1, 2, 3, 4, 5] as i (i)}
				<span class="flex items-center gap-1 rounded-[4px] border border-pv-line bg-pv-surface px-1.5 py-1.5">
					<span class={`size-1.5 rounded-full ${i === 4 ? 'bg-pv-warn' : 'bg-pv-up'}`}></span>
					<span class="h-1 flex-1 rounded-full bg-pv-ink-2/40"></span>
				</span>
			{/each}
		</div>
	</div>
{:else}
	<div aria-hidden="true" class={`mode-preview-enterprise flex overflow-hidden rounded-lg border border-pv-line bg-pv-canvas ${cls}`}>
		<div class="flex w-5 shrink-0 flex-col items-center gap-1.5 border-r border-pv-line py-2">
			<span class="size-2.5 rounded-[3px] border border-pv-accent/50"></span>
			{#each [0, 1, 2, 3, 4] as i (i)}<span class={`size-2 rounded-[2px] ${i === 0 ? 'bg-pv-ink/70' : 'bg-pv-ink-2/40'}`}></span>{/each}
		</div>
		<div class="min-w-0 flex-1 p-2">
			<div class="grid grid-cols-4 gap-px overflow-hidden rounded-[4px] border border-pv-line bg-pv-line">
				{#each [0, 1, 2, 3] as i (i)}
					<span class="bg-pv-surface px-1 py-1.5"><span class="block h-1 w-3/4 rounded-full bg-pv-ink-2/50"></span><span class="mt-1 block h-1.5 w-1/2 rounded-full bg-pv-ink"></span></span>
				{/each}
			</div>
			<div class="mt-1.5 grid gap-[2px] rounded-[4px] border border-pv-line bg-pv-surface p-1.5">
				{#each river as row, r (r)}
					<div class="grid grid-cols-[repeat(24,minmax(0,1fr))] gap-px">
						{#each row as cell, c (c)}<span class={`h-1 rounded-[1px] ${CELL[cell]}`}></span>{/each}
					</div>
				{/each}
			</div>
			<div class="mt-1.5 grid gap-1 rounded-[4px] border border-pv-line bg-pv-surface p-1.5">
				{#each ['down', 'warn', 'up', 'up'] as state, i (i)}
					<span class="flex items-center gap-1">
						<span class={`size-1 ${state === 'down' ? 'rounded-[1px] bg-pv-down' : state === 'warn' ? 'rotate-45 bg-pv-warn' : 'rounded-full bg-pv-up'}`}></span>
						<span class="h-1 w-1/3 rounded-full bg-pv-ink/60"></span>
						<span class="ml-auto h-1 w-1/5 rounded-full bg-pv-ink-2/40"></span>
					</span>
				{/each}
			</div>
		</div>
		<div class="w-1/4 shrink-0 border-l border-pv-line bg-pv-surface p-1.5">
			<span class="block h-1 w-2/3 rounded-full bg-pv-ink-2/50"></span>
			<span class="mt-1.5 block h-5 rounded-[3px] border-l-2 border-pv-down bg-pv-canvas"></span>
			<span class="mt-1 block h-5 rounded-[3px] border-l-2 border-pv-warn bg-pv-canvas"></span>
			<span class="mt-2 block h-1 w-1/2 rounded-full bg-pv-ink-2/50"></span>
			<span class="mt-1 block h-3 rounded-[3px] bg-pv-canvas"></span>
		</div>
	</div>
{/if}
