import { afterAll, expect, test } from 'bun:test';
import { QueryClient, QueryClientProvider } from '@chord/com.npmjs.tanstack__react-query';
import { officialChordReposData } from '@chord/dev.improve.chord.data.official-chord-repos';
import { type GitReposState, type PackageUpdate, PackageUpdateSource } from '@chord/dev.improve.chord.lib.typeshare';
import { renderToString } from 'react-dom/server';

// Mutations never run during server rendering; TauRPC needs the browser's Proxy constructor.
const previousWindow = globalThis.window;
globalThis.window = { Proxy, screen: {} } as unknown as Window & typeof globalThis;
const { SettingsBrowsePageContent } = await import('#settings-browse-page');
const { PackageUpdates, formatCheckedAt } = await import('#package-updates');
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
			<SettingsBrowsePageContent state={state} />
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
	const categories = [...new Set(officialChordReposData.flatMap(repo => repo.categories))];
	expect(labels).toEqual(['All', ...categories.sort((a, b) => a.localeCompare(b, 'en'))]);
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
		officialChordReposData
			.map(repo => repo.url.split('/').at(-1)!)
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

const minute = 60 * 1000;
const sha = (c: string) => c.repeat(40);

function update(name: string, source: PackageUpdateSource, from = 'a', to = 'b'): PackageUpdate {
	return { slug: `KeyChord/${name}`, installedRev: sha(from), rev: sha(to), source };
}

function renderUpdates(fields: Partial<GitReposState>, now = 1_000 * minute) {
	const repos = fields.repos ?? {};
	return renderToString(
		<QueryClientProvider client={new QueryClient()}>
			<PackageUpdates
				repos={repos}
				updates={fields.updates ?? {}}
				updateCheck={fields.updateCheck ?? { checking: false, failed: [] }}
				displayNames={new Map([['keychord/chords-raycast', 'Raycast Chords']])}
				now={now}
			/>
		</QueryClientProvider>,
	).replaceAll('<!-- -->', '');
}

function updateRow(html: string, name: string) {
	const match = html.match(new RegExp(`<li[^>]*>(?:(?!</li>).)*?>${name}</span>.*?</li>`));
	expect(match).not.toBeNull();
	return match![0];
}

test('updates sit between the page heading and the catalog', () => {
	const html = render({});
	expect(html.indexOf('>Browse</h2>')).toBeLessThan(html.indexOf('>Updates</h3>'));
	expect(html.indexOf('>Updates</h3>')).toBeLessThan(html.indexOf('aria-label="Search chord repos"'));
});

test('each update shows its package, where it comes from and both commits', () => {
	const html = renderUpdates({
		repos: {
			'KeyChord/chords-raycast': gitRepo('chords-raycast'),
			'KeyChord/chords-menu': gitRepo('chords-menu'),
		},
		updates: {
			'KeyChord/chords-raycast': update('chords-raycast', PackageUpdateSource.Latest, 'c', 'd'),
			'KeyChord/chords-menu': update('chords-menu', PackageUpdateSource.BuiltIn),
		},
		updateCheck: { checking: false, checkedAt: 995 * minute, failed: [] },
	});
	expect(html).toContain('2 packages have an update. Checked 5 minutes ago');

	// Catalog names where there is one, otherwise the repository's name.
	const raycast = updateRow(html, 'Raycast Chords');
	expect(raycast).toContain('>Latest</span>');
	expect(raycast).toMatch(/>cccccc[c]<\/span>.*>ddddddd<\/span>/);
	expect(raycast).toContain('>Update</button>');
	const menu = updateRow(html, 'chords-menu');
	expect(menu).toContain('>Built in</span>');
	expect(html.indexOf('>chords-menu</span>')).toBeLessThan(html.indexOf('>Raycast Chords</span>'));
	expect(html).toMatch(/<button[^>]*>(?:(?!<button).)*?Update All<\/button>/);
});

test('a single update has no Update All, and none at all says everything is up to date', () => {
	const repos = { 'KeyChord/chords-menu': gitRepo('chords-menu') };
	const one = renderUpdates({
		repos,
		updates: { 'KeyChord/chords-menu': update('chords-menu', PackageUpdateSource.BuiltIn) },
		updateCheck: { checking: false, checkedAt: 1_000 * minute, failed: [] },
	});
	expect(one).toContain('1 package has an update. Checked just now');
	expect(one).not.toContain('Update All');

	const none = renderUpdates({ repos, updateCheck: { checking: false, checkedAt: 900 * minute, failed: [] } });
	expect(none).toContain('Every package is up to date. Checked 1 hour ago');
	expect(none).not.toContain('<ul');
});

test('a running check disables Check Again, and unreachable packages are named', () => {
	const checking = renderUpdates({ updateCheck: { checking: true, failed: ['KeyChord/chords-raycast'] } });
	expect(checking).toContain('Checking for updates...');
	expect(checking).toMatch(/<button[^>]*disabled=""[^>]*>(?:(?!<button).)*?Check Again<\/button>/);
	expect(checking).not.toContain("Couldn't reach");

	const failed = renderUpdates({ updateCheck: { checking: false, checkedAt: 999 * minute, failed: ['KeyChord/chords-raycast'] } });
	expect(failed).toContain("Couldn&#x27;t reach Raycast Chords.");
	expect(renderUpdates({})).toContain('Not checked yet');
});

test('an update for a package that is no longer installed is not listed', () => {
	const html = renderUpdates({
		updates: { 'KeyChord/chords-menu': update('chords-menu', PackageUpdateSource.BuiltIn) },
	});
	expect(html).not.toContain('chords-menu');
});

test('check times read like a person would say them', () => {
	const now = 10_000 * minute;
	expect(formatCheckedAt(now - 30_000, now)).toBe('just now');
	expect(formatCheckedAt(now - minute, now)).toBe('1 minute ago');
	expect(formatCheckedAt(now - 3 * 60 * minute, now)).toBe('3 hours ago');
	expect(formatCheckedAt(now - 24 * 60 * minute, now)).toBe('yesterday');
	expect(formatCheckedAt(now + minute, now)).toBe('just now');
});
