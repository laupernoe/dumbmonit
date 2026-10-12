<script lang="ts">
	/**
	 * Cats run the server room. Calm: one naps on a warm rack, one sleeps in a
	 * box, one bats a dangling cable. `mood` warn: clouds outside, a cat sits on
	 * the keyboard (gibberish on the monitor). Critical: storm, puffed tails, a
	 * chewed cable spitting sparks.
	 * `pulse` plays one gesture: down = the shelf cat slowly pushes a server off
	 * the shelf while staring at you (clonk); up = it puts it back, very proud,
	 * ears up; resolved = every cat curls up in a box and naps.
	 */
	/**
	 * Shared by the status page banner (`status/scenes/CatsScene`) and the wall
	 * backdrop (`wall/cats/CatsScene`). Fills its positioned parent; the art is
	 * drawn on a 1100 x 540 sheet (`view` re-frames it).
	 */
	interface Props {
		/** ok/warn/critical, or the wall's calm/clouded/storm. */
		mood?: 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
		/** Colour set: follow the page's `.dark` class, or force one (wall). */
		tone?: 'auto' | 'light' | 'dark' | 'oled';
		/** Time of day seen through the window. */
		sky?: 'day' | 'golden' | 'dusk' | 'night';
		view?: string;
		/** Fade the left and right edges into the page (banner). */
		fade?: boolean;
		paused?: boolean;
		/** Everyone is startled: the floor cat jumps, the box cat peeks. */
		startled?: boolean;
		/** Text of the paper sign hung from the cable tray, when there is trouble. */
		sign?: string | null;
	}
	let {
		mood = 'ok',
		pulse = null,
		tone = 'auto',
		sky = 'day',
		view = '0 0 1100 540',
		fade = false,
		paused = false,
		startled = false,
		sign = null
	}: Props = $props();

	const level = $derived(
		mood === 'warn' || mood === 'clouded' ? 'warn' : mood === 'critical' || mood === 'storm' ? 'crit' : 'ok'
	);
	const signLen = $derived(sign ? Math.min(112, sign.length * 9) : 0);

	const RACKS = [390, 490, 590, 690];
	const UNITS = [0, 1, 2, 3, 4, 5, 6];
</script>

