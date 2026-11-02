/**
 * HTTPS on the web port and the cleartext report (Settings → Security).
 * Administrator session only.
 */
import { request } from './client';
import type { HttpsView } from './types';

export function getHttps(signal?: AbortSignal): Promise<HttpsView> {
	return request<HttpsView>('/security/https', { signal });
}

/** Download URL of the served certificate (PEM), to trust it on a workstation. */
export const HTTPS_CERTIFICATE_URL = '/api/security/https/certificate.pem';
