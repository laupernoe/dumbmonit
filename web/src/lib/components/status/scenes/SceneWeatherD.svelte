<script lang="ts">
	/**
	 * Weather shared by the landmark scenes, drawn inside a scene's SVG
	 * (viewBox 560 80 1220 540). Two layers: `sky` goes right after the sun
	 * and stars (cloud bank + a veil that dims them), `fx` goes last (a light
	 * veil over the whole picture, rain, lightning). ok = clear, warn = cloud,
	 * critical = storm; the aliases calm / clouded / storm are accepted.
	 * Reduced motion: everything is still, rain is a faint static texture.
	 */
	interface Props {
		part: 'sky' | 'fx';
		mood?: 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';
		/** x of the lightning bolt */
		boltX?: number;
	}
	let { part, mood = 'ok', boltX = 1480 }: Props = $props();
	const level = $derived(
		mood === 'warn' || mood === 'clouded' ? 1 : mood === 'critical' || mood === 'storm' ? 2 : 0
	);

	function rng(seed: number) {
		let s = seed;
		return () => {
			s = (s * 1664525 + 1013904223) % 4294967296;
			return s / 4294967296;
		};
	}
	const DROPS = (() => {
		const r = rng(7);
		let d = '';
		for (let i = 0; i < 110; i++) {
			d += `M${540 + Math.round(r() * 1300)} ${40 + Math.round(r() * 620)}l-8 22`;
		}
		return d;
	})();
	const CLOUDS: [number, number, number, number][] = [
		[760, 150, 150, 22], [900, 128, 110, 20], [1180, 112, 170, 24], [1450, 150, 160, 24],
		[1700, 124, 130, 20], [1010, 232, 130, 18], [1330, 210, 150, 20], [1600, 238, 120, 17],
		[700, 250, 110, 16]
	];
</script>

{#if part === 'sky'}
	<g class="wx l{level}">
		<g class="clouds">
			{#each CLOUDS as [x, y, rx, ry], i}
				<ellipse cx={x} cy={y} {rx} {ry} class="c{i % 3}" />
				<ellipse cx={x + rx * 0.25} cy={y - ry * 0.7} rx={rx * 0.6} ry={ry * 0.9} class="c{i % 3}" />
			{/each}
		</g>
		<rect class="skyveil" x="540" y="60" width="1260" height="430" />
	</g>
{:else}
	<g class="wx l{level}">
		<rect class="veil" x="540" y="60" width="1260" height="570" />
		<path class="rain" d={DROPS} />
		<path class="bolt" d="M{boltX} 80l-26 90 22-4-30 110 56-120-26 4z" />
		<rect class="flash" x="540" y="60" width="1260" height="570" />
	</g>
{/if}

<style>
	.clouds {
		fill: color-mix(in srgb, var(--sky2, #8896ab) 45%, #8c97a8);
		opacity: 0;
		transition: opacity 2s;
	}
	:global(.dark) .clouds {
		fill: color-mix(in srgb, var(--sky2, #2a2f4a) 70%, #3c4560);
	}
	.l1 .clouds {
		opacity: 0.8;
	}
	.l2 .clouds {
		opacity: 0.97;
		fill: color-mix(in srgb, var(--sky1, #445) 30%, #2b3140);
	}
	:global(.dark) .l2 .clouds {
		fill: #171b2c;
	}
	.skyveil {
		fill: #151b2c;
		opacity: 0;
		transition: opacity 2s;
	}
	.l1 .skyveil {
		opacity: 0.12;
	}
	.l2 .skyveil {
		opacity: 0.42;
	}
	.veil {
		fill: #0b1020;
		opacity: 0;
		transition: opacity 2s;
	}
	.l1 .veil {
		opacity: 0.05;
	}
	.l2 .veil {
		opacity: 0.2;
	}
	.rain {
		stroke: #dbe5ff;
		stroke-width: 1.6;
		stroke-linecap: round;
		opacity: 0;
		transition: opacity 2s;
	}
	.l2 .rain {
		opacity: 0.4;
	}
	.bolt {
		fill: #fff;
		opacity: 0;
	}
	.flash {
		fill: #dfe6ff;
		opacity: 0;
	}
	@media (prefers-reduced-motion: no-preference) {
		.clouds {
			animation: drift 40s ease-in-out infinite alternate;
		}
		.l2 .rain {
			animation: fall 0.7s linear infinite;
		}
		.l2 .flash {
			animation: flash 8s linear infinite;
		}
		.l2 .bolt {
			animation: bolt 8s linear infinite;
		}
	}
	@keyframes drift {
		to {
			transform: translateX(46px);
		}
	}
	@keyframes fall {
		to {
			transform: translate(-16px, 44px);
		}
	}
	@keyframes flash {
		0%, 91%, 100% { opacity: 0; } 92% { opacity: 0.5; } 93% { opacity: 0.05; } 94% { opacity: 0.38; } 96% { opacity: 0; }
	}
	@keyframes bolt {
		0%, 91.5%, 94.5%, 100% { opacity: 0; } 92%, 94% { opacity: 0.95; }
	}
</style>
