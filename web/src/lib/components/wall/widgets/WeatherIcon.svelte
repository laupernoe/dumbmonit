<script lang="ts">
	/** A WMO weather code as a picture (decorative: the words sit next to it). */
	import { Cloud, CloudDrizzle, CloudFog, CloudLightning, CloudMoon, CloudRain, CloudSnow, CloudSun, Moon, Sun } from 'lucide-svelte';
	import { weatherKind } from '#lib/wall/widgets.js';

	interface Props {
		code: number;
		isDay?: boolean;
		class?: string;
	}

	let { code, isDay = true, class: className = '' }: Props = $props();

	const Icon = $derived.by(() => {
		switch (weatherKind(code)) {
			case 'clear':
				return isDay ? Sun : Moon;
			case 'partly':
				return isDay ? CloudSun : CloudMoon;
			case 'fog':
				return CloudFog;
			case 'drizzle':
				return CloudDrizzle;
			case 'rain':
				return CloudRain;
			case 'snow':
				return CloudSnow;
			case 'storm':
				return CloudLightning;
			default:
				return Cloud;
		}
	});
</script>

<Icon class={className} aria-hidden="true" strokeWidth={1.9} />
