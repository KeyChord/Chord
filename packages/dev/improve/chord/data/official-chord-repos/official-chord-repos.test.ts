import { expect, test } from 'bun:test';
import { chordsMonorepoUrl, readOfficialChordRepos } from '#gen-official-chord-repos';
import { officialChordReposData } from '#official-chord-repos';

// Without the sibling chords checkout, the committed catalog is all there is to check.
const hasChordsMonorepo = await Bun.file(new URL('package.json', chordsMonorepoUrl)).exists();

test.skipIf(!hasChordsMonorepo)('the catalog matches each package.json (regenerate with `vpx _gen-official-chord-repos`)', async () => {
	expect(officialChordReposData).toEqual(await readOfficialChordRepos());
});
