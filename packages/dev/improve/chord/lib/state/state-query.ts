import type { QueryClient } from '@chord/com.npmjs.tanstack__react-query';

const queryKey = ['tauri-observable-states'] as const;

/** Keep a shared snapshot subscribed for as long as it remains in the query cache. */
export function createStateQuery<T extends object>({ stateIds, read, subscribe }: {
	stateIds: readonly (keyof T & string)[]
	read: () => Promise<T>
	subscribe: (id: keyof T & string, onChange: (value: unknown) => void) => Promise<() => void>
}) {
	const sources = new WeakMap<QueryClient, {
		ready: Promise<void>
		updates: Map<keyof T, unknown>
	}>();

	return (client: QueryClient) => ({
		queryKey,
		staleTime: Infinity,
		queryFn: async (): Promise<T> => {
			let source = sources.get(client);
			if (!source) {
				const updates = new Map<keyof T, unknown>();
				const unlisteners: (() => void)[] = [];
				let disposed = false;
				let unsubscribeCache = () => {};
				const dispose = () => {
					disposed = true;
					unlisteners.splice(0).forEach(unlisten => unlisten());
					unsubscribeCache();
					sources.delete(client);
				};
				unsubscribeCache = client.getQueryCache().subscribe(event => {
					if (event.type === 'removed' && event.query.queryKey[0] === queryKey[0]) {
						dispose();
					}
				});
				const ready = Promise.all(stateIds.map(async id => {
					const unlisten = await subscribe(id, value => {
						if (disposed) return;
						updates.set(id, value);
						client.setQueryData<T>(queryKey, previous => previous
							? { ...previous, [id]: value }
							: undefined);
					});
					if (disposed) unlisten();
					else unlisteners.push(unlisten);
				})).then(() => {}, error => {
					dispose();
					throw error;
				});
				source = { ready, updates };
				sources.set(client, source);
			}
			await source.ready;
			source.updates.clear();
			const snapshot = await read();
			// An event received while the request was in flight wins over its snapshot.
			return Object.assign({}, snapshot, Object.fromEntries(source.updates));
		},
	});
}
