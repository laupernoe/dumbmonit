/**
 * Periodic email reports. Mirrors `crates/server/src/api/reports.rs`.
 * Administrators only, reading included: the list holds recipients' addresses.
 */
import { m } from '#lib/paraglide/messages.js';
import { ApiError, clientViewHeaders, request } from './client';

export type ReportFrequency = 'daily' | 'weekly' | 'monthly';

/** `digest`: the rolling summary; `sla`: the previous calendar month, with PDF and CSV files. */
export type ReportKind = 'digest' | 'sla';

export interface ReportSchedule {
	id: number;
	name: string;
	enabled: boolean;
	frequency: ReportFrequency;
	/** 0 is Monday. */
	weekday: number;
	/** 1 to 28. */
	day_of_month: number;
	/** 0 to 23, in `timezone`. */
	hour: number;
	/** IANA name. */
	timezone: string;
	recipients: string[];
	/** An `smtp` channel; `null`: the first enabled one. */
	channel_id: number | null;
	/** RFC 3339, UTC. */
	last_sent_at: string | null;
	last_error: string | null;
	next_run_at: string | null;
	kind: ReportKind;
	attach_pdf: boolean;
	attach_csv: boolean;
	/** Status page whose title heads the SLA report; `null`: DumbMonit. */
	page_id: number | null;
	/** Availability objective in percent (SLA report); `null`: none. */
	objective: number | null;
}

/** Body of `POST` and `PUT` (a full replacement). */
export interface ReportSchedulePayload {
	name: string;
	enabled: boolean;
	frequency: ReportFrequency;
	weekday: number;
	day_of_month: number;
	hour: number;
	timezone: string;
	recipients: string[];
	channel_id: number | null;
	kind: ReportKind;
	attach_pdf: boolean;
	attach_csv: boolean;
	page_id: number | null;
	objective: number | null;
}

export interface ReportPreviewResult {
	sent: number;
	failed: number;
}

/** Recipients allowed on one report. */
export const MAX_REPORT_RECIPIENTS = 20;

export function listReportSchedules(signal?: AbortSignal): Promise<ReportSchedule[]> {
	return request<ReportSchedule[]>('/reports/schedules', { signal });
}

export function createReportSchedule(payload: ReportSchedulePayload): Promise<ReportSchedule> {
	return request<ReportSchedule>('/reports/schedules', { method: 'POST', body: payload });
}

export function updateReportSchedule(id: number, payload: ReportSchedulePayload): Promise<ReportSchedule> {
	return request<ReportSchedule>(`/reports/schedules/${id}`, { method: 'PUT', body: payload });
}

export function deleteReportSchedule(id: number): Promise<void> {
	return request<void>(`/reports/schedules/${id}`, { method: 'DELETE' });
}

/** Emails the current report, marked as a preview, to the saved recipients. */
export function sendReportPreview(id: number): Promise<ReportPreviewResult> {
	return request<ReportPreviewResult>(`/reports/schedules/${id}/send-test`, { method: 'POST' });
}

export type SlaFormat = 'pdf' | 'csv' | 'incidents_csv';

export interface SlaRequest {
	/** First day included, `YYYY-MM-DD`; with `to` omitted, up to today. */
	from?: string;
	to?: string;
	/** IANA name; the period and the dates in the report follow it. */
	tz?: string;
	format: SlaFormat;
	objective?: number;
	/** Status page whose title heads the report. */
	page?: number;
	/** MSP mode: the client reported on (its targets, name and logo). */
	client?: number;
}

/**
 * Builds the availability report of a chosen period and returns the file.
 * Administrators only. A refusal comes back as an `ApiError` like any other call.
 */
export async function downloadSlaReport(request_: SlaRequest): Promise<{ blob: Blob; filename: string }> {
	const params = new URLSearchParams();
	for (const [key, value] of Object.entries(request_)) {
		if (value !== undefined && value !== '') params.set(key, String(value));
	}
	let response: Response;
	try {
		response = await fetch(`/api/reports/sla?${params}`, {
			headers: { 'x-requested-with': 'DumbMonit', ...clientViewHeaders() }
		});
	} catch {
		throw new ApiError(m.api_error_unreachable(), 0);
	}
	if (!response.ok) {
		let message = m.api_error_status({ status: response.status });
		try {
			const body: unknown = await response.json();
			if (body && typeof body === 'object' && 'error' in body && typeof body.error === 'string') message = body.error;
		} catch {
			// Not JSON: keep the generic message.
		}
		throw new ApiError(message, response.status);
	}
	const disposition = response.headers.get('content-disposition') ?? '';
	const filename = /filename="([^"]+)"/.exec(disposition)?.[1] ?? `availability.${request_.format === 'pdf' ? 'pdf' : 'csv'}`;
	return { blob: await response.blob(), filename };
}
