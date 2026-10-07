/**
 * Web Push for the signed-in account: the server's VAPID public key, this
 * account's subscribed devices, and a test send. Mirrors
 * `crates/server/src/api/webpush.rs`.
 */
import { request } from './client';
import type { PushDevice, PushOverview, PushTestReport } from './types';

export function getPushOverview(signal?: AbortSignal): Promise<PushOverview> {
	return request<PushOverview>('/webpush', { signal });
}

/** Sends `PushSubscription.toJSON()`; the same browser subscribing again replaces its row. */
export function savePushSubscription(subscription: PushSubscriptionJSON): Promise<PushDevice> {
	return request<PushDevice>('/webpush/subscriptions', {
		method: 'POST',
		body: { endpoint: subscription.endpoint, keys: subscription.keys }
	});
}

export function deletePushDevice(id: number): Promise<void> {
	return request<void>(`/webpush/subscriptions/${id}`, { method: 'DELETE' });
}

/** Test notification to every device of the account, or to one. */
export function sendPushTest(id?: number): Promise<PushTestReport> {
	return request<PushTestReport>('/webpush/test', {
		method: 'POST',
		body: id === undefined ? {} : { id }
	});
}
