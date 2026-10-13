<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * Which optional modules the agent runs. Off by default ("detect
	 * everything"), the agent's historical behaviour; switched on, only the
	 * ticked modules run, and the others send no series and raise no alert.
	 * The choice only changes the install command (`--modules=` /
	 * `-Modules`), which writes the `modules` list of agent.yaml.
	 */
	import { Toggle } from '#lib/ui/index.js';
	import { AGENT_MODULES, type AgentModule } from './agent-modules.js';

	interface Props {
		/** `null`: every module detects itself. Bind it. */
		modules: AgentModule[] | null;
	}

	let { modules = $bindable(null) }: Props = $props();

	/** Remembered while "detect everything" is on, so a toggle back restores the ticks. */
	let picked = $state<AgentModule[]>(['docker']);
	const choosing = $derived(modules !== null);

	const labels: Record<AgentModule, () => string> = {
		docker: m.deviceform_modules_docker,
		plakar: m.deviceform_modules_plakar,
		system_health: m.deviceform_modules_system_health,
		sensors: m.deviceform_modules_sensors,
		smart: m.deviceform_modules_smart,
		zfs: m.deviceform_modules_zfs,
		wireguard: m.deviceform_modules_wireguard,
		perf_counters: m.deviceform_modules_perf_counters
	};
	const hints: Record<AgentModule, () => string> = {
		docker: m.deviceform_modules_docker_hint,
		plakar: m.deviceform_modules_plakar_hint,
		system_health: m.deviceform_modules_system_health_hint,
		sensors: m.deviceform_modules_sensors_hint,
		smart: m.deviceform_modules_smart_hint,
		zfs: m.deviceform_modules_zfs_hint,
		wireguard: m.deviceform_modules_wireguard_hint,
		perf_counters: m.deviceform_modules_perf_counters_hint
	};

	function setChoosing(on: boolean) {
		modules = on ? [...picked] : null;
	}

	function tick(module: AgentModule, on: boolean) {
		const next = AGENT_MODULES.filter((item) => (item === module ? on : (modules ?? picked).includes(item)));
		picked = next;
		modules = next;
	}
</script>

<fieldset class="grid gap-3 rounded-[var(--radius-card)] border border-line bg-surface px-3 py-3 sm:px-4">
	<legend class="px-1 text-sm font-semibold text-ink">{m.deviceform_modules_title()}</legend>
	<div class="flex items-start justify-between gap-3">
		<div class="min-w-0">
			<p class="text-sm text-ink">{m.deviceform_modules_choose()}</p>
			<p class="text-sm leading-relaxed text-ink-2">
				{choosing ? m.deviceform_modules_choose_on() : m.deviceform_modules_choose_off()}
			</p>
		</div>
		<Toggle checked={choosing} label={m.deviceform_modules_choose()} onchange={setChoosing} />
	</div>
	{#if choosing}
		<ul class="grid gap-1 sm:grid-cols-2" role="list">
			{#each AGENT_MODULES as module (module)}
				<li>
					<label class="flex min-h-10 cursor-pointer items-start gap-3 rounded-md px-1 py-1.5 hover:bg-surface-2">
						<input
							type="checkbox"
							class="mt-0.5 size-5 shrink-0 accent-[var(--c-signal)]"
							checked={modules?.includes(module) ?? false}
							onchange={(event) => tick(module, event.currentTarget.checked)}
						/>
						<span class="min-w-0">
							<span class="block text-sm font-semibold text-ink">{labels[module]()}</span>
							<span class="block text-[0.8125rem] leading-snug text-ink-2">{hints[module]()}</span>
						</span>
					</label>
				</li>
			{/each}
		</ul>
		{#if modules && modules.length === 0}
			<p class="text-sm text-ink-2">{m.deviceform_modules_none()}</p>
		{/if}
	{/if}
</fieldset>