{#snippet zz(x: number, y: number, cls: string)}
	<g class="zzg {cls}">
		<text class="zz z1" {x} {y} font-size="15">z</text>
		<text class="zz z2" x={x + 9} y={y - 12} font-size="19">z</text>
		<text class="zz z3" x={x + 19} y={y - 26} font-size="24">Z</text>
	</g>
{/snippet}

<div
	class="scene scene-cats"
	class:m-warn={level === 'warn'}
	class:m-crit={level === 'crit'}
	class:tone-auto={tone === 'auto'}
	class:tone-light={tone === 'light'}
	class:tone-dark={tone === 'dark'}
	class:tone-oled={tone === 'oled'}
	class:s-golden={sky === 'golden'}
	class:s-dusk={sky === 'dusk'}
	class:s-night={sky === 'night'}
	class:faded={fade}
	class:paused
	class:startled
	aria-hidden="true"
>
	<svg viewBox={view} preserveAspectRatio="xMidYMax slice" fill="none">
		<defs>
			<clipPath id="ct-win"><rect x="880" y="40" width="170" height="210" rx="6" /></clipPath>
			<g id="ct-ears">
				<path d="M-19-8L-22-31L-4-17zM19-8L22-31L4-17z" fill="var(--cf)" stroke="var(--ink)" stroke-width="2.4" stroke-linejoin="round" />
				<path d="M-17-13L-18-24L-9-17zM17-13L18-24L9-17z" fill="var(--pink)" />
			</g>
			<g id="ct-face">
				<ellipse rx="23" ry="19" fill="var(--cf)" stroke="var(--ink)" stroke-width="2.4" />
				<path d="M-4 5h8l-4 4z" fill="var(--pink)" />
				<path d="M0 9q-4 5-9 2M0 9q4 5 9 2M-14 5l-14-2M-14 8l-13 4M14 5l14-2M14 8l13 4" stroke="var(--ink)" stroke-width="1.5" stroke-linecap="round" />
			</g>
			<g id="ct-base"><use href="#ct-ears" /><use href="#ct-face" /></g>
			<g id="ct-eo">
				<ellipse cx="-8" cy="-3" rx="5" ry="6" fill="#fff" stroke="var(--ink)" stroke-width="1.4" /><ellipse cx="8" cy="-3" rx="5" ry="6" fill="#fff" stroke="var(--ink)" stroke-width="1.4" />
				<circle cx="-8" cy="-2" r="3" fill="var(--ink)" /><circle cx="8" cy="-2" r="3" fill="var(--ink)" /><circle cx="-7" cy="-4" r="1.1" fill="#fff" /><circle cx="9" cy="-4" r="1.1" fill="#fff" />
			</g>
			<g id="ct-es"><path d="M-13-2q5 5 10 0M3-2q5 5 10 0" stroke="var(--ink)" stroke-width="2" stroke-linecap="round" /></g>
			<g id="ct-ew">
				<ellipse cx="-8" cy="-3" rx="6.5" ry="8" fill="#fff" stroke="var(--ink)" stroke-width="1.4" /><ellipse cx="8" cy="-3" rx="6.5" ry="8" fill="#fff" stroke="var(--ink)" stroke-width="1.4" />
				<circle cx="-8" cy="-3" r="2" fill="var(--ink)" /><circle cx="8" cy="-3" r="2" fill="var(--ink)" />
				<path d="M-16-13l11 5M16-13l-11 5" stroke="var(--ink)" stroke-width="2.4" stroke-linecap="round" />
			</g>
			<!-- chat en boule : origine = centre bas -->
			<g id="ct-loaf">
				<path d="M36-14q26 0 22-22" stroke="var(--ink)" stroke-width="10" stroke-linecap="round" />
				<path d="M36-14q26 0 22-22" stroke="var(--cf)" stroke-width="6" stroke-linecap="round" />
				<ellipse cx="2" cy="-20" rx="42" ry="22" fill="var(--cf)" stroke="var(--ink)" stroke-width="2.4" />
				<path d="M10-40q2 8 0 14M22-38q2 7 0 12" stroke="var(--cs)" stroke-width="3" stroke-linecap="round" />
				<g transform="translate(-30 -22) scale(.85)"><use href="#ct-base" /><use href="#ct-es" /></g>
				<ellipse cx="-14" cy="-3" rx="9" ry="5" fill="var(--cf)" stroke="var(--ink)" stroke-width="2" />
			</g>
			<!-- avant d'une petite boîte en carton : origine = centre bas, 70 x 26 -->
			<g id="ct-mbox">
				<rect x="-35" y="-26" width="70" height="26" rx="2" fill="var(--box)" stroke="var(--ink)" stroke-width="2" />
				<rect x="-35" y="-26" width="70" height="6" fill="var(--box2)" stroke="var(--ink)" stroke-width="2" />
				<path d="M-6-20v8h12v-8" fill="var(--box2)" />
			</g>
			<linearGradient id="ct-sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--sky1)" /><stop offset="1" stop-color="var(--sky2)" /></linearGradient>
		</defs>

		<!-- mur, sol -->
		<rect x="-2000" y="-1200" width="5200" height="1648" fill="var(--wall)" />
		<rect x="-2000" y="420" width="5200" height="28" fill="var(--wall2)" />
		<rect x="-2000" y="448" width="5200" height="300" fill="var(--floor)" />
		<g stroke="var(--tile)" stroke-width="2"><path d="M-2000 448H3200M-2000 492H3200M-2000 536H3200" /><path d="M60 448L0 540M200 448L170 540M340 448L330 540M480 448L480 540M620 448L630 540M760 448L790 540M900 448L950 540M1040 448L1110 540M-80 448L-200 540M-220 448L-340 540" /></g>
		<!-- chemin de câbles au plafond -->
		<rect x="-2000" y="62" width="5200" height="12" fill="var(--rack2)" stroke="var(--ink)" stroke-width="2" />
		<g class="sign" style="--sl:{signLen}px">
			<path d="M262 74V108M366 74V108" stroke="var(--ink)" stroke-width="2" />
			{#if sign}
				<rect x="236" y="108" width="156" height="40" rx="3" fill="var(--paper)" stroke="var(--ink)" stroke-width="2.4" />
				<text x="314" y="134" font-size="17" font-weight="900" fill="var(--led-r)" text-anchor="middle" textLength={signLen} lengthAdjust="spacingAndGlyphs">{sign}</text>
			{/if}
		</g>
		<path d="M0 66Q200 90 420 66T820 68T1100 64" stroke="var(--cable)" stroke-width="4" stroke-linecap="round" />
		<path d="M0 70Q260 52 520 72T1100 70" stroke="var(--cable2)" stroke-width="3" stroke-linecap="round" />
		<!-- poster : le poisson du mois -->
		<g transform="translate(320 196)">
			<rect width="62" height="82" rx="3" fill="var(--paper)" stroke="var(--ink)" stroke-width="2.4" />
			<path d="M12 44q16-18 34 0q-18 18-34 0zM46 44l8-8v16z" fill="var(--cable2)" stroke="var(--ink)" stroke-width="2" stroke-linejoin="round" /><circle cx="23" cy="42" r="2" fill="var(--ink)" />
			<path d="M12 66h38M18 72h26" stroke="var(--ink)" stroke-width="2" stroke-linecap="round" opacity=".5" />
		</g>

		<!-- fenêtre : météo -->
		<rect x="874" y="34" width="182" height="222" rx="9" fill="var(--rack2)" stroke="var(--ink)" stroke-width="3" />
		<g clip-path="url(#ct-win)">
			<rect x="880" y="40" width="170" height="210" fill="url(#ct-sky)" />
			<g class="stars" fill="#fff"><circle cx="900" cy="70" r="1.6" /><circle cx="940" cy="110" r="1.2" /><circle cx="1010" cy="62" r="1.8" /><circle cx="1030" cy="120" r="1.3" /><circle cx="920" cy="190" r="1.4" /><circle cx="1000" cy="200" r="1.6" /></g>
			<circle class="sun" cx="968" cy="110" r="26" fill="var(--sun)" />
			<g class="clouds-w" fill="var(--cloud)">
				<g class="cl cl1"><ellipse cx="930" cy="96" rx="48" ry="15" /><ellipse cx="958" cy="82" rx="30" ry="15" /></g>
				<g class="cl cl2"><ellipse cx="1010" cy="150" rx="44" ry="13" /><ellipse cx="990" cy="138" rx="26" ry="13" /></g>
			</g>
			<g class="storm" fill="var(--stormcl)"><ellipse cx="925" cy="72" rx="64" ry="24" /><ellipse cx="1000" cy="64" rx="66" ry="26" /><ellipse cx="965" cy="100" rx="70" ry="18" /></g>
			<g class="rain" stroke="var(--rain)" stroke-width="1.6" stroke-linecap="round">
				<path d="M896 120l-6 16M920 140l-6 16M944 118l-6 16M968 148l-6 16M992 122l-6 16M1016 142l-6 16M1038 118l-6 16M908 176l-6 16M956 186l-6 16M1004 180l-6 16M1030 196l-6 16" />
			</g>
			<path class="bolt" d="M978 96l-14 40h12l-8 34 24-46h-13l9-28z" fill="#fff6b0" stroke="#f2c230" stroke-width="1.5" stroke-linejoin="round" />
			<rect class="flash-w" x="880" y="40" width="170" height="210" fill="#fff" />
		</g>
		<path d="M965 40V250M880 145H1050" stroke="var(--rack2)" stroke-width="5" />
		<rect x="874" y="34" width="182" height="222" rx="9" stroke="var(--ink)" stroke-width="3" />

		<!-- racks -->
		{#each RACKS as x, ri (x)}
			<g transform="translate({x} 170)">
				<rect width="90" height="300" rx="4" fill="var(--rack)" stroke="var(--ink)" stroke-width="3" />
				{#each UNITS as u (u)}
					<rect x="6" y={10 + u * 40} width="78" height="34" rx="3" fill="var(--rack2)" stroke="var(--ink)" stroke-width="1.6" />
					<path d="M36 {20 + u * 40}h40M36 {26 + u * 40}h40M36 {32 + u * 40}h40" stroke="var(--vent)" stroke-width="2" stroke-linecap="round" />
					<circle class="led {(u + ri) % 3 === 0 ? 'x' : (u + ri) % 2 ? 'a' : 'g'}" style="--d:{((u * 7 + ri * 3) % 11) / 5}s" cx="16" cy={20 + u * 40} r="3" />
					<circle class="led {(u * 2 + ri) % 4 === 0 ? 'a' : 'g'}" style="--d:{((u * 5 + ri * 2) % 9) / 4}s" cx="16" cy={30 + u * 40} r="3" />
				{/each}
			</g>
		{/each}
		<!-- chaleur au-dessus du rack 2 -->
		<g class="heat" stroke="var(--heat)" stroke-width="3" stroke-linecap="round"><path d="M510 150q-6-10 0-20t0-20" /><path d="M540 150q-6-10 0-20t0-20" /><path d="M570 150q-6-10 0-20t0-20" /></g>
		<use href="#dm-mascot" transform="translate(728 140) scale(.1)" />

		<!-- câbles au sol -->
		<path d="M300 486Q340 470 380 484T470 488T600 482T770 490" stroke="var(--cable)" stroke-width="5" stroke-linecap="round" />
		<path d="M320 494Q380 508 460 496T640 498" stroke="var(--cable2)" stroke-width="4" stroke-linecap="round" />

		<!-- chat endormi sur le rack 2 (chaud) -->
		<g transform="translate(535 170) scale(.95)" style="--cf:var(--cat-o);--cs:var(--cat-od)">
			<g class="breath">
				<use href="#ct-loaf" />
				<g class="crit-only"><path d="M52-30q14 6 14 28t-4 44" stroke="var(--ink)" stroke-width="20" stroke-linecap="round" /><path d="M52-30q14 6 14 28t-4 44" stroke="var(--cf)" stroke-width="15" stroke-linecap="round" /><path d="M58-26l8-4M64-14l9-1M66 0l9 3M64 14l9 5M62 28l8 7" stroke="var(--ink)" stroke-width="2.2" stroke-linecap="round" /></g>
			</g>
		</g>
		{@render zz(496, 128, 'zz-rest')}

		<!-- bureau, écran, clavier -->
		<rect x="868" y="378" width="196" height="10" rx="2" fill="var(--box2)" stroke="var(--ink)" stroke-width="2.4" />
		<path d="M880 388v100M1052 388v100" stroke="var(--ink)" stroke-width="5" />
		<g>
			<rect x="882" y="294" width="74" height="58" rx="4" fill="var(--rack)" stroke="var(--ink)" stroke-width="2.6" />
			<rect x="887" y="299" width="64" height="46" rx="2" fill="var(--screen)" />
			<path d="M912 352l-4 26h24l-4-26z" fill="var(--rack2)" stroke="var(--ink)" stroke-width="2" />
			<g class="scr-ok" stroke="var(--led-g)" stroke-width="2.4" stroke-linecap="round"><path d="M893 308h30M893 316h44M893 324h22M893 332h38" /></g>
			<g class="scr-warn" font-family="ui-monospace, monospace" font-size="7.5" font-weight="700" fill="var(--led-a)"><text class="gib g1" x="890" y="309">asdfj;lkq</text><text class="gib g2" x="890" y="319">wpoe;;;ll</text><text class="gib g3" x="890" y="329">zzxcvvbnn</text><text class="gib g1" x="890" y="339">mmm,,,,..</text></g>
			<g class="scr-crit" font-family="ui-monospace, monospace" font-weight="800" fill="var(--led-r)"><text x="896" y="319" font-size="11">!! ERR !!</text><path d="M893 330h52M893 336h36" stroke="var(--led-r)" stroke-width="2.4" stroke-linecap="round" /></g>
		</g>
		<rect x="924" y="364" width="88" height="13" rx="3" fill="var(--rack2)" stroke="var(--ink)" stroke-width="2" />
		<g class="keys" fill="var(--paper)"><rect class="k1" x="930" y="367" width="9" height="6" /><rect class="k2" x="942" y="367" width="9" height="6" /><rect class="k3" x="954" y="367" width="9" height="6" /><rect class="k1" x="966" y="367" width="9" height="6" /><rect class="k2" x="978" y="367" width="9" height="6" /><rect class="k3" x="990" y="367" width="9" height="6" /></g>
		<!-- chat sur le clavier -->
		<g class="kb-cat" style="--cf:var(--cat-g);--cs:var(--cat-gd)">
			<path d="M1010 372q22 0 18-24" stroke="var(--ink)" stroke-width="9" stroke-linecap="round" /><path d="M1010 372q22 0 18-24" stroke="var(--cf)" stroke-width="5" stroke-linecap="round" />
			<ellipse cx="978" cy="352" rx="28" ry="24" fill="var(--cf)" stroke="var(--ink)" stroke-width="2.4" />
			<g transform="translate(978 322)"><use href="#ct-base" /><use href="#ct-eo" /></g>
			<ellipse class="pawl" cx="962" cy="372" rx="10" ry="6" fill="var(--cf)" stroke="var(--ink)" stroke-width="2" /><ellipse class="pawr" cx="990" cy="372" rx="10" ry="6" fill="var(--cf)" stroke="var(--ink)" stroke-width="2" />
		</g>

		<!-- câble rongé et étincelles (critique) -->
		<g class="chewed">
			<path d="M780 330q4 40 -4 70" stroke="var(--ink)" stroke-width="8" stroke-linecap="round" /><path d="M780 330q4 40 -4 70" stroke="var(--cable)" stroke-width="5" stroke-linecap="round" />
			<path d="M776 400l-5 8M776 400l2 10M776 400l7 6" stroke="var(--cable2)" stroke-width="2.4" stroke-linecap="round" />
			<path d="M822 470q-14 6-28 2" stroke="var(--ink)" stroke-width="8" stroke-linecap="round" /><path d="M822 470q-14 6-28 2" stroke="var(--cable)" stroke-width="5" stroke-linecap="round" />
			<path d="M794 472l-8-4M794 472l-7 3M794 472l-6 8" stroke="var(--cable2)" stroke-width="2.4" stroke-linecap="round" />
			<g class="sparks" fill="#ffd84a" stroke="#fff3a0" stroke-width="1">
				<path class="sp s1" d="M778 408l3 7 7 2-7 3-3 7-3-7-7-3 7-2z" /><path class="sp s2" d="M790 466l2 5 5 1-5 2-2 5-2-5-5-2 5-1z" />
				<path class="sp s3" d="M768 420l2 4 4 1-4 1-2 4-2-4-4-1 4-1z" />
			</g>
		</g>

		<!-- câble qui pend (calme / alerte) -->
		<g class="cable-play">
			<g class="cable-sw">
				<path d="M780 330Q776 390 802 436" stroke="var(--ink)" stroke-width="8" stroke-linecap="round" /><path d="M780 330Q776 390 802 436" stroke="var(--cable)" stroke-width="5" stroke-linecap="round" />
				<rect x="796" y="432" width="12" height="16" rx="2" transform="rotate(-24 802 440)" fill="var(--rack2)" stroke="var(--ink)" stroke-width="2" />
			</g>
		</g>

		{#key pulse?.at}
			<g class="gest" class:g-down={pulse?.kind === 'down'} class:g-up={pulse?.kind === 'up'} class:g-res={pulse?.kind === 'resolved'}>
				<!-- étagère, serveur et chat poussant -->
				<path d="M70 262H300M90 262v26M280 262v26" stroke="var(--ink)" stroke-width="9" stroke-linecap="round" />
				<path d="M70 262H300" stroke="var(--box)" stroke-width="5" stroke-linecap="round" />
				<g transform="translate(206 262)">
					<g class="srv">
						<rect y="-40" width="92" height="40" rx="3" fill="var(--rack)" stroke="var(--ink)" stroke-width="2.6" />
						<rect x="5" y="-35" width="82" height="30" rx="2" fill="var(--rack2)" stroke="var(--ink)" stroke-width="1.4" />
						<path d="M34-26h46M34-20h46M34-14h46" stroke="var(--vent)" stroke-width="2" stroke-linecap="round" />
						<circle cx="14" cy="-26" r="3" fill="var(--led-g)" /><circle cx="14" cy="-16" r="3" fill="var(--led-a)" />
					</g>
				</g>
				<g class="shelf-cat" style="--cf:var(--cat-w);--cs:var(--cat-wd)">
					<g transform="translate(140 262)">
						<g class="awake">
							<g class="tail-s"><path d="M-26-14q-34 0-30-28" stroke="var(--ink)" stroke-width="11" stroke-linecap="round" /><path d="M-26-14q-34 0-30-28" stroke="var(--cf)" stroke-width="7" stroke-linecap="round" /></g>
							<ellipse cx="0" cy="-26" rx="30" ry="26" fill="var(--cf)" stroke="var(--ink)" stroke-width="2.4" />
							<g transform="translate(0 -64)">
								<g class="head">
									<g class="ears"><use href="#ct-ears" /></g><use href="#ct-face" /><use href="#ct-eo" />
									<path class="smug" d="M-4 9q4 4 8 0" stroke="var(--ink)" stroke-width="1.6" stroke-linecap="round" />
								</g>
							</g>
							<ellipse cx="-12" cy="-4" rx="11" ry="6" fill="var(--cf)" stroke="var(--ink)" stroke-width="2" />
							<ellipse class="paw" cx="22" cy="-22" rx="14" ry="7" fill="var(--cf)" stroke="var(--ink)" stroke-width="2" />
						</g>
						<g class="nap"><g transform="translate(0 -12) scale(.6)"><use href="#ct-loaf" /></g><use href="#ct-mbox" /></g>
					</g>
				</g>
				<text class="clonk" x="372" y="470" font-size="30" font-weight="900" font-style="italic" fill="var(--clonk)" stroke="var(--ink)" stroke-width="1.6" paint-order="stroke" text-anchor="middle">CLONK!</text>
				<g class="impact" stroke="var(--clonk)" stroke-width="3" stroke-linecap="round"><path d="M330 466l-16-6M340 452l-10-12M404 452l10-12M414 466l16-6" /></g>
				<g class="glint" fill="#fff6b0" stroke="var(--ink)" stroke-width="1"><path d="M196 214l3 8 8 3-8 3-3 8-3-8-8-3 8-3z" /><path d="M300 224l2 6 6 2-6 2-2 6-2-6-6-2 6-2z" /></g>
				{@render zz(158, 196, 'zz-nap')}

				<!-- chat dans la boîte -->
				<g transform="translate(145 500)" style="--cf:var(--cat-b);--cs:var(--cat-bd)">
					<g class="box-sleep"><g class="breath"><g transform="translate(0 -18) scale(.9)"><use href="#ct-loaf" /></g></g></g>
					<g class="box-alarm"><g transform="translate(0 -52)"><use href="#ct-base" /><use href="#ct-ew" /></g><path d="M34-26q18-4 20-26" stroke="var(--ink)" stroke-width="18" stroke-linecap="round" /><path d="M34-26q18-4 20-26" stroke="var(--cf)" stroke-width="13" stroke-linecap="round" /><path d="M42-44l-8-3M52-52l-9-1M56-62l-8 2" stroke="var(--ink)" stroke-width="2.2" stroke-linecap="round" /></g>
					<rect x="-50" y="-32" width="100" height="34" rx="2" fill="var(--box)" stroke="var(--ink)" stroke-width="2.4" />
					<path d="M-14-32v13h28v-13" fill="var(--box2)" stroke="var(--ink)" stroke-width="1.8" />
					<path d="M-50-32l-10-12h26zM50-32l10-12h-26z" fill="var(--box2)" stroke="var(--ink)" stroke-width="2" stroke-linejoin="round" />
				</g>
				{@render zz(150, 440, 'zz-rest')}

				<!-- chat au sol : joue avec le câble, ou fait le dos rond -->
				<g class="floor-cat" style="--cf:var(--cat-o);--cs:var(--cat-od)">
					<g class="awake">
						<g class="calm-v" transform="translate(850 488)">
							<g class="tail-f"><path d="M26-8q30 0 28-26" stroke="var(--ink)" stroke-width="11" stroke-linecap="round" /><path d="M26-8q30 0 28-26" stroke="var(--cf)" stroke-width="7" stroke-linecap="round" /></g>
							<ellipse cx="0" cy="-26" rx="26" ry="28" fill="var(--cf)" stroke="var(--ink)" stroke-width="2.4" />
							<g transform="translate(-12 -62) rotate(-8)"><use href="#ct-base" /><use href="#ct-eo" /></g>
							<ellipse cx="14" cy="-3" rx="11" ry="6" fill="var(--cf)" stroke="var(--ink)" stroke-width="2" />
							<g class="bat"><ellipse cx="-24" cy="-34" rx="13" ry="6.5" fill="var(--cf)" stroke="var(--ink)" stroke-width="2" /></g>
						</g>
						<g class="crit-v" transform="translate(846 488)">
							<g class="tail-p"><path d="M30-30q14-8 18-30" stroke="var(--ink)" stroke-width="22" stroke-linecap="round" /><path d="M30-30q14-8 18-30" stroke="var(--cf)" stroke-width="17" stroke-linecap="round" /><path d="M36-44l10-5M44-52l9-1M48-62l8 4M42-36l10 0" stroke="var(--ink)" stroke-width="2.2" stroke-linecap="round" /></g>
							<path d="M-30-8Q-28-60 0-62Q28-60 30-8z" fill="var(--cf)" stroke="var(--ink)" stroke-width="2.4" />
							<path d="M-20-58l-4-8M-8-62l-2-9M6-62l1-9M18-58l5-7" stroke="var(--ink)" stroke-width="2.4" stroke-linecap="round" />
							<g transform="translate(-26 -38) rotate(-14) scale(.95)"><use href="#ct-base" /><use href="#ct-ew" /></g>
							<ellipse cx="-14" cy="-3" rx="10" ry="5" fill="var(--cf)" stroke="var(--ink)" stroke-width="2" /><ellipse cx="14" cy="-3" rx="10" ry="5" fill="var(--cf)" stroke="var(--ink)" stroke-width="2" />
						</g>
					</g>
					<g transform="translate(850 496)"><g class="nap"><g transform="translate(0 -12) scale(.6)"><use href="#ct-loaf" /></g><use href="#ct-mbox" /></g></g>
				</g>
				{@render zz(868, 424, 'zz-nap')}
			</g>
		{/key}
	</svg>
</div>

<style>
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(180deg, var(--wall), var(--wall) 83%, var(--floor) 83.2%);
	}
	.scene svg {
		display: block;
		width: 100%;
		height: 100%;
	}
	.faded svg {
		-webkit-mask-image: linear-gradient(90deg, transparent, #000 6%, #000 88%, transparent 99%);
		mask-image: linear-gradient(90deg, transparent, #000 6%, #000 88%, transparent 99%);
	}
	.scene-cats {
		--wall: #cfd9e8; --wall2: #b9c6dc; --floor: #a7b4cb; --tile: #94a2bd; --ink: #2b2540;
		--rack: #6d7a96; --rack2: #8b98b3; --vent: #4a5575; --screen: #1c2438; --paper: #fbf4e4;
		--led-g: #34d399; --led-a: #f5a524; --led-r: #ff4d4d;
		--cat-o: #f2a14a; --cat-od: #d97f2b; --cat-g: #9aa3b5; --cat-gd: #78829a; --cat-b: #4a4560; --cat-bd: #2f2b44; --cat-w: #fff3df; --cat-wd: #e6cfa9;
		--pink: #f5a3b5; --box: #d6a56a; --box2: #b0813f; --cable: #e9b53a; --cable2: #e0556a; --heat: #f2845c; --clonk: #ffd84a; --zz: #5b6fb0;
		--sky1: #7ec3f2; --sky2: #d8f0ff; --sun: #ffe27a; --cloud: #fff; --stormcl: #5b6076; --rain: #cfe3ff;
	}
	:global(.dark) .scene-cats.tone-auto,
	.scene-cats.tone-dark,
	.scene-cats.tone-oled {
		--wall: #1a2038; --wall2: #141a2e; --floor: #10152a; --tile: #232c4a; --ink: #0b0e1c;
		--rack: #3c4668; --rack2: #4b5780; --vent: #272f4d; --screen: #0a1020; --paper: #c9d0e6;
		--cat-o: #e08a3a; --cat-od: #b86a22; --cat-g: #8089a4; --cat-gd: #626b88; --cat-b: #5a5578; --cat-bd: #3c3858; --cat-w: #e6dcc8; --cat-wd: #c2b496;
		--pink: #d98a9d; --box: #a87b46; --box2: #86602f; --cable: #f0c24a; --cable2: #ef6a80; --heat: #ff9a6a; --clonk: #ffd84a; --zz: #a9b8ff;
		--sky1: #0a1030; --sky2: #243061; --sun: #e8eef8; --cloud: #35416b; --stormcl: #1c2238; --rain: #7f95cf;
	}
	.scene-cats.tone-oled {
		--wall: #04060d; --wall2: #02030a; --floor: #02030a; --tile: #0d1226; --screen: #000;
	}
	/* wall: the window follows the real hour, whatever the room's tone */
	.scene-cats.tone-light,
	.scene-cats.tone-dark,
	.scene-cats.tone-oled {
		--sky1: #7ec3f2; --sky2: #d8f0ff; --sun: #ffe27a; --cloud: #fff; --stormcl: #5b6076; --rain: #cfe3ff;
	}
	.scene-cats.s-golden {
		--sky1: #f29a52; --sky2: #ffe2a6; --sun: #fff1b0; --cloud: #ffd9b0;
	}
	.scene-cats.s-dusk {
		--sky1: #3b2c6b; --sky2: #f08a7a; --sun: #ffb98a; --cloud: #8a5a86;
	}
	.scene-cats.s-night {
		--sky1: #060a22; --sky2: #1b2552; --sun: #e8eef8; --cloud: #2a3560; --stormcl: #151a30; --rain: #7f95cf;
	}
	.stars {
		opacity: 0;
	}
	.s-night .stars,
	.s-dusk .stars {
		opacity: 0.9;
	}
	.m-warn .stars,
	.m-crit .stars {
		opacity: 0.25;
	}
	.sign {
		transform-box: fill-box;
		transform-origin: 50% 0;
	}
	.paused,
	.paused * {
		animation-play-state: paused !important;
	}
	.startled .box-alarm {
		opacity: 1;
	}
	.startled .box-sleep {
		opacity: 0;
	}

	/* chaque état est caché par défaut, puis révélé par l'humeur ou le geste */
	.crit-only,
	.storm,
	.rain,
	.bolt,
	.flash-w,
	.scr-warn,
	.scr-crit,
	.kb-cat,
	.chewed,
	.crit-v,
	.box-alarm,
	.nap,
	.clonk,
	.impact,
	.glint,
	.clouds-w,
	.zz-nap {
		opacity: 0;
	}
	.m-warn .clouds-w,
	.m-warn .scr-warn,
	.m-warn .kb-cat,
	.m-crit .scr-crit,
	.m-crit .storm,
	.m-crit .rain,
	.m-crit .crit-only,
	.m-crit .crit-v,
	.m-crit .box-alarm,
	.m-crit .chewed {
		opacity: 1;
	}
	.m-warn .sun {
		opacity: 0.55;
	}
	.m-warn .scr-ok,
	.m-crit .scr-ok,
	.m-crit .sun,
	.m-crit .calm-v,
	.m-crit .box-sleep,
	.m-crit .zz-rest,
	.m-crit .cable-play {
		opacity: 0;
	}
	.m-crit .keys {
		opacity: 0.5;
	}
	.led {
		fill: var(--led-g);
	}
	.led.a,
	.m-warn .led.x {
		fill: var(--led-a);
	}
	.m-crit .led.x {
		fill: var(--led-r);
	}
	.zz {
		fill: var(--zz);
		font-weight: 900;
		font-family: ui-rounded, system-ui, sans-serif;
	}
	.zz-rest {
		opacity: 0.8;
	}
	.g-up .head {
		transform: translateY(-5px);
	}
	/* resolved sans mouvement : la pose finale, tout le monde dort */
	.g-res .awake {
		opacity: 0;
	}
	.g-res .nap,
	.g-res .zz-nap {
		opacity: 1;
	}

	@media (prefers-reduced-motion: no-preference) {
		.sign {
			animation: ct-sway 5s ease-in-out infinite;
		}
		.startled .calm-v {
			animation: ct-hop 0.9s ease-out both;
		}
		.led {
			animation: ct-blink 1.6s ease-in-out var(--d, 0s) infinite;
		}
		.m-warn .led.x {
			animation-duration: 0.5s;
		}
		.m-crit .led.x {
			animation-duration: 0.25s;
		}
		.breath {
			transform-box: fill-box;
			transform-origin: 50% 100%;
			animation: ct-breath 3.6s ease-in-out infinite;
		}
		.heat path {
			animation: ct-heat 2.6s ease-in-out infinite;
		}
		.heat path:nth-child(2) {
			animation-delay: -0.9s;
		}
		.heat path:nth-child(3) {
			animation-delay: -1.7s;
		}
		.zz-rest .zz {
			animation: ct-zz 3.4s ease-out infinite;
		}
		.zz-rest .z2 {
			animation-delay: 0.7s;
		}
		.zz-rest .z3 {
			animation-delay: 1.4s;
		}
		.cable-sw {
			transform-box: view-box;
			transform-origin: 780px 330px;
			animation: ct-swing 2.2s ease-in-out infinite;
		}
		.bat {
			transform-box: fill-box;
			transform-origin: 100% 50%;
			animation: ct-bat 2.2s ease-in-out infinite;
		}
		.tail-s,
		.tail-f {
			transform-box: fill-box;
			transform-origin: 0% 100%;
			animation: ct-tail 2.8s ease-in-out infinite;
		}
		.tail-p {
			transform-box: fill-box;
			transform-origin: 0% 100%;
			animation: ct-tail 0.45s ease-in-out infinite;
		}
		.gib {
			animation: ct-type 0.7s steps(2) infinite;
		}
		.gib.g2 {
			animation-delay: -0.2s;
		}
		.gib.g3 {
			animation-delay: -0.45s;
		}
		.keys .k1 {
			animation: ct-key 0.4s steps(2) infinite;
		}
		.keys .k2 {
			animation: ct-key 0.55s steps(2) 0.1s infinite;
		}
		.keys .k3 {
			animation: ct-key 0.3s steps(2) 0.2s infinite;
		}
		.m-warn .pawl {
			animation: ct-tap 0.4s ease-in-out infinite;
		}
		.m-warn .pawr {
			animation: ct-tap 0.4s ease-in-out 0.2s infinite;
		}
		.sp {
			transform-box: fill-box;
			transform-origin: center;
			animation: ct-spark 0.7s steps(3) infinite;
		}
		.sp.s2 {
			animation-delay: -0.25s;
		}
		.sp.s3 {
			animation-delay: -0.5s;
		}
		.m-crit .box-alarm {
			animation: ct-shake 0.3s linear infinite;
		}
		.rain path {
			animation: ct-rain 0.55s linear infinite;
		}
		.m-crit .bolt {
			animation: ct-bolt 7s linear infinite;
		}
		.m-crit .flash-w {
			animation: ct-flash 7s linear infinite;
		}
		.cl1 {
			animation: ct-cloud 40s linear infinite alternate;
		}
		.cl2 {
			animation: ct-cloud 55s linear -20s infinite alternate;
		}

		/* down : le chat pousse le serveur en vous fixant, clonk */
		.g-down .srv {
			animation: ct-down 3.6s cubic-bezier(0.4, 0, 0.6, 1) both;
			transform-box: fill-box;
			transform-origin: 100% 100%;
		}
		.g-down .paw {
			animation: ct-push 3.6s ease-in-out both;
		}
		.g-down .clonk {
			animation: ct-clonk 3.6s ease-out both;
			transform-box: fill-box;
			transform-origin: center;
		}
		.g-down .impact {
			animation: ct-clonk-fx 3.6s linear both;
		}
		/* up : il le remet en place, fier, les oreilles dressées */
		.g-up .srv {
			animation: ct-up 3.4s cubic-bezier(0.3, 0.6, 0.4, 1) both;
			transform-box: fill-box;
			transform-origin: 100% 100%;
		}
		.g-up .head {
			animation: ct-chin 3.4s ease-in-out both;
		}
		.g-up .ears {
			transform-box: fill-box;
			transform-origin: 50% 100%;
			animation: ct-ears 3.4s ease-in-out both;
		}
		.g-up .glint {
			animation: ct-glint 3.4s ease-out both;
		}
		/* resolved : tous les chats se roulent en boule dans leur boîte */
		.g-res .awake {
			animation: ct-out 4.6s ease-in-out both;
		}
		.g-res .nap {
			animation: ct-in 4.6s ease-in-out both;
		}
		.g-res .zz-nap {
			animation: ct-zzfade 4.6s ease-in-out both;
		}
		.g-res .zz-nap .zz {
			animation: ct-zz 2.4s ease-out 1s infinite;
		}
	}

	@keyframes ct-sway {
		0%, 100% { transform: rotate(-1.5deg); }
		50% { transform: rotate(1.5deg); }
	}
	@keyframes ct-hop {
		0% { translate: 0 0; }
		25% { translate: 0 -34px; }
		55% { translate: 0 0; }
		70% { translate: 0 -10px; }
		100% { translate: 0 0; }
	}
	@keyframes ct-blink {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.25; }
	}
	@keyframes ct-breath {
		0%, 100% { transform: scaleY(1); }
		50% { transform: scaleY(1.05); }
	}
	@keyframes ct-heat {
		0%, 100% { opacity: 0.2; transform: translateY(0); }
		50% { opacity: 0.8; transform: translateY(-8px); }
	}
	@keyframes ct-zz {
		0% { opacity: 0; transform: translate(0, 6px); }
		30% { opacity: 0.9; }
		100% { opacity: 0; transform: translate(8px, -16px); }
	}
	@keyframes ct-swing {
		0%, 100% { transform: rotate(-9deg); }
		50% { transform: rotate(9deg); }
	}
	@keyframes ct-bat {
		0%, 100% { transform: rotate(10deg) translateX(0); }
		50% { transform: rotate(-24deg) translateX(-8px); }
	}
	@keyframes ct-tail {
		0%, 100% { transform: rotate(-6deg); }
		50% { transform: rotate(8deg); }
	}
	@keyframes ct-type {
		0% { opacity: 1; }
		50% { opacity: 0.2; }
		100% { opacity: 1; }
	}
	@keyframes ct-key {
		0% { fill-opacity: 1; }
		100% { fill-opacity: 0.35; }
	}
	@keyframes ct-tap {
		0%, 100% { transform: translateY(0); }
		50% { transform: translateY(-5px); }
	}
	@keyframes ct-spark {
		0% { opacity: 1; transform: scale(1) rotate(0); }
		60% { opacity: 1; transform: scale(1.6) rotate(25deg); }
		100% { opacity: 0; transform: scale(0.4) rotate(50deg); }
	}
	@keyframes ct-shake {
		0%, 100% { transform: translate(0, 0); }
		25% { transform: translate(-1.5px, 0.5px); }
		75% { transform: translate(1.5px, -0.5px); }
	}
	@keyframes ct-rain {
		from { transform: translate(0, -20px); }
		to { transform: translate(-7px, 20px); }
	}
	@keyframes ct-bolt {
		0%, 58%, 64%, 100% { opacity: 0; }
		60%, 62% { opacity: 1; }
	}
	@keyframes ct-flash {
		0%, 58%, 66%, 100% { opacity: 0; }
		60% { opacity: 0.7; }
		62% { opacity: 0.15; }
		63% { opacity: 0.5; }
	}
	@keyframes ct-cloud {
		from { transform: translateX(-40px); }
		to { transform: translateX(50px); }
	}
	@keyframes ct-down {
		0%, 24% { transform: translate(0, 0) rotate(0); opacity: 1; }
		44% { transform: translate(26px, 0) rotate(0); }
		56% { transform: translate(44px, 0) rotate(14deg); }
		64% { transform: translate(60px, 40px) rotate(40deg); animation-timing-function: cubic-bezier(0.5, 0, 1, 0.6); }
		72% { transform: translate(106px, 222px) rotate(95deg); }
		75% { transform: translate(110px, 214px) rotate(88deg); }
		78%, 90% { transform: translate(108px, 222px) rotate(92deg); opacity: 1; }
		96% { transform: translate(108px, 222px) rotate(92deg); opacity: 0; }
		96.01% { transform: translate(0, 0) rotate(0); opacity: 0; }
		100% { transform: translate(0, 0) rotate(0); opacity: 1; }
	}
	@keyframes ct-push {
		0%, 14% { transform: translate(0, 0); }
		30%, 46% { transform: translate(24px, -4px); }
		60%, 100% { transform: translate(0, 0); }
	}
	@keyframes ct-clonk {
		0%, 70% { opacity: 0; transform: scale(0.3); }
		74% { opacity: 1; transform: scale(1.35) rotate(-6deg); }
		80% { transform: scale(1) rotate(-4deg); }
		92% { opacity: 1; }
		100% { opacity: 0; transform: scale(1) rotate(-4deg); }
	}
	@keyframes ct-clonk-fx {
		0%, 72% { opacity: 0; }
		74%, 84% { opacity: 1; }
		92%, 100% { opacity: 0; }
	}
	@keyframes ct-up {
		0% { transform: translate(108px, 222px) rotate(92deg); opacity: 0; }
		8% { opacity: 1; }
		50% { transform: translate(50px, -56px) rotate(-8deg); opacity: 1; }
		72% { transform: translate(0, -10px) rotate(0); }
		80% { transform: translate(0, 0) rotate(0); }
		100% { transform: translate(0, 0) rotate(0); opacity: 1; }
	}
	@keyframes ct-chin {
		0%, 40% { transform: translateY(0); }
		60%, 92% { transform: translateY(-6px); }
		100% { transform: translateY(-5px); }
	}
	@keyframes ct-ears {
		0%, 45% { transform: scaleY(1); }
		62% { transform: scaleY(1.5); }
		70%, 94% { transform: scaleY(1.3); }
		100% { transform: scaleY(1); }
	}
	@keyframes ct-glint {
		0%, 74% { opacity: 0; transform: scale(0.3); }
		84% { opacity: 1; transform: scale(1.2); }
		100% { opacity: 0; transform: scale(1); }
	}
	@keyframes ct-out {
		0%, 8%, 100% { opacity: 1; }
		22%, 86% { opacity: 0; }
	}
	@keyframes ct-in {
		0%, 14% { opacity: 0; transform: translateY(8px); }
		30%, 84% { opacity: 1; transform: translateY(0); }
		96%, 100% { opacity: 0; }
	}
	@keyframes ct-zzfade {
		0%, 20% { opacity: 0; }
		40%, 80% { opacity: 1; }
		100% { opacity: 0; }
	}
</style>
