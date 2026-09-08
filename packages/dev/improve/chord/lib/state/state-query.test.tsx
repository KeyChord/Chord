import { afterEach, expect, test } from 'bun:test';
import { QueryClient } from '@chord/com.npmjs.tanstack__react-query';
import type { UseQueryResult } from '@chord/com.npmjs.tanstack__react-query';
import { renderToString } from 'react-dom/server';
import { createStateQuery } from '#state-query';
import { StateQueries } from '#state-queries';

const clients: QueryClient[] = [];
afterEach(() => { clients.splice(0).forEach(client => client.clear()); });

function deferred<T>() {
	let resolve!: (value: T) => void;
	let reject!: (error: Error) => void;
	const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
	return { promise, resolve, reject };
}

function setup() {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	clients.push(client);
	const listeners = new Map<string, (value: unknown) => void>();
	const snapshot = deferred<{ settings: { enabled: boolean } }>();
	const reading = deferred<void>();
	let requests = 0;
	const options = createStateQuery<{ settings: { enabled: boolean } }>({
		stateIds: ['settings'],
		read: () => { requests++; reading.resolve(); return snapshot.promise; },
		subscribe: async (id, listener) => {
			listeners.set(id, listener);
			return () => { listeners.delete(id); };
		},
	})(client);
	return { client, listeners, snapshot, reading, options, requests: () => requests };
}

test('requests are lazy and concurrent consumers share the initial snapshot', async () => {
	const { client, options, snapshot, reading, requests } = setup();
	expect(requests()).toBe(0);
	const first = client.fetchQuery(options);
	const second = client.fetchQuery(options);
	await reading.promise;
	expect(requests()).toBe(1);
	snapshot.resolve({ settings: { enabled: true } });
	expect(await first).toEqual(await second);
	await client.fetchQuery(options);
	expect(requests()).toBe(1);
});

test('events during the initial request override the older snapshot and keep updating the cache', async () => {
	const { client, options, snapshot, reading, listeners } = setup();
	const pending = client.fetchQuery(options);
	await reading.promise;
	listeners.get('settings')!({ enabled: true });
	// Partial events must not masquerade as a complete initial snapshot.
	expect(client.getQueryData(options.queryKey)).toBeUndefined();
	snapshot.resolve({ settings: { enabled: false } });
	expect(await pending).toEqual({ settings: { enabled: true } });
	listeners.get('settings')!({ enabled: false });
	const cached = client.getQueryData<{ settings: { enabled: boolean } }>(options.queryKey);
	expect(cached?.settings.enabled).toBe(false);
	client.removeQueries({ queryKey: options.queryKey });
	expect(listeners.size).toBe(0);
});

test('failed reads can be retried without duplicating subscriptions', async () => {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	clients.push(client);
	let reads = 0;
	let subscriptions = 0;
	const options = createStateQuery<{ settings: boolean }>({
		stateIds: ['settings'],
		read: async () => {
			if (++reads === 1) throw new Error('backend unavailable');
			return { settings: true };
		},
		subscribe: async () => { subscriptions++; return () => {}; },
	})(client);
	await expect(client.fetchQuery(options)).rejects.toThrow('backend unavailable');
	const retried = await client.fetchQuery(options);
	expect(retried.settings).toBe(true);
	expect(subscriptions).toBe(1);
});

test('listeners registered after cache removal are cleaned up', async () => {
	const client = new QueryClient();
	clients.push(client);
	const registration = deferred<() => void>();
	let cleaned = false;
	const options = createStateQuery<{ settings: boolean }>({
		stateIds: ['settings'],
		read: async () => ({ settings: true }),
		subscribe: () => registration.promise,
	})(client);
	const pending = client.fetchQuery(options).catch(() => {});
	client.removeQueries({ queryKey: options.queryKey });
	registration.resolve(() => { cleaned = true; });
	await pending;
	await registration.promise;
	expect(cleaned).toBe(true);
});

test('pending state leaves independent UI visible and does not render dependent controls', () => {
	const pending = { data: undefined, isError: false } as UseQueryResult<boolean, Error>;
	const html = renderToString(<>
		<aside>Settings navigation</aside>
		<StateQueries queries={[pending]}>{() => <button>Change setting</button>}</StateQueries>
	</>);
	expect(html).toContain('Settings navigation');
	expect(html).toContain('Loading');
	expect(html).not.toContain('Change setting');
});

test('failed state offers retry without removing independent UI', () => {
	const failed = { data: undefined, isError: true } as UseQueryResult<boolean, Error>;
	const html = renderToString(<>
		<aside>Settings navigation</aside>
		<StateQueries queries={[failed]}>{() => <button>Change setting</button>}</StateQueries>
	</>);
	expect(html).toContain('Settings navigation');
	expect(html).toContain('Retry');
	expect(html).not.toContain('Change setting');
});
