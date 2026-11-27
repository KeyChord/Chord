import { RouterProvider } from '@chord/com.npmjs.tanstack__react-router';
import { attachConsole } from '@tauri-apps/plugin-log';
import ReactDOM from 'react-dom/client';
import { getRouter } from './router.tsx';
import { followAppearance } from '@chord/dev.improve.chord.lib.state/appearance';

followAppearance();

if (import.meta.env.DEV) {
	void attachConsole();
}

const rootElement = document.getElementById('root');

if (rootElement && !rootElement.innerHTML) {
	const root = ReactDOM.createRoot(rootElement);
	root.render(<RouterProvider router={getRouter()} />);
}
