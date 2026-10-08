<script lang="ts">
	/**
	 * "Don't alert me about this again" for one device: disables the alert's
	 * rule for that device alone (a per-device override, `enabled: false`).
	 * The alert disappears for good — not snoozed, not acknowledged — the
	 * instant the engine's next cycle runs, and never comes back unless
	 * someone reverses it here, from the device page, or from the rule in
	 * Alerts → Rules.
	 *
	 * Two steps to arm (`Confirm`), then a brief inline "Undo" — ignoring a
	 * rule is reversible and cheap to undo, so a toast would be overkill, but
	 * a silent, permanent-feeling action deserves a beat before it is gone.
	 */
	import type { AlertRule, RuleOverride, Target } from '#lib/api/index.js';
	import { deleteRuleOverride, listRuleOverrides, putRuleOverride } from '#lib/api/index.js';
	import { Button, Confirm } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { EyeOff } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		rule?: AlertRule;
		target?: Target;
		/** The rule stopped (or started) applying to this device: refresh. */
		onchanged?: () => void;
	}

	let { rule, target, onchanged }: Props = $props();

	let overrides = $state<RuleOverride[]>([]);
	let busy = $state(false);
	let error = $state<string | null>(null);
	let justIgnored = $state(false);
	let undoTimer: ReturnType<typeof setTimeout> | null = null;

	const ignored = $derived(
		rule && target
			? (overrides.find((o) => o.rule_uid === rule.uid && o.enabled === false) ?? null)
			: null
	);

	$effect(() => {
		if (!rule || !target) return;
		const controller = new AbortController();
		listRuleOverrides(target.id, controller.signal)
			.then((list) => (overrides = list))
			.catch(() => {
				// A failed read just leaves the button at its default state.
			});
		return () => controller.abort();
	});

	$effect(() => () => {
		if (undoTimer) clearTimeout(undoTimer);
	});

	async function ignore() {
		if (!rule || !target) return;
		busy = true;
		error = null;
		try {
			const saved = await putRuleOverride(rule.id, target.id, { enabled: false });
			overrides = [...overrides.filter((o) => o.rule_uid !== rule.uid), saved];
			justIgnored = true;
			if (undoTimer) clearTimeout(undoTimer);
			undoTimer = setTimeout(() => (justIgnored = false), 8000);
			onchanged?.();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : m.alerts_ignore_error();
		} finally {
			busy = false;
		}
	}

	async function unignore() {
		if (!rule || !target) return;
		busy = true;
		error = null;
		try {
			await deleteRuleOverride(rule.id, target.id);
			overrides = overrides.filter((o) => o.rule_uid !== rule.uid);
			justIgnored = false;
			if (undoTimer) {
				clearTimeout(undoTimer);
				undoTimer = null;
			}
			onchanged?.();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : m.alerts_ignore_error_undo();
		} finally {
			busy = false;
		}
	}
</script>

{#if auth.canOperate && rule && target}
	<div class="flex flex-col items-start gap-1 sm:items-end">
		{#if ignored}
			<Button size="sm" variant="ghost" loading={busy} onclick={unignore}>{m.alerts_ignore_stop()}</Button>
		{:else if justIgnored}
			<p class="text-[0.8125rem] text-ink-2">
				{m.alerts_ignore_done({ device: target.name })}
				<button type="button" class="min-h-10 px-1 font-medium text-ink hover:underline sm:min-h-0" onclick={unignore}>
					{m.alerts_ignore_undo()}
				</button>
			</p>
		{:else}
			<Confirm size="sm" variant="secondary" confirmLabel={m.alerts_ignore_confirm()} loading={busy} onconfirm={ignore}>
				<EyeOff class="size-3.5" aria-hidden="true" />
				{m.alerts_ignore_button()}
			</Confirm>
		{/if}
	</div>
	{#if error}
		<p class="basis-full text-[0.8125rem] text-warning-ink" role="alert">{error}</p>
	{/if}
{/if}
