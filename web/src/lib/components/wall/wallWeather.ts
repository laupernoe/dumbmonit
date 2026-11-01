/**
 * The weather over the wall, from the state of the network: clear when all is
 * well, clouds for advisories, rain once a device or a critical alert needs
 * someone, a thunderstorm (rain and lightning) when it is a real outage. The
 * weather is never the only sign: the headline says it in words and the
 * tiles carry their own plates.
 *
 * Pure module (no Svelte): also loaded by `tests/wall-weather.test.mjs`.
 */

export type WallWeather = 'clear' | 'cloudy' | 'rain' | 'thunder';

export const WALL_WEATHERS: readonly WallWeather[] = ['clear', 'cloudy', 'rain', 'thunder'];

/** What the weather needs of the sky's counts (`Sky.attention` and `Sky.counts`). */
export interface WeatherInput {
	/** Devices and alerts worth a row. */
	attention: number;
	/** Critical alerts nobody has acknowledged. */
	warnings: number;
	/** Devices that cannot be reached. */
	unreachable: number;
}

export function weatherFor({ attention, warnings, unreachable }: WeatherInput): WallWeather {
	if (attention <= 0) return 'clear';
	if (warnings === 0 && unreachable === 0) return 'cloudy';
	// Two critical alerts, or three things down or critical at once: a storm.
	if (warnings >= 2 || warnings + unreachable >= 3) return 'thunder';
	return 'rain';
}

/** The Paris scene's own vocabulary (rain and its sky are one level; lightning is a layer above). */
export function sceneMoodFor(weather: WallWeather): 'calm' | 'clouded' | 'storm' {
	if (weather === 'clear') return 'calm';
	if (weather === 'cloudy') return 'clouded';
	return 'storm';
}

/** The night cities' vocabulary: they have clear, clouds, and a storm with lightning. */
export function nightToneFor(weather: WallWeather): 'ok' | 'warn' | 'critical' {
	if (weather === 'clear') return 'ok';
	if (weather === 'cloudy') return 'warn';
	return 'critical';
}

/** The weather a dev override (`?mood=ok|warn|critical`) stands for. */
export function weatherForTone(tone: 'ok' | 'warn' | 'critical'): WallWeather {
	return tone === 'ok' ? 'clear' : tone === 'warn' ? 'cloudy' : 'thunder';
}

/** A `?weather=` value (to see a state on a screenshot or a demo), or null. */
export function parseForcedWeather(value: string | null): WallWeather | null {
	return (WALL_WEATHERS as readonly string[]).includes(value ?? '') ? (value as WallWeather) : null;
}
