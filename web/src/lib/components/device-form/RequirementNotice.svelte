<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * What the server's deployment is missing for a device type to work fully.
	 *
	 * Rendered generically from `CollectorInfo.requirements`: the server sends an
	 * `id`, whether it found the requirement in place, the exact Compose lines and
	 * a documentation link. The sentences below are keyed by `id`; an id this
	 * interface does not know still gets a neutral title, the YAML and the link.
	 * Nothing is shown for a requirement the server found satisfied.
	 */
	import { ExternalLink } from 'lucide-svelte';
	import type { CollectorRequirement } from '#lib/api/index.js';
	import { CopyBlock, Plate } from '#lib/ui/index.js';

	interface Props {
		requirements: CollectorRequirement[];
	}

	let { requirements }: Props = $props();

	const missing = $derived(requirements.filter((requirement) => !requirement.satisfied));

	function title(id: string): string {
		if (id === 'browser') return m.deviceform_requirement_browser_title();
		if (id === 'icmp') return m.deviceform_requirement_icmp_title();
		return m.deviceform_requirement_unknown_title();
	}

	/** The consequence of leaving it out, in one sentence. */
	function consequence(id: string): string {
		if (id === 'browser') return m.deviceform_requirement_browser_consequence();
		if (id === 'icmp') return m.deviceform_requirement_icmp_consequence();
		return m.deviceform_requirement_unknown_consequence();
	}
</script>

{#each missing as requirement (requirement.id)}
	<section
		class="rounded-lg border border-advisory/35 bg-advisory-soft px-3 py-2.5"
		aria-label={title(requirement.id)}
	>
		<Plate tone="advisory" label={m.deviceform_requirement_label()} />
		<p class="mt-1.5 text-sm font-semibold text-ink">{title(requirement.id)}</p>
		<p class="mt-1 text-sm leading-relaxed text-ink">{consequence(requirement.id)}</p>
		{#if requirement.compose_snippet}
			<p class="mt-2.5 mb-1.5 text-sm leading-relaxed text-ink-2">{m.deviceform_requirement_add()}</p>
			<CopyBlock value={requirement.compose_snippet} label={m.deviceform_setup_copy()} />
		{/if}
		{#if requirement.doc_url}
			<a
				href={requirement.doc_url}
				target="_blank"
				rel="noreferrer noopener"
				class="mt-2.5 inline-flex min-h-10 items-center gap-1 text-sm font-semibold text-signal-ink hover:underline"
			>
				{m.deviceform_requirement_docs()}
				<ExternalLink class="size-3.5" aria-hidden="true" />
			</a>
		{/if}
	</section>
{/each}
