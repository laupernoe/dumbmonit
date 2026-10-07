<script lang="ts">
	/**
	 * Venice at the golden hour — basilica, campanile, Doge's Palace, a gondola,
	 * a mooring post whose lamp takes the page's status colour
	 * (`--scene-tone`, set by the page) and two DumbMonit pigeons on the quay.
	 * Static, no animation. Follows the scene contract: fills its positioned
	 * parent, the headline zone (top left) is left clear, the palette lives here.
	 */
</script>

<div class="scene scene-venice" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
  <pattern id="vn-dia" width="22" height="22" patternUnits="userSpaceOnUse"><rect width="22" height="22" fill="var(--wh)"/><path d="M11 0L22 11L11 22L0 11Z" fill="var(--pink)"/></pattern>
  <linearGradient id="vn-wat" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--water1)"/><stop offset="1" stop-color="var(--water2)"/></linearGradient>
  <g id="vn-dome"><path d="M-34 0C-38-28-16-52 0-76C16-52 38-28 34 0Z" fill="var(--stone)"/><path d="M0-76C-6-40-8-20-8 0M0-76C6-40 8-20 8 0" stroke="var(--stone2)" stroke-width="2"/><rect x="-3" y="-96" width="6" height="22" fill="var(--gold)"/><circle cx="0" cy="-100" r="5" fill="var(--gold)"/></g>
		</defs>
