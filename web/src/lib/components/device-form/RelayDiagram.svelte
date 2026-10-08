<script lang="ts">
	/**
	 * How a relay agent works, in one picture: the agent sits inside the
	 * remote site (behind its NAT), probes the devices there, and carries the
	 * results out to this server over HTTPS. Every arrow points the way the
	 * connection is opened: nothing ever goes in to the site.
	 *
	 * Geometry only, coloured by the theme's tokens. Small packets travel the
	 * lines while the diagram is on screen; under reduced motion the arrows
	 * alone carry the direction.
	 */
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		/** Inside a control that already says it in words: hidden from assistive tech. */
		decorative?: boolean;
		class?: string;
	}
	let { decorative = false, class: className = '' }: Props = $props();

	const uid = $props.id();

	/** Probes from the agent to each device: a line, its angle and length for the packet. */
	const DEVICES = [
		{ label: m.deviceform_relay_switch(), y: 33 },
		{ label: m.deviceform_relay_nas(), y: 64 },
		{ label: m.deviceform_relay_proxmox(), y: 95 }
	].map((d, i) => {
		const dx = 240 - 204;
		const dy = d.y - 64;
		return {
			...d,
			i,
			angle: (Math.atan2(dy, dx) * 180) / Math.PI,
			length: Math.hypot(dx, dy) - 4
		};
	});
</script>

<svg
	viewBox="0 0 320 128"
	class={`relay-diagram ${className}`}
	role={decorative ? undefined : 'img'}
	aria-hidden={decorative ? 'true' : undefined}
	aria-label={decorative
		? undefined
		: m.deviceform_relay_aria()}
>
	<defs>
		<marker id={`${uid}-head`} viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
			<path d="M0 0.5L8 4L0 7.5Z" class="head" />
		</marker>
		<marker id={`${uid}-head-signal`} viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
			<path d="M0 0.5L8 4L0 7.5Z" class="head head--signal" />
		</marker>
	</defs>

	<!-- This server -->
	<g>
		<rect x="4" y="44" width="84" height="40" rx="6" class="unit" />
		<circle cx="15" cy="58" r="3" class="led" />
		<text x="23" y="61.5" class="t-strong">DumbMonit</text>
		<text x="23" y="75" class="t-soft">{m.deviceform_relay_this_server()}</text>
	</g>

	<!-- The remote site, behind its NAT -->
	<rect x="124" y="5" width="192" height="118" rx="11" class="site" />
	<text x="134" y="19" class="t-tape">{m.deviceform_relay_remote_site()}</text>
	<text x="307" y="19" text-anchor="end" class="t-faint">{m.deviceform_relay_behind_nat()}</text>

	<!-- Outbound link: agent → server -->
	<line x1="140" y1="64" x2="90" y2="64" class="link link--signal" marker-end={`url(#${uid}-head-signal)`} />
	<text x="106" y="57" text-anchor="middle" class="t-faint">HTTPS</text>
	<text x="106" y="78" text-anchor="middle" class="t-faint">{m.deviceform_relay_out()}</text>
	<g transform="translate(138 64) rotate(180)">
		<circle r="2.4" class="packet packet--signal" style="--len: 42px; --delay: 0ms" />
		<circle r="2.4" class="packet packet--signal" style="--len: 42px; --delay: 800ms" />
	</g>

	<!-- The relay agent, with its mast -->
	<g>
		<line x1="172" y1="48" x2="172" y2="36" class="mast" />
		<path d="M166 34a8 8 0 0 1 12 0M162.5 30a13 13 0 0 1 19 0" class="waves" />
		<rect x="140" y="48" width="64" height="32" rx="6" class="unit unit--relay" />
		<text x="172" y="61.5" text-anchor="middle" class="t-strong">{m.deviceform_relay_relay()}</text>
		<text x="172" y="74" text-anchor="middle" class="t-soft">{m.deviceform_relay_agent()}</text>
	</g>

	<!-- Probes: agent → devices of the site -->
	{#each DEVICES as d (d.label)}
		<line x1="204" y1="64" x2="238" y2={d.y} class="link" marker-end={`url(#${uid}-head)`} />
		<g transform="translate(206 64) rotate({d.angle})">
			<circle r="2" class="packet" style="--len: {d.length}px; --delay: {300 + d.i * 260}ms" />
		</g>
		<rect x="244" y={d.y - 11} width="68" height="22" rx="5" class="unit" />
		<circle cx="253" cy={d.y} r="2.6" class="led" />
		<text x="261" y={d.y + 3.6} class="t-unit">{d.label}</text>
	{/each}
</svg>

<style>
	.relay-diagram {
		display: block;
		width: 100%;
		height: auto;
		overflow: visible;
		font-family: inherit;
	}
	.unit {
		fill: var(--c-surface);
		stroke: var(--c-line-strong);
		stroke-width: 1;
	}
	.unit--relay {
		stroke: var(--c-signal);
		stroke-width: 1.5;
	}
	.site {
		fill: var(--c-ghost);
		fill-opacity: 0.5;
		stroke: var(--c-line-strong);
		stroke-width: 1;
		stroke-dasharray: 4 3;
	}
	.led {
		fill: var(--c-signal);
	}
	.link {
		stroke: var(--c-ink-3);
		stroke-width: 1;
	}
	.link--signal {
		stroke: var(--c-signal);
		stroke-width: 1.6;
	}
	.head {
		fill: var(--c-ink-3);
	}
	.head--signal {
		fill: var(--c-signal);
	}
	.mast {
		stroke: var(--c-signal);
		stroke-width: 1.6;
		stroke-linecap: round;
	}
	.waves {
		fill: none;
		stroke: var(--c-signal);
		stroke-width: 1.4;
		stroke-linecap: round;
		animation: waves 2.4s ease-in-out infinite;
	}
	.t-strong {
		font-size: 10.5px;
		font-weight: 700;
		fill: var(--c-ink);
	}
	.t-soft {
		font-size: 9.5px;
		fill: var(--c-ink-2);
	}
	.t-unit {
		font-size: 9.5px;
		font-weight: 600;
		fill: var(--c-ink);
	}
	.t-tape {
		font-size: 8.5px;
		font-weight: 700;
		letter-spacing: 0.09em;
		text-transform: uppercase;
		fill: var(--c-ink-2);
	}
	.t-faint {
		font-size: 8.5px;
		fill: var(--c-ink-2);
	}
	.packet {
		fill: var(--c-ink-3);
		opacity: 0;
		animation: packet 1.6s linear infinite;
		animation-delay: var(--delay);
	}
	.packet--signal {
		fill: var(--c-signal);
	}
	@keyframes packet {
		0% {
			opacity: 0;
			transform: translateX(0);
		}
		15%,
		80% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translateX(var(--len));
		}
	}
	@keyframes waves {
		0%,
		100% {
			opacity: 0.35;
		}
		50% {
			opacity: 1;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.packet {
			animation: none;
			opacity: 0;
		}
		.waves {
			animation: none;
			opacity: 1;
		}
	}
</style>
