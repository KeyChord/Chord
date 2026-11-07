import { expect, test } from 'bun:test';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { chordsMonorepoUrl, readOfficialChordRepos } from '#gen-official-chord-repos';
import { officialChordReposData } from '#official-chord-repos';

// Without the sibling chords checkout, the committed catalog is all there is to check.
const hasChordsMonorepo = await Bun.file(new URL('package.json', chordsMonorepoUrl)).exists();

test.skipIf(!hasChordsMonorepo)('the catalog matches each package.json (regenerate with `vpx _gen-official-chord-repos`)', async () => {
	expect(officialChordReposData).toEqual(await readOfficialChordRepos());
});

test('lists public packages by display name, leaving out the ones Chord installs itself', async () => {
	const dir = await mkdtemp(join(tmpdir(), 'official-chord-repos-'));
	try {
		const writePackage = (name: string, fields: object) =>
			Bun.write(join(dir, `@keychord/${name}/package.json`), JSON.stringify({ name: `@keychord/${name}`, ...fields }));
		const listing = (displayName: string, name: string) => ({
			displayName,
			description: 'Chords.',
			categories: ['Tools'],
			repository: { url: `https://github.com/KeyChord/${name}` },
		});

		await writePackage('chords-zed', listing('Zed Chords', 'chords-zed'));
		await writePackage('chords-alpha', listing('alpha Chords', 'chords-alpha'));
		await writePackage('chords-built-in', listing('Built-in Chords', 'chords-built-in'));
		await writePackage('tool', { private: true });
		await Bun.write(join(dir, '@keychord/leftover/readme.md'), '');
		await Bun.write(join(dir, 'defaults.toml'), '[packages."@keychord/chords-built-in"]\ngit = "https://github.com/KeyChord/chords-built-in"\n');

		const repos = await readOfficialChordRepos(Bun.pathToFileURL(`${dir}/`), Bun.pathToFileURL(join(dir, 'defaults.toml')));
		expect(repos.map(repo => repo.packageName)).toEqual(['@keychord/chords-alpha', '@keychord/chords-zed']);
	} finally {
		await rm(dir, { recursive: true });
	}
});
