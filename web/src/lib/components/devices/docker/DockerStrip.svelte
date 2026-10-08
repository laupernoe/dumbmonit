<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The Docker line under an agent's header: how many containers, how many
	 * run, how many could be updated, and what the policies cover — with the
	 * two things a person comes here for: reach the container list, or set the
	 * restart / auto-update policies for the whole fleet at once. The bulk
	 * editor unfolds inline; each switch saves its own row, so a failure only
	 * ever concerns one container.
	 *
	 * Rendered only once the fleet is known and not empty: without Docker there
	 * is nothing to summarise, and the Containers section says so below.
	 */
	import { Container } from 'lucide-svelte';
	import type { Target } from '#lib/api/index.js';
	import { formatRelative } from '#lib/format.js';
	import { Button, Plate, Toggle, type Tone } from '#lib/ui/index.js';
	import { openFold } from '../fold.svelte';
	import { toApiError } from '#lib/api/client.js';
	import { commandContainer, commandLabel, type CommandStatus, type ContainerPolicy, type ContainerView } from './api';
	import { fleetFor } from './fleet.svelte';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	const fleet = $derived(fleetFor(target.id));

	$effect(() => fleet.retain());
	$effect(() => {
		fleet.poll(fleet.busy ? 5_000 : 30_000);
	});

	let editing = $state(false);
	/** Errors of the bulk editor, per container; the row keeps its previous value. */
	let rowError = $state<Record<string, string>>({});
	let applyingAll = $state<'auto_restart' | 'auto_update' | null>(null);

	const total = $derived(fleet.containers.length);
	const stopped = $derived(total - fleet.running);
	const lastCommand = $derived(fleet.commands[0] ?? null);

	const STATUS_TONE: Record<CommandStatus, Tone> = {
		queued: 'ghost',
		running: 'info',
		done: 'signal',
		failed: 'warning',
		cancelled: 'muted',
		expired: 'advisory'
	};
	const STATUS_WORD: Record<CommandStatus, string> = {
		get queued() {
			return m.devices_docker_status_queued();
		},
		get running() {
			return m.devices_docker_status_running();
		},
		get done() {
			return m.devices_docker_status_done();
		},
		get failed() {
			return m.devices_docker_status_failed();
		},
		get cancelled() {
			return m.devices_docker_status_cancelled();
		},
		get expired() {
			return m.devices_docker_status_expired();
		}
	};

	function manage() {
		openFold('containers');
	}

	async function save(c: ContainerView, patch: Partial<ContainerPolicy>) {
		const { [c.name]: _dropped, ...rest } = rowError;
		rowError = rest;
		const failure = await fleet.setPolicy(c, patch);
		if (failure) rowError = { ...rowError, [c.name]: toApiError(failure).message };
	}

	/** Header switch: every container gets the value; rows already there are left alone. */
	async function applyAll(key: 'auto_restart' | 'auto_update', value: boolean) {
		applyingAll = key;
		try {
			await Promise.all(fleet.containers.filter((c) => c.policy[key] !== value).map((c) => save(c, { [key]: value })));
		} finally {
			applyingAll = null;
		}
	}

	const allRestart = $derived(total > 0 && fleet.autoRestart === total);
	const allUpdate = $derived(total > 0 && fleet.autoUpdate === total);

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && editing) {
			editing = false;
			event.stopPropagation();
		}
	}
</script>

<svelte:window {onkeydown} />

