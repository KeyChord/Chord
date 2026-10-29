import { afterAll, expect, test } from 'bun:test';
import { QueryClient, QueryClientProvider } from '@chord/com.npmjs.tanstack__react-query';
import { renderToString } from 'react-dom/server';

// Mutations never run during server rendering; TauRPC needs the browser's Proxy constructor.
const previousWindow = globalThis.window;
globalThis.window = { Proxy, screen: {} } as unknown as Window & typeof globalThis;
const { SettingsBrowsePageContent } = await import('#settings-browse-page');
afterAll(() => {
	globalThis.window = previousWindow;
});

type Repos = Parameters<typeof SettingsBrowsePageContent>[0]['repos'];

function gitRepo(name: string, fields: Partial<Repos[string]> = {}): Repos[string] {
	return {
		owner: 'KeyChord',
		name,
		slug: `KeyChord/${name}`,
		url: `https://github.com/KeyChord/${name}`,
		localAbspath: `/cache/KeyChord/${name}`,
		...fields,
	};
}

function render(repos: Repos) {
	return renderToString(
		<QueryClientProvider client={new QueryClient()}>
			<SettingsBrowsePageContent repos={repos} />
		</QueryClientProvider>,
	).replaceAll('<!-- -->', '');
}

function listing(html: string, name: string) {
	const match = html.match(new RegExp(`<article[^>]*><div[^>]*><div[^>]*><h3[^>]*>${name}</h3>.*?</article>`));
	expect(match).not.toBeNull();
	return match![0];
}

test('each repo lists its categories, description, slug and install action', () => {
	const html = render({});
	const chromium = listing(html, 'Chromium Chords');
	expect(chromium).toContain('>Browsers</span>');
	expect(chromium).toContain('Chords for Google Chrome');
	expect(chromium).toContain('>KeyChord/chords-chromium</span>');
	expect(chromium).toContain('Install</button>');
	expect(chromium).not.toContain('Uninstall');
});

test('categories appear once each as filters after All', () => {
	const html = render({});
	const filters = html.match(/<div[^>]*aria-label="Category"[^>]*>(.*?)<\/div>/)![1]!;
	const labels = [...filters.matchAll(/<button[^>]*>([^<]*)<\/button>/g)].map(match => match[1]);
	expect(labels).toEqual(['All', 'Browsers', 'Editors', 'Terminals', 'Window Management']);
});

test('installed repos show their commit and uninstall action; linked and pinned repos say so', () => {
	const html = render({
		'KeyChord/chords-vscode': gitRepo('chords-vscode', { headShortSha: 'abc1234' }),
		'KeyChord/chords-terminal': gitRepo('chords-terminal', { headShortSha: 'def5678', linkedLocalPath: '/src/chords-terminal' }),
		'KeyChord/chords-aerospace': gitRepo('chords-aerospace', { headShortSha: '0123abc', pinnedRev: 'v1' }),
	});

	const vscode = listing(html, 'VSCode Chords');
	expect(vscode).toContain('>abc1234</span>');
	expect(vscode).toContain('Uninstall</button>');

	const terminal = listing(html, 'Terminal Chords');
	expect(terminal).not.toContain('def5678');
	expect(terminal).toContain('>Linked locally</span>');

	expect(listing(html, 'AeroSpace Chords')).toContain('>Pinned</span>');
	expect(listing(html, 'Chromium Chords')).toContain('Install</button>');
});

test('install all is disabled once every repo is installed', () => {
	const installAllButton = (html: string) => html.match(/<button[^>]*>(?:(?!<button).)*?Install All<\/button>/)![0];
	expect(installAllButton(render({}))).not.toContain('disabled=""');

	const all = Object.fromEntries(
		['chords-chromium', 'chords-vscode', 'chords-jetbrains', 'chords-terminal', 'chords-warp', 'chords-aerospace']
			.map(name => [`KeyChord/${name}`, gitRepo(name)]),
	);
	expect(installAllButton(render(all))).toContain('disabled=""');
});

test('community packages are behind a trust warning, not a direct link', () => {
	const html = render({});
	expect(html).toContain('>Community chord packages</h3>');
	expect(html).toMatch(/<button[^>]*aria-expanded="false"[^>]*>(?:(?!<button).)*?Search GitHub<\/button>/);
	expect(html).not.toContain('topic%3Achordpack');
});
