/**
 * HTTPS on the web port and the cleartext report (Settings → Security).
 * Administrator session only.
 */
import { request } from './client';
import type { HttpsView } from './types';

export function getHttps(signal?: AbortSignal): Promise<HttpsView> {
	return request<HttpsView>('/security/https', { signal });
}

/** Send plain-HTTP navigations of the interface to HTTPS on the same port. */
export function setHttpsRedirect(redirect: boolean): Promise<HttpsView> {
	return request<HttpsView>('/security/https', { method: 'PUT', body: { redirect } });
}

/** Download URL of the served certificate (PEM), to trust it on a workstation. */
export const HTTPS_CERTIFICATE_URL = '/api/security/https/certificate.pem';
