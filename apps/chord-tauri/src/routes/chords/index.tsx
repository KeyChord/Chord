import { ChordsPage } from '@chord/dev.improve.chord.routes.chords.index';
import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/chords/')({
	component: ChordsPage,
});
