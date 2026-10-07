<script lang="ts">
	/**
	 * The Golden Gate Bridge in the fog — both Art Deco towers in international
	 * orange, the suspension cables, Marin headlands and a fog bank rolling in.
	 * Static, no animation. Follows the scene contract: fills its positioned
	 * parent, the headline zone (top left) is left clear, the palette lives here.
	 */
	const t1 = 1010;
	const t2 = 1410;
	const topY = 234;
	const deck = 436;
	const sag = 400; // lowest point of the main cable
	const mid = (t1 + t2) / 2;
	const k = (sag - topY) / ((t2 - t1) / 2) ** 2;
	const cable = (x: number) => sag - (x - mid) ** 2 * k;
	const hangers = Array.from({ length: 31 }, (_, i) => t1 + 10 + i * ((t2 - t1 - 20) / 30));
	const mainPts = Array.from({ length: 41 }, (_, i) => {
		const x = t1 + (i * (t2 - t1)) / 40;
		return `${x.toFixed(1)},${cable(x).toFixed(1)}`;
	}).join(' ');
	// side spans: straight-ish curves from the tower tops to the anchorages
	const leftX = 780;
	const rightX = 1640;
	const sideHangers = (from: number, to: number, n: number) =>
		Array.from({ length: n }, (_, i) => {
			const f = (i + 1) / (n + 1);
			const x = from + (to - from) * f;
			const y = topY + (deck + 6 - topY) * f ** 2;
			return { x, y };
		});
	const left = sideHangers(t1, leftX, 12);
	const right = sideHangers(t2, rightX, 10);
</script>

