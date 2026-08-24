import { describe, expect, it } from 'vitest';
import { captureShortcut, shortcutLabels } from './shortcutView';

const key = (overrides: Partial<KeyboardEvent>) =>
	({
		key: 'd',
		code: 'KeyD',
		ctrlKey: false,
		altKey: false,
		shiftKey: false,
		metaKey: false,
		...overrides
	}) as KeyboardEvent;

describe('shortcut recorder helpers', () => {
	it('waits for a non-modifier key', () => {
		expect(captureShortcut(key({ key: 'Control', code: 'ControlLeft', ctrlKey: true }))).toEqual({
			kind: 'wait'
		});
	});

	it('normalizes a modified key chord', () => {
		expect(captureShortcut(key({ ctrlKey: true, altKey: true }))).toEqual({
			kind: 'candidate',
			value: 'control+alt+KeyD'
		});
	});

	it('rejects an unmodified ordinary key', () => {
		expect(captureShortcut(key({}))).toEqual({
			kind: 'invalid',
			message: 'Include at least one modifier key'
		});
	});

	it('cancels capture with Escape', () => {
		expect(captureShortcut(key({ key: 'Escape', code: 'Escape' }))).toEqual({
			kind: 'cancel'
		});
	});

	it('renders backend-normalized labels', () => {
		expect(shortcutLabels('shift+control+KeyD')).toEqual(['Shift', 'Ctrl', 'D']);
		expect(shortcutLabels('super+Space')).toEqual(['Win', 'Space']);
	});
});
