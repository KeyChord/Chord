import { afterAll, expect, test } from 'bun:test';
import { QueryClient, QueryClientProvider } from '@chord/com.npmjs.tanstack__react-query';
import { renderToString } from 'react-dom/server';
import { initialPractice } from '#practice';

// Effects never run during server rendering; TauRPC needs the browser's Proxy constructor.
const previousWindow = globalThis.window;
globalThis.window = { Proxy, screen: {} } as unknown as Window & typeof globalThis;
const { PermissionStep } = await import('#step-permission');
const { PracticeView } = await import('#step-practice');
const { PacksView } = await import('#step-packs');
const { WelcomeStep } = await import('#step-welcome');
const { DoneStep } = await import('#step-done');
const { MoveStep } = await import('#step-move');
const { ChordKeysPanel } = await import('#step-first-chord');
afterAll(() => {
	globalThis.window = previousWindow;
});

function render(node: React.ReactNode) {
	return renderToString(
		<QueryClientProvider client={new QueryClient()}>{node}</QueryClientProvider>,
	).replaceAll('<!-- -->', '');
}

test('welcome says what Chord does and names the keys in words', () => {
	const html = render(<WelcomeStep onContinue={() => {}} onSkip={() => {}} />);
	expect(html).toContain('Get Started');
	expect(html).toContain('caps lock');
	expect(html).toContain('Skip Setup');
});

test('the permission step uses the name of the page macOS actually shows', () => {
	const html = render(<PermissionStep paneName="Device Control and Data Access" stepNumber={1} stepCount={4} onSkip={() => {}} />);
	expect(html).toContain('Device Control and Data Access');
	expect(html).toContain('Open System Settings');
	expect(html).toContain('Modify Settings');
});

test('the permission step prepares people for the wording macOS uses', () => {
	const html = render(<PermissionStep paneName="Accessibility" stepNumber={1} stepCount={4} onSkip={() => {}} />);
	expect(html).toContain('monitor your keyboard');
	expect(html).toContain('doesn&#x27;t record what you type');
});

test('offering to move Chord explains why and what to click', () => {
	const html = render(<MoveStep onSkip={() => {}} />);
	expect(html).toContain('Move to Applications');
	expect(html).toContain('stop working');
});

test('practice starts by showing where caps lock and space are', () => {
	const html = render(<PracticeView progress={initialPractice} sequence="/s" appName="Safari" />);
	expect(html).toContain('caps lock');
	expect(html).toContain('keep holding space');
	expect(html).toMatch(/data-key="caps"[^>]*class="[^"]*bg-primary/);
	expect(html).toMatch(/data-key="space"[^>]*class="[^"]*bg-primary/);
});

test('practice highlights the next keys and shows them being pressed', () => {
	const html = render(
		<PracticeView progress={{ ...initialPractice, inChordMode: true, typed: '/' }} sequence="/s" appName="Safari" />,
	);
	expect(html).toMatch(/data-key="\/"[^>]*data-pressed="true"/);
	expect(html).toMatch(/data-key="s"[^>]*class="[^"]*bg-primary/);
	expect(html).toContain('typed: ');
});

test('practice explains letting go of space too soon', () => {
	const html = render(
		<PracticeView progress={{ ...initialPractice, hint: 'released-too-soon' }} sequence="/s" appName="Safari" />,
	);
	expect(html).toContain('let go of space before typing');
});

test('practice explains typing something else', () => {
	const html = render(
		<PracticeView progress={{ ...initialPractice, typed: '/x', hint: 'wrong-keys' }} sequence="/s" appName="Safari" />,
	);
	expect(html).toContain('/x');
	expect(html).toContain('/s');
});

test('practice does not call the right keys wrong when the chord could not run', () => {
	const html = render(
		<PracticeView
			progress={{ ...initialPractice, typed: '/s', attempted: '/s', hint: 'did-not-run', error: 'No chord found' }}
			sequence="/s"
			appName="Safari"
		/>,
	);
	expect(html).toContain('correctly');
	expect(html).toContain('No chord found');
	expect(html).not.toContain('No harm done');
});

const keysPanel = {
	appName: 'Todoist',
	keys: 't',
	isAllowing: false,
	onKeysChange: () => {},
	onAllowFolder: () => {},
};

test('a letter another chord uses names that chord and asks for a different one', () => {
	const conflict = { packageName: 'p', chordsFilePathslug: 'f', chord: { name: 'Terminal' } } as never;
	const html = render(<ChordKeysPanel {...keysPanel} conflict={conflict} />);
	expect(html).toContain('/t');
	expect(html).toContain('already used by');
	expect(html).toContain('Terminal');
	expect(html).toContain('different letter');
});

test('a free letter shows no conflict', () => {
	expect(render(<ChordKeysPanel {...keysPanel} />)).not.toContain('already used by');
});

test('a chords folder Chord may not use explains why and offers to allow it', () => {
	const blockedFolder = { name: 'default', path: '/Users/me/.chord/packages/default', identity: 'id', approved: false };
	const html = render(<ChordKeysPanel {...keysPanel} blockedFolder={blockedFolder} />);
	expect(html).toContain('/Users/me/.chord/packages/default');
	expect(html).toContain('wouldn&#x27;t run');
	expect(html).toContain('can run code');
	expect(html).toContain('Allow This Folder');
});

test('practice celebrates the chord running', () => {
	const html = render(
		<PracticeView progress={{ ...initialPractice, ran: true }} sequence="/s" appName="Safari" />,
	);
	expect(html).toContain('opened Safari');
});

test('packs show which are added, and every official pack is offered', () => {
	const html = render(
		<PacksView installedSlugs={new Set(['keychord/chords-vscode'])} onAdd={() => {}} />,
	);
	expect(html).toContain('VSCode Chords');
	expect(html).toContain('Added');
	expect(html).toContain('Add Chromium Chords');
	expect(html).not.toContain('Add VSCode Chords');
});

test('a pack still loading its chords says so, even once it is saved', () => {
	const html = render(
		<PacksView
			installedSlugs={new Set(['keychord/chords-terminal'])}
			pendingUrl="https://github.com/KeyChord/chords-terminal"
			onAdd={() => {}}
		/>,
	);
	expect(html).toContain('Adding…');
	expect(html).not.toContain('Added');
});

test('finishing without the permission warns that chords will not work', () => {
	expect(render(<DoneStep isPermissionGranted={false} onFinish={() => {}} />)).toContain('won&#x27;t work');
	expect(render(<DoneStep isPermissionGranted onFinish={() => {}} />)).not.toContain('won&#x27;t work');
});
