// The Hint Panel's own page. Its web content process stays loaded while Chord runs, so it
// leaves out Settings: the router, its routes and their styles.
import { QueryClient, QueryClientProvider } from '@chord/com.npmjs.tanstack__react-query';
import { ChordsPage } from '@chord/dev.improve.chord.routes.chords.index';
import { panelFontStyles } from '@chord/dev.improve.chord.lib.state/panel-preferences';
import { listen } from '@tauri-apps/api/event';
import { attachConsole } from '@tauri-apps/plugin-log';
import ReactDOM from 'react-dom/client';

if (import.meta.env.DEV) {
	void attachConsole();
}

// The fonts chosen in Settings, applied now and again before every show.
const fontStyles = document.createElement('style');
document.head.append(fontStyles);
const applyFonts = () => {
	fontStyles.textContent = panelFontStyles();
};
applyFonts();
window.addEventListener('storage', applyFonts);
void listen('chorder-will-show', applyFonts);

const queryClient = new QueryClient();
const rootElement = document.getElementById('root');

if (rootElement && !rootElement.innerHTML) {
	ReactDOM.createRoot(rootElement).render(
		<QueryClientProvider client={queryClient}>
			<ChordsPage />
		</QueryClientProvider>,
	);
}
