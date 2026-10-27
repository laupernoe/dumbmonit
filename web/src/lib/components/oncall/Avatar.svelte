<script lang="ts">
	/** A person as initials on their own look; the name is always spelled out next to it. */
	import type { Person } from '#lib/api/index.js';
	import { initials, personLook, personName } from './helpers';

	interface Props {
		people: Person[];
		id: number;
		size?: 'sm' | 'md';
	}
	let { people, id, size = 'md' }: Props = $props();
	const name = $derived(personName(people, id));
</script>

<span
	class={`inline-flex shrink-0 items-center justify-center rounded-full border border-transparent font-semibold ${size === 'md' ? 'size-9 text-[0.8125rem]' : 'size-6 text-[0.6875rem]'} ${personLook(people, id)}`}
	aria-hidden="true"
>
	{initials(name)}
</span>
