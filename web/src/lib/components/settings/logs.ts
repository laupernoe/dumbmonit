/**
 * Pure logic of the Logs section: export period, sizes, destination payload.
 * No UI or message import, so it can be tested on its own.
 */
import { getLocale } from '#lib/paraglide/runtime.js';
import type { KindField } from '#lib/components/notifications/kinds';
import type { LogDestination, LogLevel } from '#lib/api/types';
import type { LogDestinationPayload } from '#lib/api/types';

export const RETENTION_MIN = 1;
export const RETENTION_MAX = 365;
export const MAX_MB_MIN = 16;
export const MAX_MB_MAX = 102400;
export const LEVELS: readonly LogLevel[] = ['error', 'warn', 'info', 'debug'];
/** Quick choices for the export period, in days. */
export const EXPORT_CHOICES: readonly number[] = [1, 7, 14, 30];

/** Quick choices with whether each fits within the retention. */
export function exportChoices(retention: number): { days: number; disabled: boolean }[] {
	return EXPORT_CHOICES.map((days) => ({ days, disabled: days > retention }));
}

/** A server day ("2026-10-21", UTC) as a localized date; empty when absent. */
export function formatLogDay(day: string | null | undefined): string {
	if (!day) return '';
	const date = new Date(`${day}T00:00:00Z`);
	if (Number.isNaN(date.getTime())) return '';
	return new Intl.DateTimeFormat(getLocale(), { dateStyle: 'medium', timeZone: 'UTC' }).format(date);
}

function asNumber(value: unknown): number {
	const text = String(value ?? '').trim();
	return text === '' ? NaN : Number(text);
}

/** A whole number of days between 1 and the retention; `null` when unusable. */
export function clampDays(value: unknown, retention: number): number | null {
	const n = asNumber(value);
	if (!Number.isFinite(n) || n < 1) return null;
	return Math.min(Math.floor(n), Math.max(1, retention));
}

/** True when `value` is an integer within `[min, max]`. */
export function inRange(value: unknown, min: number, max: number): boolean {
	const n = asNumber(value);
	return Number.isInteger(n) && n >= min && n <= max;
}

/** Binary units, one decimal under 10 (`1.5 MB`, `812 kB`). */
export function formatSize(value: number): string {
	if (!Number.isFinite(value) || value <= 0) return '0 B';
	const units = ['B', 'kB', 'MB', 'GB', 'TB'];
	let n = value;
	let unit = 0;
	while (n >= 1024 && unit < units.length - 1) {
		n /= 1024;
		unit += 1;
	}
	return `${n < 10 && unit > 0 ? n.toFixed(1) : Math.round(n)} ${units[unit]}`;
}

export type FieldValue = string | boolean;

/** Turns a stored value into what its input edits. */
export function toFieldValue(field: KindField, raw: unknown): FieldValue {
	if (field.input === 'boolean') return raw === true;
	if (raw === null || raw === undefined) return '';
	if (field.shape === 'list') return Array.isArray(raw) ? raw.map(String).join('\n') : String(raw);
	if (field.shape === 'object') return typeof raw === 'string' ? raw : JSON.stringify(raw, null, 2);
	return typeof raw === 'string' ? raw : String(raw);
}

export function isBlank(value: FieldValue | undefined): boolean {
	return value === undefined || (typeof value === 'string' && value.trim() === '');
}

/** Converts a field value to what the server expects; throws `'number'`, `'json'` or `'json_object'`. */
export function toServerValue(field: KindField, value: FieldValue): unknown {
	if (field.input === 'boolean') return value === true;
	if (typeof value !== 'string') return value;
	const text = value.trim();
	if (field.shape === 'list') {
		return text
			.split(/\r?\n/)
			.map((line) => line.trim())
			.filter(Boolean);
	}
	if (field.shape === 'object') {
		let parsed: unknown;
		try {
			parsed = JSON.parse(text);
		} catch {
			throw new Error('json');
		}
		if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) throw new Error('json_object');
		return parsed;
	}
	if (field.input === 'number') {
		const n = Number(text);
		if (Number.isNaN(n)) throw new Error('number');
		return n;
	}
	return text;
}

/** Normalised for comparison: blank and absent are the same. */
function comparable(value: FieldValue | undefined): string {
	if (value === undefined || isBlank(value)) return '';
	return typeof value === 'string' ? value.trim() : String(value);
}

/**
 * Whether a destination field changed on an existing destination that holds
 * saved secrets: the server then wants the secrets typed again.
 */
export function destinationChanged(
	existing: Pick<LogDestination, 'kind' | 'has_secret' | 'settings'> | undefined,
	kind: string,
	settingFields: KindField[],
	settings: Record<string, FieldValue>
): boolean {
	if (!existing || !existing.has_secret) return false;
	if (kind !== existing.kind) return true;
	return settingFields.some(
		(field) =>
			field.destination === true &&
			comparable(settings[field.key]) !== comparable(toFieldValue(field, existing.settings?.[field.key]))
	);
}

export interface PayloadInput {
	name: string;
	kind: string;
	enabled: boolean;
	minLevel: LogLevel;
	settingFields: KindField[];
	secretFields: KindField[];
	settings: Record<string, FieldValue>;
	secrets: Record<string, string>;
	existing?: LogDestination;
	clearSecrets?: boolean;
	destinationChanged?: boolean;
}

/**
 * Create: `secrets` always sent (maybe `{}`). Edit: sent only when something
 * was typed, a clear was asked or the destination changed; omitted = keep.
 */
export function buildDestinationPayload(input: PayloadInput): LogDestinationPayload {
	const { existing, settingFields, secretFields } = input;
	const editing = existing !== undefined;
	const out: Record<string, unknown> = existing && existing.kind === input.kind ? { ...existing.settings } : {};
	for (const field of settingFields) {
		const value = input.settings[field.key];
		if (field.input !== 'boolean' && isBlank(value)) {
			delete out[field.key];
			continue;
		}
		out[field.key] = toServerValue(field, value ?? '');
	}
	const typed: Record<string, unknown> = {};
	for (const field of secretFields) {
		const value = input.secrets[field.key];
		if (!isBlank(value)) typed[field.key] = value.trim();
		else if (editing && input.clearSecrets) typed[field.key] = null;
	}
	const payload: LogDestinationPayload = {
		name: input.name.trim(),
		kind: input.kind,
		enabled: input.enabled,
		min_level: input.minLevel,
		settings: out
	};
	if (!editing || input.clearSecrets || input.destinationChanged || Object.keys(typed).length > 0) {
		payload.secrets = typed;
	}
	return payload;
}