{#if !fleet.loading && !fleet.error && total > 0}
	<section
		class="rise-in mt-4 rounded-[var(--radius-card)] border border-line bg-surface px-4 py-3 shadow-lift sm:px-5"
		aria-label="Docker"
		style="--rise-delay: 60ms"
	>
		<div class="flex flex-wrap items-center gap-x-4 gap-y-2">
			<span class="inline-flex items-center gap-2 font-semibold text-ink">
				<Container class="size-4 text-ink-3" aria-hidden="true" />
				Docker
			</span>
			<p class="tnum min-w-0 flex-1 basis-60 text-sm text-ink-2">
				{total === 1 ? m.devices_docker_containers_one({ count: total }) : m.devices_docker_containers_other({ count: total })} · {m.devices_docker_running({ count: fleet.running })}{#if stopped > 0}
					· {m.devices_docker_stopped_n({ count: stopped })}{/if}{#if fleet.updates > 0}
					· {fleet.updates === 1 ? m.devices_docker_updates_one({ count: fleet.updates }) : m.devices_docker_updates_other({ count: fleet.updates })}{/if}
				· {m.devices_docker_policies({ restart: fleet.autoRestart, update: fleet.autoUpdate })}
			</p>
			<div class="flex flex-wrap items-center gap-2">
				<Button size="sm" variant="secondary" class="min-h-10 sm:min-h-0" onclick={manage}>{m.devices_docker_manage()}</Button>
				<Button size="sm" variant={editing ? 'secondary' : 'ghost'} class="min-h-10 sm:min-h-0" onclick={() => (editing = !editing)} aria-expanded={editing} aria-controls={`docker-policies-${target.id}`}>
					{editing ? m.devices_docker_policies_close() : m.devices_docker_policies_open()}
				</Button>
			</div>
		</div>

		<!-- Last thing the agent was asked to do -->
		<p class="mt-2 flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-ink-2" aria-live="polite">
			<span>{m.devices_docker_last_action()}</span>
			{#if lastCommand}
				<Plate tone={STATUS_TONE[lastCommand.status]} label={STATUS_WORD[lastCommand.status]} pulse={lastCommand.status === 'running'} />
				<span class="text-ink">{commandLabel(lastCommand.kind)} <span class="font-semibold break-all">{commandContainer(lastCommand)}</span></span>
				<span class="tnum" title={lastCommand.created_at}>{formatRelative(lastCommand.created_at)}</span>
			{:else}
				<span>{m.devices_docker_last_none()}</span>
			{/if}
		</p>

		{#if editing}
			<div id={`docker-policies-${target.id}`} class="mt-3 border-t border-line pt-3">
				{#if !fleet.commandsSupported}
					<p class="mb-3 rounded-lg border border-advisory/35 bg-advisory-soft px-3 py-2 text-sm text-ink" role="status">
						<Plate tone="advisory" label={m.devices_docker_unavailable()} class="mr-1" />
						{m.devices_docker_strip_unavailable()}
					</p>
				{/if}
				<p class="text-sm text-ink-2">
					{m.devices_docker_window_note()}
					<a href="/alerts#scheduled" class="text-ink underline decoration-line-strong underline-offset-2 hover:text-signal-ink">{m.devices_docker_window_link()}</a>.
					{m.devices_docker_update_how()}
				</p>

				<div class="mt-3 grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-x-4 gap-y-2 sm:gap-x-8" role="table" aria-label={m.devices_docker_policies_aria()}>
					<div class="contents" role="row">
						<span class="label-tape" role="columnheader">{m.devices_docker_col_container()}</span>
						<span class="label-tape text-center leading-tight" role="columnheader"><span class="sm:hidden">{m.devices_docker_col_restart()}</span><span class="hidden sm:inline">{m.devices_docker_restart_if_down()}</span></span>
						<span class="label-tape text-center leading-tight" role="columnheader"><span class="sm:hidden">{m.devices_docker_col_update()}</span><span class="hidden sm:inline">{m.devices_docker_auto_update()}</span></span>
					</div>

					<!-- Apply to all -->
					<div class="contents" role="row">
						<span class="text-sm font-semibold text-ink" role="cell">{m.devices_docker_apply_all()}</span>
						<span class="flex justify-center" role="cell">
							<Toggle id={`policy-all-restart-${target.id}`} checked={allRestart} disabled={applyingAll !== null} label={m.devices_docker_restart_all()} onchange={(v) => void applyAll('auto_restart', v)} />
						</span>
						<span class="flex justify-center" role="cell">
							<Toggle id={`policy-all-update-${target.id}`} checked={allUpdate} disabled={applyingAll !== null} label={m.devices_docker_update_all()} onchange={(v) => void applyAll('auto_update', v)} />
						</span>
					</div>
					<div class="col-span-3 graticule" aria-hidden="true"></div>

					{#each fleet.containers as c (c.name)}
						{@const saving = (fleet.saving[c.name] ?? false) || applyingAll !== null}
						<div class="contents" role="row">
							<span class="flex min-w-0 flex-col gap-0.5" role="cell">
								<span class="flex min-w-0 items-center gap-2">
									<span class="min-w-0 text-sm leading-tight text-ink break-all">{c.name}</span>
									{#if !c.up}<Plate tone="warning" label={m.devices_docker_state_stopped()} bare />{/if}
									{#if c.update_available === true}<Plate tone="info" label={m.devices_docker_update_available()} bare />{/if}
								</span>
								{#if rowError[c.name]}
									<span class="text-xs text-warning-ink" role="alert">{rowError[c.name]}</span>
								{/if}
							</span>
							<span class="flex justify-center" role="cell">
								<Toggle id={`policy-restart-${target.id}-${c.name}`} checked={c.policy.auto_restart} disabled={saving} label={m.devices_docker_restart_one({ name: c.name })} onchange={(v) => void save(c, { auto_restart: v })} />
							</span>
							<span class="flex justify-center" role="cell">
								<Toggle id={`policy-update-${target.id}-${c.name}`} checked={c.policy.auto_update} disabled={saving} label={m.devices_docker_update_one_label({ name: c.name })} onchange={(v) => void save(c, { auto_update: v })} />
							</span>
						</div>
					{/each}
				</div>

				<div class="mt-3 flex justify-end">
					<Button size="sm" variant="ghost" class="min-h-10 sm:min-h-0" onclick={() => (editing = false)}>{m.devices_docker_done()}</Button>
				</div>
			</div>
		{/if}
	</section>
{/if}
