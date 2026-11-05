import { expect, test } from 'bun:test';
import {
	advancePractice,
	currentTask,
	initialPractice,
	keysToText,
	type PracticeEvent,
} from '#practice';

const TARGET = '/s';
const run = (events: PracticeEvent[]) =>
	events.reduce((state, event) => advancePractice(state, event, TARGET), initialPractice);

const enter: PracticeEvent = { type: 'mode', chord: true };
const leave: PracticeEvent = { type: 'mode', chord: false };
const typed = (...input: string[]): PracticeEvent => ({ type: 'input', input, executed: null, failed: false });
const executed = (...keys: string[]): PracticeEvent => ({ type: 'input', input: [], executed: keys, failed: false });
const failed = (...keys: string[]): PracticeEvent => ({ type: 'input', input: [], executed: keys, failed: true });

test('reads the keys Chord publishes as what they type', () => {
	expect(keysToText(['Slash', 'KeyS', 'Digit1', 'Comma'])).toBe('/s1,');
});

test('a chord typed correctly and released counts as done', () => {
	const state = run([enter, typed('Slash'), typed('Slash', 'KeyS'), executed('Slash', 'KeyS'), typed(), leave]);
	expect(state.ran).toBe(true);
	expect(state.hint).toBeNull();
});

test('the instruction moves along with the keys', () => {
	expect(currentTask(run([]), TARGET)).toBe('hold');
	expect(currentTask(run([enter]), TARGET)).toBe('type');
	expect(currentTask(run([enter, typed('Slash')]), TARGET)).toBe('type');
	expect(currentTask(run([enter, typed('Slash', 'KeyS')]), TARGET)).toBe('release');
});

test('letting go of Space before typing explains what happened', () => {
	expect(run([enter, leave]).hint).toBe('released-too-soon');
});

test('keys that are no chord are cleared before Space comes up, and still count as typed', () => {
	const state = run([enter, typed('Slash'), typed('Slash', 'KeyX'), typed(), leave]);
	expect(state.hint).toBe('wrong-keys');
	expect(state.typed).toBe('/x');
});

test('the hint waits until the attempt is over, so it never interrupts typing', () => {
	expect(run([enter, typed('Slash', 'KeyX')]).hint).toBeNull();
});

test('another chord that runs is not mistaken for this one', () => {
	const state = run([enter, typed('Digit0', 'Comma'), executed('Digit0', 'Comma'), typed(), leave]);
	expect(state.ran).toBe(false);
	expect(state.hint).toBe('wrong-keys');
	expect(state.typed).toBe('0,');
});

test('keys that are no chord at all explain what happened', () => {
	const state = run([enter, typed('Slash', 'KeyX'), failed('Slash', 'KeyX'), leave]);
	expect(state.ran).toBe(false);
	expect(state.hint).toBe('wrong-keys');
});

test('the right keys failing to run do not count', () => {
	expect(run([enter, failed('Slash', 'KeyS')]).ran).toBe(false);
});

test('trying again clears the last hint and keys', () => {
	const state = run([enter, leave, enter]);
	expect(state.hint).toBeNull();
	expect(state.typed).toBe('');
});

test('a finished exercise stays finished', () => {
	const state = run([enter, executed('Slash', 'KeyS'), leave, enter, leave]);
	expect(state.ran).toBe(true);
	expect(state.hint).toBeNull();
});
