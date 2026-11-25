import { afterAll, expect, test } from 'bun:test';
import { QueryClient, QueryClientProvider } from '@chord/com.npmjs.tanstack__react-query';
import { officialChordReposData } from '@chord/dev.improve.chord.data.official-chord-repos';
import type { GitReposState } from '@chord/dev.improve.chord.lib.typeshare';
import { renderToString } from 'react-dom/server';

// Mutations never run during server rendering; TauRPC needs the browser's Proxy constructor.
const previousWindow = globalThis.window;
globalThis.window = { Proxy, screen: {}, navigator: { userAgent: 'Macintosh' } } as unknown as Window & typeof globalThis;
const { SettingsBrowsePageContent } = await import('#settings-browse-page');
const { FilterMenu } = await import('#filter-menu');
afterAll(() => {
	globalThis.window = previousWindow;
});

type Repos = GitReposState['repos'];

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

function render(repos: Repos, fields: Partial<GitReposState> = {}) {
	const state: GitReposState = {
		repos,
		updates: {},
		updateCheck: { checking: false, failed: [] },
		...fields,
	};
	return renderToString(
		<QueryClientProvider client={new QueryClient()}>
			<SettingsBrowsePageContent state={state} onConfigure={() => {}} />
		</QueryClientProvider>,
	).replaceAll('<!-- -->', '');
}

function listing(html: string, name: string) {
	const match = html.match(new RegExp(`<tr[^>]*><td[^>]*><div[^>]*><h3[^>]*>${name}</h3>.*?</tr>`));
	expect(match).not.toBeNull();
	return match![0];
}

test('each repo lists its categories, description, slug and install action', () => {
	const html = render({});
	expect(html).toMatch(/<thead[^>]*><tr><th[^>]*>Repository<\/th><th[^>]*>Category<\/th><th[^>]*>Actions<\/th>/);
	const chromium = listing(html, 'Chromium Chords');
	expect(chromium).toContain('>Browsers</span>');
	expect(chromium).toContain('Chords for Google Chrome');
	expect(chromium).toContain('>KeyChord/chords-chromium</span>');
	expect(chromium).toContain('Install</button>');
	expect(chromium).not.toContain('Uninstall');
	expect(chromium).not.toContain('Configure');
});

test('the page is the search field with a Filter button beside it, then the table', () => {
	const html = render({});
	expect(html).not.toContain('>Browse</h2>');
	expect(html).not.toContain('Updates');
	const search = html.indexOf('aria-label="Search chord repos"');
	const filter = html.search(/<button[^>]*aria-label="Filter"/);
	expect(search).toBeGreaterThan(-1);
	expect(filter).toBeGreaterThan(search);
	expect(html.indexOf('<table')).toBeGreaterThan(filter);
	// The status and category choices live in the Filter menu, not on the page.
	expect(html).not.toContain('Not Installed');
	expect(html).not.toContain('aria-label="Category"');
});

test('every official repo is listed in one table', () => {
	const html = render({});
	expect(html.match(/<table/g)).toHaveLength(1);
	for (const repo of officialChordReposData) {
		listing(html, repo.displayName);
	}
});

test('the Filter button counts the filters in use', () => {
	function renderFilter(install: 'all' | 'installed', category: string) {
		return renderToString(
			<FilterMenu
				install={install}
				onInstallChange={() => {}}
				categories={[{ value: 'all', label: 'All' }, { value: 'Browsers', label: 'Browsers' }]}
				category={category}
				onCategoryChange={() => {}}
			/>,
		).replaceAll('<!-- -->', '');
	}
	expect(renderFilter('all', 'all')).toMatch(/aria-label="Filter"[^>]*>(?:(?!<\/button>).)*Filter<\/button>/);
	expect(renderFilter('installed', 'all')).toContain('aria-label="Filter (1 active)"');
	expect(renderFilter('installed', 'Browsers')).toMatch(/aria-label="Filter \(2 active\)"[^>]*>(?:(?!<\/button>).)*>2<\/span><\/button>/);
});

test('installed repos show Uninstall to the left of Configure and indicate linked or pinned repos', () => {
	const html = render({
		'KeyChord/chords-vscode': gitRepo('chords-vscode', { headShortSha: 'abc1234' }),
		'KeyChord/chords-terminal': gitRepo('chords-terminal', { headShortSha: 'def5678', linkedLocalPath: '/src/chords-terminal' }),
		'KeyChord/chords-aerospace': gitRepo('chords-aerospace', { headShortSha: '0123abc', pinnedRev: 'v1' }),
	});

	const vscode = listing(html, 'VSCode Chords');
	expect(vscode).toContain('>abc1234</span>');
	expect(vscode).toMatch(/<button[^>]*class="[^"]*text-destructive underline[^"]*"[^>]*>Uninstall<\/button><button[^>]*data-variant="default"[^>]*>Configure<\/button>/);

	const terminal = listing(html, 'Terminal Chords');
	expect(terminal).not.toContain('def5678');
	expect(terminal).toContain('>Linked locally</span>');

	expect(listing(html, 'AeroSpace Chords')).toContain('>Pinned</span>');
	expect(listing(html, 'Chromium Chords')).toContain('Install</button>');
});

test('Browse offers individual installs without an Install All button', () => {
	const html = render({});
	expect(html).toContain('Install</button>');
	expect(html).not.toContain('Install All');
});

test('community packages are behind a trust warning, not a direct link', () => {
	const html = render({});
	expect(html).toContain('>Community chord packages</h3>');
	expect(html).toMatch(/<button[^>]*aria-expanded="false"[^>]*>(?:(?!<button).)*?Search GitHub<\/button>/);
	expect(html).not.toContain('topic%3Achordpack');
});
