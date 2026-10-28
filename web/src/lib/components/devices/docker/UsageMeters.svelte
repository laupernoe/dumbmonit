<script lang="ts">
	/**
	 * CPU and memory of one container as two thin meters with their figures
	 * written out ("CPU 12 %", "RAM 250 MiB · 25 %"): the number carries the
	 * meaning, the bar and its tone only help the eye. Without a memory limit
	 * only the figure is shown, as there is nothing to fill.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { TONE_COLOR, TONE_GLYPH } from '#lib/viz/tone.js';
	import { formatBytes, type ContainerView } from './api';
	import { memoryRatio, ratioTone } from './usage';

	interface Props {
		container: ContainerView;
	}
	let { container }: Props = $props();

	const cpu = $derived(container.cpu_percent ?? null);
	const memory = $derived(container.memory_bytes ?? null);
	const ratio = $derived(memoryRatio(container));
	const memTone = $derived(ratio == null ? 'signal' : ratioTone(ratio));
	/** One full core is the scale; beyond it the bar is full and the figure says how far. */
	const cpuFill = $derived(cpu == null ? 0 : Math.min(100, cpu));
	const cpuTone = $derived(cpu == null ? 'signal' : ratioTone(cpu / 100));
</script>

{#if cpu !== null || memory !== null}
	<span class="flex min-w-0 flex-wrap items-center gap-x-4 gap-y-1 text-xs text-ink-2">
		{#if cpu !== null}
			<span class="flex items-center gap-2" title={m.devices_docker_cpu_hint()}>
				<span>{m.devices_docker_cpu_label()}</span>
				<span
					class="track"
					role="meter"
					aria-label={m.devices_docker_cpu_label()}
					aria-valuemin={0}
					aria-valuemax={100}
					aria-valuenow={cpuFill}
					aria-valuetext={`${cpu.toFixed(cpu >= 10 ? 0 : 1)}%`}
				>
					<span class="fill" style:width={`${cpuFill}%`} style:background={TONE_COLOR[cpuTone]}></span>
				</span>
				<span class="tnum w-10 text-right text-ink">{cpu.toFixed(cpu >= 10 ? 0 : 1)}%</span>
				{#if cpuTone !== 'signal'}<span aria-hidden="true">{TONE_GLYPH[cpuTone]}</span>{/if}
			</span>
		{/if}
		{#if memory !== null}
			<span class="flex items-center gap-2">
				<span>{m.devices_docker_mem_label()}</span>
				{#if ratio !== null}
					<span
						class="track"
						role="meter"
						aria-label={m.devices_docker_mem_label()}
						aria-valuemin={0}
						aria-valuemax={100}
						aria-valuenow={Math.min(100, Math.round(ratio * 100))}
						aria-valuetext={`${formatBytes(memory)}, ${Math.round(ratio * 100)}%`}
					>
						<span class="fill" style:width={`${Math.min(100, ratio * 100)}%`} style:background={TONE_COLOR[memTone]}></span>
					</span>
				{/if}
				<span class="tnum text-ink">
					{formatBytes(memory)}{#if ratio !== null} · {Math.round(ratio * 100)}%{/if}
				</span>
				{#if memTone !== 'signal'}<span aria-hidden="true">{TONE_GLYPH[memTone]}</span>{/if}
			</span>
		{/if}
	</span>
{/if}

<style>
	.track {
		display: inline-block;
		position: relative;
		width: 4rem;
		height: 0.375rem;
		border-radius: 9999px;
		background: var(--c-ghost);
		overflow: hidden;
	}
	.fill {
		display: block;
		height: 100%;
		border-radius: 9999px;
		opacity: 0.85;
	}
	@media (prefers-reduced-motion: no-preference) {
		.fill {
			transition: width 200ms ease-out;
		}
	}
</style>