<g transform="translate(0 80)">
 <circle cx="1110" cy="110" r="46" fill="var(--sun)" opacity=".9"/>
 <g fill="var(--cloud)" opacity=".75"><ellipse cx="820" cy="110" rx="70" ry="16"/><ellipse cx="860" cy="98" rx="46" ry="16"/><ellipse cx="1290" cy="200" rx="80" ry="14"/></g>
 <g transform="translate(170 0)">
 <!-- far: Salute -->
 <g opacity=".5" transform="translate(470 392)"><rect x="-70" y="-60" width="140" height="60" fill="var(--stone2)"/><path d="M-28-60C-30-96 0-120 0-120C0-120 30-96 28-60Z" fill="var(--stone2)"/><rect x="-2" y="-138" width="4" height="20" fill="var(--stone2)"/></g>
 <!-- Basilica -->
 <g transform="translate(0 0)">
  <rect x="640" y="300" width="320" height="100" fill="var(--stone)"/>
  <g fill="var(--stone2)"><rect x="640" y="296" width="320" height="8"/></g>
  <use href="#vn-dome" x="715" y="296"/><use href="#vn-dome" x="800" y="290" transform="translate(0 0)"/><use href="#vn-dome" x="885" y="296"/>
  <g fill="var(--gold)" opacity=".9"><path d="M660 342a22 22 0 0 1 44 0v16h-44zM716 332a22 22 0 0 1 44 0v26h-44zM772 322a28 28 0 0 1 56 0v36h-56zM840 332a22 22 0 0 1 44 0v26h-44zM896 342a22 22 0 0 1 44 0v16h-44z"/></g>
  <g fill="var(--dark)"><path d="M664 400v-30a18 18 0 0 1 36 0v30zM720 400v-40a18 18 0 0 1 36 0v40zM778 400v-48a22 22 0 0 1 44 0v48zM844 400v-40a18 18 0 0 1 36 0v40zM900 400v-30a18 18 0 0 1 36 0v30z"/></g>
 </g>
 <!-- Campanile -->
 <g>
  <rect x="978" y="190" width="46" height="210" fill="var(--brick)"/>
  <g stroke="var(--brick2)" stroke-width="2"><path d="M988 190v210M998 190v210M1008 190v210M1018 190v210" opacity=".5"/></g>
  <rect x="972" y="150" width="58" height="40" fill="var(--stone)"/>
  <g fill="var(--dark)"><path d="M980 190v-20a6 6 0 0 1 12 0v20zM1010 190v-20a6 6 0 0 1 12 0v20z"/></g>
  <rect x="970" y="132" width="62" height="18" fill="var(--wh)"/>
  <path d="M970 132L1001 52L1032 132Z" fill="var(--roof)"/>
  <rect x="999" y="30" width="4" height="26" fill="var(--gold)"/><path d="M990 32l11-14 11 14z" fill="var(--gold)"/>
 </g>
 <!-- Doge's Palace -->
 <g>
  <rect x="1036" y="258" width="520" height="142" fill="url(#vn-dia)"/>
  <rect x="1036" y="250" width="520" height="10" fill="var(--wh)"/>
  <g fill="var(--wh)"><path d="M1036 250h520v-6h-520z"/></g>
  <rect x="1036" y="332" width="520" height="68" fill="var(--wh)"/>
  <rect x="1036" y="296" width="520" height="38" fill="var(--wh)"/>
  <g fill="var(--dark)" opacity=".85"><path d="M1044 400v-30a9 9 0 0 1 18 0v30zM1086 400v-30a9 9 0 0 1 18 0v30zM1128 400v-30a9 9 0 0 1 18 0v30zM1170 400v-30a9 9 0 0 1 18 0v30zM1212 400v-30a9 9 0 0 1 18 0v30zM1254 400v-30a9 9 0 0 1 18 0v30zM1296 400v-30a9 9 0 0 1 18 0v30zM1338 400v-30a9 9 0 0 1 18 0v30zM1380 400v-30a9 9 0 0 1 18 0v30z"/></g>
  <g fill="var(--pink)"><circle cx="1053" cy="314" r="9"/><circle cx="1095" cy="314" r="9"/><circle cx="1137" cy="314" r="9"/><circle cx="1179" cy="314" r="9"/><circle cx="1221" cy="314" r="9"/><circle cx="1263" cy="314" r="9"/><circle cx="1305" cy="314" r="9"/><circle cx="1347" cy="314" r="9"/><circle cx="1389" cy="314" r="9"/></g>
  <g class="win" opacity=".9"><path d="M1090 292v-26a10 10 0 0 1 20 0v26zM1190 292v-26a10 10 0 0 1 20 0v26zM1290 292v-26a10 10 0 0 1 20 0v26zM1390 292v-26a10 10 0 0 1 20 0v26z"/></g>
  <g fill="var(--wh)"><path d="M1036 244l12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10 12-10 12 10v6h-520z"/></g>
 </g>
 </g>
 <!-- quay -->
 <rect x="560" y="398" width="900" height="8" fill="var(--stone2)"/>
 <!-- water -->
 <rect x="0" y="404" width="1440" height="600" fill="url(#vn-wat)"/>
 <g opacity=".22" transform="translate(0 808) scale(1 -1)"><rect x="978" y="404" width="46" height="120" fill="var(--brick)"/><rect x="640" y="404" width="320" height="60" fill="var(--stone)"/><rect x="1036" y="404" width="420" height="60" fill="var(--pink)"/></g>
 <g stroke="var(--wh)" stroke-width="2" opacity=".5" stroke-linecap="round"><path d="M560 430h60M700 446h90M880 424h70M1040 440h120M1220 428h80M620 470h70M780 486h110M960 466h80M1120 482h100M1280 468h90M420 440h60M300 476h80"/></g>
 <!-- gondola -->
 <g transform="translate(890 452) scale(.8)">
  <path d="M0 0C40 10 150 10 190-6C194-18 196-34 188-52L183-52C187-34 182-18 170-12C120-4 50-4 0-12C-6-8-6-4 0 0Z" fill="#14182a"/>
  <path d="M-2-12C-10-20-10-34 0-42L0-12Z" fill="#14182a"/>
  <path d="M184-52l10 3-4 7-8-4z" fill="#c9ced8"/>
  <path d="M70-4L100-4L98-14L72-14z" fill="#8a3a3a"/>
  <g fill="#14182a"><ellipse cx="150" cy="-62" rx="6" ry="6"/><path d="M144-56h12l4 40h-20z"/></g>
  <path d="M148-50L110 4" stroke="#6b4a2b" stroke-width="3"/>
 </g>
 <!-- piles with status lamps -->
 <g>
  <g fill="#fff" stroke="#c0392b"><rect x="880" y="372" width="12" height="70" rx="3" fill="#fff" stroke="none"/><path d="M880 384l12-6v10l-12 6zM880 404l12-6v10l-12 6zM880 424l12-6v10l-12 6z" fill="#2f6db5" stroke="none"/></g>
  <circle cx="886" cy="366" r="9" fill="var(--scene-tone, var(--c-signal))"/><circle cx="886" cy="366" r="20" fill="var(--scene-tone, var(--c-signal))" opacity=".25"/>
 </g>
 <!-- pigeons on the quay and the mascot -->
 <use href="#dm-mascot" transform="translate(1194 370) scale(.1)"/><use href="#dm-mascot" transform="translate(1276 370) scale(.1)"/>
</g></svg>
</div>

<style>
	.scene-venice {
		--sky1: #f6d9b4;
		--sky2: #fbefd9;
		--sun: #ffd98a;
		--water1: #9ec3c4;
		--water2: #6fa0a6;
		--stone: #efe4cf;
		--stone2: #dccdb0;
		--brick: #b95c3f;
		--brick2: #9d4a32;
		--roof: #4f9a86;
		--gold: #e0a82e;
		--pink: #e7b8a6;
		--wh: #f7efe0;
		--dark: #3a3f52;
		--win: #3a3f52;
		--cloud: #fff;
	}
	:global(.dark) .scene-venice {
		--sky1: #0d1630;
		--sky2: #1a2748;
		--sun: #e8eef8;
		--water1: #16294a;
		--water2: #0a1428;
		--stone: #6d7690;
		--stone2: #4d5671;
		--brick: #6a3c44;
		--brick2: #53303a;
		--roof: #2c5b58;
		--gold: #e8bd4f;
		--pink: #7c6270;
		--wh: #8b92a8;
		--dark: #0a0f1d;
		--win: #f5c76a;
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
	.win {
		fill: var(--win);
	}
</style>
