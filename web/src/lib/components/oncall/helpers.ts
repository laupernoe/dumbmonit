/** Shared helpers of the on-call screens: names, colours, clocks, timezones, windows. */
import type { Layer, Person, Window } from '#lib/api/index.js';
import { m } from '#lib/paraglide/messages.js';
import { getLocale } from '#lib/paraglide/runtime.js';

export function personName(people: Person[], id: number | null | undefined): string {
	if (id === null || id === undefined) return '';
	const person = people.find((p) => p.id === id);
	return person ? person.display_name.trim() || person.username || `#${id}` : `#${id}`;
}

export function initials(name: string): string {
	const words = name.trim().split(/[\s._-]+/).filter(Boolean);
	if (words.length === 0) return '?';
	if (words.length === 1) return Array.from(words[0]).slice(0, 2).join('').toUpperCase();
	return (Array.from(words[0])[0] + Array.from(words[words.length - 1])[0]).toUpperCase();
}

/**
 * One look per person, from the design tokens (soft or solid). The name or the
 * initials always sit on the bar too: a colour alone never says who it is.
 */
const PERSON_LOOKS = [
	'bg-signal text-on-signal',
	'bg-info text-white',
	'bg-warning text-white',
	'bg-ink text-canvas',
	'bg-signal-soft text-signal-ink border-signal',
	'bg-info-soft text-info-ink border-info',
	'bg-advisory-soft text-advisory-ink border-advisory',
	'bg-warning-soft text-warning-ink border-warning'
];

/** Same person, same look, whatever the screen: ranked by id among everyone known. */
export function personLook(people: Person[], id: number): string {
	const ids = people.map((p) => p.id).sort((a, b) => a - b);
	const rank = ids.indexOf(id);
	return PERSON_LOOKS[(rank < 0 ? id : rank) % PERSON_LOOKS.length];
}

export const pad2 = (n: number) => String(n).padStart(2, '0');

export function minutesToClock(minute: number): string {
	return `${pad2(Math.floor(minute / 60))}:${pad2(minute % 60)}`;
}

export function clockToMinutes(text: string): number | null {
	const match = /^(\d{1,2}):(\d{2})$/.exec(text.trim());
	if (!match) return null;
	const minutes = Number(match[1]) * 60 + Number(match[2]);
	return minutes >= 0 && minutes < 1440 ? minutes : null;
}

/** Short weekday names in the UI language, Monday first. */
export function weekdayNames(style: 'short' | 'long' = 'short'): string[] {
	return [0, 1, 2, 3, 4, 5, 6].map((i) =>
		new Intl.DateTimeFormat(getLocale(), { weekday: style, timeZone: 'UTC' }).format(new Date(Date.UTC(2024, 0, 1 + i)))
	);
}

export interface WindowPreset {
	id: 'working' | 'offhours';
	label: string;
	windows: Window[];
}

export function windowPresets(): WindowPreset[] {
	return [
		{
			id: 'working',
			label: m.oncall_preset_working(),
			windows: [{ days: [0, 1, 2, 3, 4], start_minute: 9 * 60, end_minute: 18 * 60 }]
		},
		{
			id: 'offhours',
			label: m.oncall_preset_nights(),
			// Evenings into the next morning (Sunday's runs into Monday), and whole weekends.
			windows: [
				{ days: [0, 1, 2, 3, 4, 6], start_minute: 18 * 60, end_minute: 9 * 60 },
				{ days: [5, 6], start_minute: 0, end_minute: 0 }
			]
		}
	];
}

/** IANA zones the browser knows, with a short fallback list for older engines. */
export function timezoneList(current?: string): string[] {
	let zones: string[] = [];
	try {
		const supported = (Intl as unknown as { supportedValuesOf?: (key: string) => string[] }).supportedValuesOf;
		if (supported) zones = supported('timeZone');
	} catch {
		zones = [];
	}
	if (zones.length === 0) {
		zones = [
			'UTC',
			'Europe/London',
			'Europe/Paris',
			'Europe/Berlin',
			'Europe/Madrid',
			'Europe/Rome',
			'Europe/Moscow',
			'America/New_York',
			'America/Chicago',
			'America/Denver',
			'America/Los_Angeles',
			'America/Sao_Paulo',
			'Asia/Dubai',
			'Asia/Kolkata',
			'Asia/Shanghai',
			'Asia/Tokyo',
			'Australia/Sydney'
		];
	}
	if (!zones.includes('UTC')) zones = ['UTC', ...zones];
	if (current && !zones.includes(current)) zones = [current, ...zones];
	return zones;
}

export function browserTimezone(): string {
	try {
		return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
	} catch {
		return 'UTC';
	}
}

/** A fresh layer: weekly rotation starting this coming Monday at 09:00, local to the schedule. */
export function newLayer(name: string): Layer {
	const d = new Date();
	d.setDate(d.getDate() + ((8 - d.getDay()) % 7 || 7));
	return {
		name,
		rotation: { kind: 'weekly', start: `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}T09:00`, length: 1, unit: 'days' },
		participants: [],
		restrictions: []
	};
}

// --- Dates for the calendar and the forms -------------------------------------

export function startOfDay(date: Date): Date {
	return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

export function addDays(date: Date, days: number): Date {
	return new Date(date.getFullYear(), date.getMonth(), date.getDate() + days, date.getHours(), date.getMinutes());
}

/** Monday 00:00, local. */
export function startOfWeek(date: Date): Date {
	const day = startOfDay(date);
	return addDays(day, -((day.getDay() + 6) % 7));
}

export function startOfMonth(date: Date): Date {
	return new Date(date.getFullYear(), date.getMonth(), 1);
}

export function addMonths(date: Date, months: number): Date {
	return new Date(date.getFullYear(), date.getMonth() + months, 1);
}

/** `<input type="datetime-local">` value for a date, in the browser's zone. */
export function toLocalInput(date: Date): string {
	return `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())}T${pad2(date.getHours())}:${pad2(date.getMinutes())}`;
}

/** The instant a `datetime-local` value means in the browser's zone, as RFC 3339 UTC; null when unreadable. */
export function fromLocalInput(value: string): string | null {
	if (!value) return null;
	const date = new Date(value);
	return Number.isNaN(date.getTime()) ? null : date.toISOString();
}

export function formatDay(date: Date, options: Intl.DateTimeFormatOptions = { weekday: 'short', day: 'numeric', month: 'short' }): string {
	return new Intl.DateTimeFormat(getLocale(), options).format(date);
}

export function formatClock(date: Date): string {
	return new Intl.DateTimeFormat(getLocale(), { hour: '2-digit', minute: '2-digit' }).format(date);
}

export function parseIso(value: string | null | undefined): Date | null {
	if (!value) return null;
	const date = new Date(value);
	return Number.isNaN(date.getTime()) ? null : date;
}
