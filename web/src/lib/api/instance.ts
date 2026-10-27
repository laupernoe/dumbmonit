/**
 * Product mode, instance name, language and time zone, and the setup wizard's
 * end. Mirrors `crates/server/src/api/instance.rs`.
 */
import { request } from './client';
import type { InstancePatch, InstanceSettings, PublicInstance } from './types';

/** Public: answered before sign-in, so the right layout shows from the start. */
export function getInstance(signal?: AbortSignal): Promise<PublicInstance> {
	return request<PublicInstance>('/instance', { signal, allowUnauthorized: true });
}

export function getInstanceSettings(signal?: AbortSignal): Promise<InstanceSettings> {
	return request<InstanceSettings>('/instance/settings', { signal });
}

/** Administrators only. */
export function updateInstanceSettings(patch: InstancePatch): Promise<InstanceSettings> {
	return request<InstanceSettings>('/instance/settings', { method: 'PUT', body: patch });
}