<div class="scene scene-sanfrancisco" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<linearGradient id="sf-wat" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--water1)" />
				<stop offset="1" stop-color="var(--water2)" />
			</linearGradient>
			<linearGradient id="sf-fog" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--fog)" stop-opacity="0" />
				<stop offset=".5" stop-color="var(--fog)" stop-opacity=".85" />
				<stop offset="1" stop-color="var(--fog)" stop-opacity="0" />
			</linearGradient>
			<g id="sf-tower">
				<path d="M-34 250V0h14V250Z M20 250V0h14V250Z" fill="var(--orange)" />
				<path d="M-34 0L-30 -10H-20V0Z M20 0L24 -10H34V0Z" fill="var(--orange)" />
				<rect x="-36" y="-14" width="72" height="12" fill="var(--orange)" />
				<rect x="-30" y="-24" width="60" height="10" fill="var(--orange)" />
				<rect x="-34" y="46" width="68" height="10" fill="var(--orange)" />
				<rect x="-34" y="104" width="68" height="10" fill="var(--orange)" />
				<rect x="-34" y="162" width="68" height="10" fill="var(--orange)" />
				<path d="M-20 14H20L20 46H-20Z M-20 56H20V104H-20Z" fill="var(--orange2)" opacity=".55" />
				<path d="M-20 114H20V162H-20Z" fill="var(--orange2)" opacity=".4" />
				<rect x="-36" y="212" width="72" height="8" fill="var(--orange2)" />
			</g>
		</defs>
		<circle cx="1180" cy="180" r="40" fill="var(--sun)" opacity=".85" />
		<g fill="var(--cloud)" opacity=".7">
			<ellipse cx="800" cy="170" rx="80" ry="12" />
			<ellipse cx="1600" cy="160" rx="70" ry="10" />
		</g>
		<!-- Marin headlands -->
		<path d="M1480 486C1520 420 1580 380 1660 366C1710 358 1760 372 1800 390V490H1480Z" fill="var(--hill2)" />
		<path d="M1560 490C1620 440 1690 430 1800 436V490Z" fill="var(--hill)" />
		<!-- San Francisco side -->
		<path d="M540 490C640 462 720 448 800 446V490Z" fill="var(--hill)" />
		<!-- water -->
		<rect x="540" y="484" width="1260" height="160" fill="url(#sf-wat)" />
		<g stroke="var(--wh)" stroke-width="2" opacity=".45" stroke-linecap="round">
			<path d="M620 530h70M800 548h100M1180 530h90M1330 560h110M1500 538h80M1640 560h90M720 586h80M940 576h120" />
		</g>
		<!-- side spans: cables and hangers -->
		<g stroke="var(--orange)" stroke-width="3.5" stroke-linecap="round">
			<path d="M{t1} {topY}Q{(t1 + leftX) / 2 + 30} {deck + 6} {leftX} {deck + 6}" />
			<path d="M{t2} {topY}Q{(t2 + rightX) / 2 - 30} {deck + 6} {rightX} {deck + 6}" />
		</g>
		<g stroke="var(--orange)" stroke-width="1.4" opacity=".85">
			{#each left as h}
				<line x1={h.x} y1={h.y + 12} x2={h.x} y2={deck} />
			{/each}
			{#each right as h}
				<line x1={h.x} y1={h.y + 12} x2={h.x} y2={deck} />
			{/each}
		</g>
		<!-- main span -->
		<g>
			<g stroke="var(--orange)" stroke-width="1.6" opacity=".9">
				{#each hangers as x}
					<line x1={x} y1={cable(x)} x2={x} y2={deck} />
				{/each}
			</g>
			<polyline points={mainPts} stroke="var(--orange)" stroke-width="4" stroke-linejoin="round" />
		</g>
		<!-- deck -->
		<rect x={leftX} y={deck} width={rightX - leftX} height="9" fill="var(--orange)" />
		<rect x={leftX} y={deck + 9} width={rightX - leftX} height="4" fill="var(--orange2)" />
		<g stroke="var(--orange2)" stroke-width="1.2" opacity=".6">
			{#each hangers as x}
				<line x1={x} y1={deck + 9} x2={x + 6} y2={deck + 13} />
			{/each}
		</g>
		<!-- towers -->
		<use href="#sf-tower" transform="translate({t1} {topY + 18})" />
		<use href="#sf-tower" transform="translate({t2} {topY + 18})" />
		<!-- deck over tower legs -->
		<rect x={t1 - 34} y={deck - 2} width="68" height="9" fill="var(--orange)" />
		<rect x={t2 - 34} y={deck - 2} width="68" height="9" fill="var(--orange)" />
		<!-- fog banks -->
		<g>
			<rect x="540" y="400" width="1260" height="110" fill="url(#sf-fog)" opacity=".9" />
			<g fill="var(--fog)" opacity=".75">
				<ellipse cx="860" cy="470" rx="220" ry="26" />
				<ellipse cx="1210" cy="486" rx="300" ry="22" />
				<ellipse cx="1560" cy="466" rx="200" ry="24" />
				<ellipse cx="1000" cy="300" rx="80" ry="9" opacity=".6" />
				<ellipse cx="1480" cy="280" rx="100" ry="10" opacity=".55" />
			</g>
			<g fill="var(--fog)" opacity=".5">
				<ellipse cx="1230" cy="520" rx="260" ry="14" />
				<ellipse cx="800" cy="520" rx="180" ry="10" />
			</g>
		</g>
		<!-- sailboat -->
		<g transform="translate(1180 536)">
			<path d="M0 0h50c-3 8-9 11-16 11H12C6 11 2 6 0 0Z" fill="var(--boat)" />
			<path d="M24 -2V-42L44 -2Z" fill="var(--wh)" />
			<path d="M22 -2V-34L8 -2Z" fill="var(--wh)" opacity=".85" />
		</g>
		<!-- status buoy -->
		<g>
			<circle cx="920" cy="574" r="9" fill="var(--scene-tone, var(--c-signal))" />
			<circle cx="920" cy="574" r="22" fill="var(--scene-tone, var(--c-signal))" opacity=".25" />
			<rect x="916" y="582" width="8" height="10" fill="var(--orange2)" />
		</g>
	</svg>
</div>

<style>
	.scene-sanfrancisco {
		--sky1: #e9c9bd;
		--sky2: #f6e6dd;
		--sun: #fff0cf;
		--water1: #8fb0bc;
		--water2: #587f96;
		--orange: #d4492b;
		--orange2: #a8351f;
		--hill: #7e9a73;
		--hill2: #9fb59a;
		--fog: #fbf6f2;
		--boat: #3a4a63;
		--wh: #fff;
		--cloud: #fff;
	}
	:global(.dark) .scene-sanfrancisco {
		--sky1: #0d1630;
		--sky2: #1f2a4a;
		--sun: #dfe6f4;
		--water1: #16294a;
		--water2: #0a1428;
		--orange: #c4503a;
		--orange2: #8d3a2e;
		--hill: #1d3a3a;
		--hill2: #2a4a4c;
		--fog: #5a6584;
		--boat: #1d2640;
		--wh: #8b92a8;
		--cloud: #2a3658;
	}
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(
			180deg,
			var(--sky1),
			var(--sky2) 73.5%,
			var(--water1) 74.4%,
			var(--water1) 80%,
			var(--water2)
		);
	}
	svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		-webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
		mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
	}
</style>
