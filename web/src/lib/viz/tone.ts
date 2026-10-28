import type { Tone } from './types.ts';

/** Token behind each tone. Never a raw colour: light, dark and the wall theme redefine them. */
export const TONE_COLOR: Record<Tone, string> = {
	signal: 'var(--c-signal)',
	info: 'var(--c-info)',
	advisory: 'var(--c-advisory)',
	warning: 'var(--c-warning)',
	ghost: 'var(--c-ink-3)'
};

/** Text-safe variant (contrast on the surface). */
export const TONE_INK: Record<Tone, string> = {
	signal: 'var(--c-signal-ink)',
	info: 'var(--c-info-ink)',
	advisory: 'var(--c-advisory-ink)',
	warning: 'var(--c-warning-ink)',
	ghost: 'var(--c-ink-3)'
};

export const TONE_SOFT: Record<Tone, string> = {
	signal: 'var(--c-signal-soft)',
	info: 'var(--c-info-soft)',
	advisory: 'var(--c-advisory-soft)',
	warning: 'var(--c-warning-soft)',
	ghost: 'var(--c-ghost)'
};

/** A shape per tone, so a status survives colour blindness and greyscale. */
export const TONE_GLYPH: Record<Tone, string> = {
	signal: '✓',
	info: 'i',
	advisory: '!',
	warning: '✕',
	ghost: '–'
};
