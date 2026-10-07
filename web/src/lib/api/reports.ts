/**
 * Periodic email reports. Mirrors `crates/server/src/api/reports.rs`.
 * Administrators only, reading included: the list holds recipients' addresses.
 */
import { request } from './client';

export type ReportFrequency = 'daily' | 'weekly' | 'monthly';

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
