<script lang="ts">
	/**
	 * The DumbMonit mark: the pigeon's face on a tile of night sky, the same
	 * deep blues as the Overview banner, with a few stars and a thin lighter
	 * edge. The sky is the same in both themes (like the banner): the edge
	 * lifts the tile off the black console, and on the light one the dark
	 * tile reads on its own. The static favicon (static/favicon.svg) wears the
	 * same tile.
	 */
	interface Props {
		class?: string;
	}
	let { class: className = 'size-7' }: Props = $props();

	// One gradient per instance: a page can hold several marks, and a gradient
	// defined inside a hidden one (the rail on a phone) would not paint the others.
	const uid = $props.id();
	const sky = `${uid}-sky`;
</script>

<svg viewBox="0 0 64 64" class={className} aria-hidden="true">
	<defs>
		<linearGradient id={sky} gradientUnits="userSpaceOnUse" x1="6" y1="0" x2="58" y2="64">
			<stop offset="0" stop-color="#0a1330" />
			<stop offset="0.6" stop-color="#13244a" />
			<stop offset="1" stop-color="#1c3160" />
		</linearGradient>
	</defs>
	<rect width="64" height="64" rx="16" fill="url(#{sky})" />
	<g fill="#fbf9f4">
		<circle cx="11" cy="10" r="1.5" opacity="0.85" />
		<circle cx="27" cy="5.5" r="0.9" opacity="0.5" />
		<circle cx="53" cy="8" r="1.7" opacity="0.9" />
		<circle cx="58" cy="50" r="1.1" opacity="0.5" />
	</g>
	<circle cx="24" cy="30" r="14" fill="#fff" stroke="#1e2640" stroke-width="4" />
	<circle cx="45" cy="30" r="11" fill="#fff" stroke="#1e2640" stroke-width="4" />
	<circle cx="20" cy="27" r="5" fill="#1e2640" />
	<circle cx="48" cy="27" r="4" fill="#1e2640" />
	<path d="M27 40c3-3 11-3 14 0l-7 12z" fill="#e97b3a" stroke="#1e2640" stroke-width="3" stroke-linejoin="round" />
	<rect x="0.625" y="0.625" width="62.75" height="62.75" rx="15.375" fill="none" stroke="#a9bfee" stroke-opacity="0.26" stroke-width="1.25" />
</svg>
