import type { UseQueryResult } from '@chord/com.npmjs.tanstack__react-query';
import type { ReactNode } from 'react';

/** Scope loading and errors to the UI that actually needs backend state. */
export function StateQueries<const T extends unknown[]>({ queries, children, quiet = false }: {
	queries: { [K in keyof T]: UseQueryResult<T[K], Error> }
	children: (...states: NoInfer<T>) => ReactNode
	quiet?: boolean
}) {
	const failed = queries.find(query => query.isError && query.data === undefined);
	if (failed) {
		return (
			<div role="alert" className="p-4 text-sm text-muted-foreground">
				Could not load app state.{' '}
				<button type="button" className="underline" onClick={() => { void failed.refetch(); }}>
					Retry
				</button>
			</div>
		);
	}
	if (queries.some(query => query.data === undefined)) {
		return quiet ? null : <div role="status" className="p-4 text-sm text-muted-foreground">Loading…</div>;
	}
	return children(...queries.map(query => query.data) as unknown as T);
}
