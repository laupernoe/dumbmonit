<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * First step of "Synology DSM": the agent package or the DSM API. The two
	 * can be combined on the same NAS; the choice only decides which to set up now.
	 */
	import { KeyRound, Package } from 'lucide-svelte';
	import { Button, Plate } from '#lib/ui/index.js';

	let { onpick }: { onpick: (mode: 'agent' | 'api') => void } = $props();
</script>

<div class="grid gap-4">
	<div class="grid gap-3 md:grid-cols-2">
		<section class="flex flex-col gap-3 rounded-[var(--radius-card)] border border-signal/40 bg-surface px-4 py-4" aria-labelledby="syn-pkg">
			<div class="flex flex-wrap items-center gap-2">
				<Package class="size-5 text-signal-ink" aria-hidden="true" />
				<h3 id="syn-pkg" class="text-base font-semibold text-ink">{m.deviceform_synology_pkg_title()}</h3>
				<Plate tone="signal" label={m.deviceform_synology_pkg_recommended()} />
			</div>
			<ul class="grid list-disc gap-1 pl-5 text-sm leading-relaxed text-ink-2">
				<li>{m.deviceform_synology_pkg_point_install()}</li>
				<li>{m.deviceform_synology_pkg_point_outbound()}</li>
				<li>{m.deviceform_synology_pkg_point_reports()}</li>
				<li>{m.deviceform_synology_pkg_point_dsm()}</li>
			</ul>
			<div class="mt-auto"><Button variant="primary" onclick={() => onpick('agent')}>{m.deviceform_synology_pkg_cta()}</Button></div>
		</section>

		<section class="flex flex-col gap-3 rounded-[var(--radius-card)] border border-line bg-surface px-4 py-4" aria-labelledby="syn-api">
			<div class="flex flex-wrap items-center gap-2">
				<KeyRound class="size-5 text-ink-2" aria-hidden="true" />
				<h3 id="syn-api" class="text-base font-semibold text-ink">{m.deviceform_synology_api_title()}</h3>
			</div>
			<ul class="grid list-disc gap-1 pl-5 text-sm leading-relaxed text-ink-2">
				<li>{m.deviceform_synology_api_point_login()}</li>
				<li>{m.deviceform_synology_api_point_gives()}</li>
				<li>{m.deviceform_synology_api_point_dsm()}</li>
				<li>{m.deviceform_synology_api_point_needs()}</li>
			</ul>
			<div class="mt-auto"><Button variant="secondary" onclick={() => onpick('api')}>{m.deviceform_synology_api_cta()}</Button></div>
		</section>
	</div>
	<p class="text-sm leading-relaxed text-ink-2">{m.deviceform_synology_combine()}</p>
</div>
