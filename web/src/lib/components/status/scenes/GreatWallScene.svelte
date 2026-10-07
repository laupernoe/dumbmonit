<script lang="ts">
	/**
	 * The Great Wall of China on a misty morning — the wall climbing a ridge
	 * with watchtowers, layered blue-grey mountains fading into haze.
	 * Static, no animation. Follows the scene contract: fills its positioned
	 * parent, the headline zone (top left) is left clear, the palette lives here.
	 */
</script>

<div class="scene scene-greatwall" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<linearGradient id="gw-mist" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--mist)" stop-opacity="0" />
				<stop offset="1" stop-color="var(--mist)" stop-opacity=".9" />
			</linearGradient>
			<g id="gw-tower">
				<rect x="-17" y="-52" width="34" height="52" fill="var(--wall)" />
				<rect x="-17" y="-52" width="12" height="52" fill="var(--wall2)" />
				<rect x="-21" y="-60" width="42" height="10" fill="var(--wall)" />
				<path d="M-21-60v-7h6v7zM-9-60v-7h6v7zM3-60v-7h6v7zM15-60v-7h6v7z" fill="var(--wall)" />
				<path d="M-8-38v14a5 5 0 0 0 10 0v-14a5 5 0 0 0-10 0z" fill="var(--dark)" transform="translate(6 0)" />
				<rect x="-21" y="-52" width="42" height="3" fill="var(--wall2)" />
			</g>
		</defs>
		<circle cx="1380" cy="150" r="44" fill="var(--sun)" opacity=".9" />
		<g fill="var(--cloud)" opacity=".7"><ellipse cx="900" cy="150" rx="80" ry="14" /><ellipse cx="1000" cy="138" rx="46" ry="12" /><ellipse cx="1650" cy="290" rx="70" ry="12" /></g>
		<!-- far ranges -->
		<path d="M560 400L640 350L720 392L810 300L890 372L970 330L1060 396L1150 320L1240 380L1330 300L1420 370L1520 290L1620 360L1700 310L1780 350V500H560Z" fill="var(--far1)" />
		<path d="M560 440L670 380L760 430L880 340L980 420L1090 362L1200 430L1330 350L1440 420L1560 340L1680 410L1780 370V500H560Z" fill="var(--far2)" />
		<rect x="560" y="330" width="1220" height="170" fill="url(#gw-mist)" />
		<!-- ridge carrying the wall -->
		<path d="M560 500V470L700 466L860 404L960 424L1080 336L1180 354L1300 258L1400 276L1520 206L1640 236L1780 196V500Z" fill="var(--ridge)" />
		<path d="M860 404L960 424L1080 336L1180 354L1300 258L1400 276L1520 206L1640 236L1780 196V250L1640 290L1520 260L1400 330L1300 312L1180 410L1080 392L960 480L860 460Z" fill="var(--ridge2)" opacity=".55" />
		<!-- wall -->
		<g stroke-linejoin="round" stroke-linecap="butt">
			<path d="M700 466L860 404L960 424L1080 336L1180 354L1300 258L1400 276L1520 206L1640 236L1780 196" stroke="var(--wall2)" stroke-width="22" transform="translate(0 4)" />
			<path d="M700 466L860 404L960 424L1080 336L1180 354L1300 258L1400 276L1520 206L1640 236L1780 196" stroke="var(--wall)" stroke-width="14" />
			<path d="M700 466L860 404L960 424L1080 336L1180 354L1300 258L1400 276L1520 206L1640 236L1780 196" stroke="var(--wall)" stroke-width="7" stroke-dasharray="7 6" transform="translate(0 -10)" />
		</g>
		<!-- towers -->
		<use href="#gw-tower" x="860" y="408" />
		<use href="#gw-tower" x="1080" y="340" />
		<use href="#gw-tower" x="1300" y="262" />
		<use href="#gw-tower" x="1520" y="210" />
		<use href="#gw-tower" x="1740" y="212" transform="translate(0 -4)" />
		<!-- near slope and trees -->
		<path d="M560 520V500C700 470 860 500 1000 490C1180 476 1360 500 1560 478C1660 468 1740 480 1780 474V620H560Z" fill="var(--near)" />
		<g fill="var(--tree)"><path d="M980 500l-14 0 7-34z" /><path d="M1010 506l-18 0 9-44z" /><path d="M1046 498l-12 0 6-30z" /><path d="M1400 492l-16 0 8-38z" /><path d="M1440 498l-20 0 10-46z" /><path d="M1480 490l-12 0 6-30z" /></g>
		<rect x="560" y="470" width="1220" height="150" fill="url(#gw-mist)" opacity=".45" />
	</svg>
</div>

<style>
	.scene-greatwall {
		--sky1: #cfe0e6;
		--sky2: #f2ead7;
		--sun: #fff0c4;
		--water1: #9fb59a;
		--water2: #6f8c6d;
		--far1: #bccbd2;
		--far2: #a6bbc0;
		--mist: #eef0e6;
		--ridge: #7f9a78;
		--ridge2: #5d7b5f;
		--wall: #cdbb97;
		--wall2: #a8946f;
		--dark: #4a4033;
		--near: #7a9768;
		--tree: #3f6648;
		--cloud: #fff;
	}
	:global(.dark) .scene-greatwall {
		--sky1: #0c1630;
		--sky2: #243050;
		--sun: #e8eef8;
		--water1: #1c3040;
		--water2: #0c1a26;
		--far1: #2c3a5c;
		--far2: #243050;
		--mist: #3a4a70;
		--ridge: #1f3a3f;
		--ridge2: #14292f;
		--wall: #8a8aa0;
		--wall2: #5b5d78;
		--dark: #f5c76a;
		--near: #122c30;
		--tree: #0b1c22;
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
