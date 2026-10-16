<script lang="ts">
	/**
	 * The weather now, under the wall's clock: an icon and the temperature of
	 * the place chosen in Settings, nothing more. The words (place, sky,
	 * temperature) are what a screen reader hears; the source, Open-Meteo,
	 * is credited in the tooltip. Nothing shows while there is no reading.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import type { WallSection, WallWeather } from '#lib/api/types.js';
	import { roundTemperature, weatherKind, type WeatherKind } from '#lib/wall/widgets.js';
	import WeatherIcon from './widgets/WeatherIcon.svelte';

	interface Props {
		section: WallSection<WallWeather>;
	}

	let { section }: Props = $props();

	const SKY: Record<WeatherKind, () => string> = {
		clear: m.wall_widgets_sky_clear,
		partly: m.wall_widgets_sky_partly,
		cloudy: m.wall_widgets_sky_cloudy,
		fog: m.wall_widgets_sky_fog,
		drizzle: m.wall_widgets_sky_drizzle,
		rain: m.wall_widgets_sky_rain,
		snow: m.wall_widgets_sky_snow,
		storm: m.wall_widgets_sky_storm
	};

	const data = $derived(section.data);
	const spoken = $derived(
		data
			? m.wall_weather_now({
					place: data.name,
					sky: SKY[weatherKind(data.code)](),
					temperature: `${Math.round(data.temperature)}${data.temperature_unit}`
				})
			: ''
	);
</script>

{#if data}
	<p class="weather" role="img" aria-label={spoken} title={`${spoken} · ${m.wall_widgets_source({ source: section.source })}`}>
		<WeatherIcon code={data.code} isDay={data.is_day} class="icon" />
		<span class="temp tnum" aria-hidden="true">{roundTemperature(data.temperature)}</span>
	</p>
{/if}

<style>
	.weather {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: 0.375rem;
		color: var(--c-ink-2);
	}
	.weather :global(.icon) {
		width: 1.75rem;
		height: 1.75rem;
	}
	.temp {
		font-size: 1.5rem;
		font-weight: 650;
	}
	@media (min-width: 1024px) {
		.weather :global(.icon) {
			width: 2.25rem;
			height: 2.25rem;
		}
		.temp {
			font-size: 1.875rem;
		}
	}
</style>
