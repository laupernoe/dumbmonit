<script lang="ts">
	/**
	 * A discreet note per optional feature the probe skipped because the token
	 * lacks a privilege ("Guest agent data skipped: the token lacks VM.Monitor").
	 * Not an error and not an alert: the probe went fine, this only tells the
	 * user why a piece of data is missing. Renders nothing when nothing was
	 * skipped, and nothing if the read fails.
	 */
	import { untrack } from 'svelte';
	import { Info } from 'lucide-svelte';
	import { getSkippedFeatures } from '#lib/api/skipped-features.js';
	import type { SkippedFeature, Target } from '#lib/api/index.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let features = $state<SkippedFeature[]>([]);

	/** Human name of a feature id; an unknown id is shown as served. */
	function featureName(feature: string): string {
		switch (feature) {
			case 'guest_agent':
				return m.devicesb_skipped_features_name_guest_agent();
			case 'updates':
				return m.devicesb_skipped_features_name_updates();
			case 'certificates':
				return m.devicesb_skipped_features_name_certificates();
			case 'remote_updates':
				return m.devicesb_skipped_features_name_remote_updates();
			case 'disks':
				return m.devicesb_skipped_features_name_disks();
			case 'services':
				return m.devicesb_skipped_features_name_services();
			case 'traffic_control':
				return m.devicesb_skipped_features_name_traffic_control();
			default:
				return feature;
		}
	}

	async function load(signal?: AbortSignal) {
		try {
			features = (await getSkippedFeatures(target.id, signal)).features;
		} catch {
			// A note is a courtesy: a failed read leaves the page as it was.
		}
	}

	$effect(() => {
		void target.id;
		features = [];
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});
</script>

{#if features.length > 0}
	<ul class="flex flex-col gap-1 px-1 text-xs text-ink-3">
		{#each features as item (item.feature + item.privilege)}
			<li class="flex items-start gap-1.5">
				<Info class="mt-px size-3.5 shrink-0" aria-hidden="true" />
				<span>
					{m.devicesb_skipped_features_note({
						feature: featureName(item.feature),
						privilege: item.privilege
					})}
				</span>
			</li>
		{/each}
	</ul>
{/if}
