// The Hint Panel's own page. Its web content process stays loaded while Chord runs, so it
// leaves out Settings: the router, its routes and their styles.
import { QueryClient, QueryClientProvider } from '@chord/com.npmjs.tanstack__react-query';
import { ChordsPage } from '@chord/dev.improve.chord.routes.chords.index';
import { attachConsole } from '@tauri-apps/plugin-log';
import ReactDOM from 'react-dom/client';
import { followSystemAppearance } from './appearance.ts';

followSystemAppearance();

if (import.meta.env.DEV) {
	void attachConsole();
}

const queryClient = new QueryClient();
const rootElement = document.getElementById('root');

if (rootElement && !rootElement.innerHTML) {
	ReactDOM.createRoot(rootElement).render(
		<QueryClientProvider client={queryClient}>
			<ChordsPage />
		</QueryClientProvider>,
	);
}
