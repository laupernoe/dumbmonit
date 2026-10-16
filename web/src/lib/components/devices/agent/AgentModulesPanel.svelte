<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The services the agent looks for on this machine, found or not.
	 *
	 * Nothing is chosen before the install: the agent detects each module on its
	 * own and stays silent where there is nothing. Each one it finds becomes a
	 * device of its own under this machine (status, alerts, rules). Here an
	 * admin unticks a found service to drop its device and stop its readings,
	 * or ticks one the agent has not found yet to watch it all the same.
	 */
	import { listAgentModules, setAgentModule, type AgentModule, type Target } from '#lib/api/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { ErrorNotice, Panel, Plate, type Tone } from '#lib/ui/index.js';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let modules = $state<AgentModule[]>([]);
	let error = $state<unknown>(null);
	let saveError = $state<unknown>(null);
	let saving = $state<string | null>(null);

	/** Translated names; a module this build does not know shows the server's label. */
	const NAMES: Record<string, () => string> = {
		docker: m.deviceform_modules_docker,
		plakar: m.deviceform_modules_plakar,
		backup_repos: m.deviceform_feature_backups_label,
		zfs: m.deviceform_modules_zfs,
		libvirt: m.deviceform_modules_libvirt,
		wireguard: m.deviceform_modules_wireguard,
		ipsec: m.deviceform_modules_ipsec,
		speedtest: m.deviceform_feature_speedtest_label
	};
	const nameOf = (module: AgentModule) => NAMES[module.id]?.() ?? module.label;

	function plateOf(module: AgentModule): { tone: Tone; label: string } {
		if (module.detected && module.enabled) return { tone: 'signal', label: m.devices_agentmodules_found() };
		if (module.detected) return { tone: 'muted', label: m.devices_agentmodules_off() };
		if (module.enabled) return { tone: 'advisory', label: m.devices_agentmodules_waiting() };
		return { tone: 'ghost', label: m.devices_agentmodules_not_found() };
	}

	$effect(() => {
		const id = target.id;
		const controller = new AbortController();
		error = null;
		listAgentModules(id, controller.signal)
			.then((list) => (modules = list))
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				error = cause;
			});
		return () => controller.abort();
	});

	async function toggle(module: AgentModule, enabled: boolean) {
		saving = module.id;
		saveError = null;
		try {
			const saved = await setAgentModule(target.id, module.id, enabled);
			modules = modules.map((item) => (item.id === saved.id ? saved : item));
		} catch (cause) {
			saveError = cause;
		} finally {
			saving = null;
		}
	}

	const found = $derived(modules.filter((module) => module.detected).length);
</script>

{#if error}
	<ErrorNotice {error} title={m.devices_agentmodules_err_load()} />
{:else if modules.length > 0}
	<Panel title={m.devices_agentmodules_title()} description={m.devices_agentmodules_description()} class="rise-in">
		{#snippet aside()}
			<span class="tnum text-sm text-ink-2">{m.devices_agentmodules_count({ found, total: modules.length })}</span>
		{/snippet}
		<ul class="divide-y divide-line" role="list">
			{#each modules as module (module.id)}
				{@const plate = plateOf(module)}
				<li class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 py-2">
					<label class="flex min-h-10 min-w-0 flex-1 cursor-pointer items-center gap-3">
						<input
							type="checkbox"
							class="size-5 shrink-0 accent-[var(--c-signal)]"
							checked={module.enabled}
							disabled={!auth.isAdmin || saving !== null}
							aria-label={m.devices_agentmodules_toggle({ name: nameOf(module) })}
							onchange={(event) => void toggle(module, event.currentTarget.checked)}
						/>
						<span class="min-w-0 truncate text-sm font-semibold text-ink">{nameOf(module)}</span>
					</label>
					<span class="flex min-w-0 items-center gap-2">
						{#if module.last_seen_at}
							<time class="tnum hidden text-[0.8125rem] text-ink-2 sm:inline" title={formatDateTime(module.last_seen_at)}>
								{m.devices_agentmodules_seen({ when: formatRelative(module.last_seen_at) })}
							</time>
						{/if}
						<Plate tone={plate.tone} label={plate.label} />
						{#if module.target_id !== null}
							<a href={`/targets/${module.target_id}`} class="inline-flex min-h-10 items-center text-sm text-ink underline decoration-line-strong underline-offset-2 hover:text-signal-ink sm:min-h-0">
								{m.devices_agentmodules_open()}
							</a>
						{/if}
					</span>
				</li>
			{/each}
		</ul>
		{#if auth.isAdmin}
			<p class="mt-3 text-sm text-ink-2">{m.devices_agentmodules_hint()}</p>
		{/if}
		{#if saveError}
			<ErrorNotice error={saveError} title={m.devices_agentmodules_err_save()} class="mt-3" />
		{/if}
	</Panel>
{/if}
