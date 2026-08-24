export type ShortcutCapture =
	| { kind: 'wait' }
	| { kind: 'cancel' }
	| { kind: 'invalid'; message: string }
	| { kind: 'candidate'; value: string };

const modifierCodes = new Set([
	'ControlLeft',
	'ControlRight',
	'AltLeft',
	'AltRight',
	'ShiftLeft',
	'ShiftRight',
	'MetaLeft',
	'MetaRight'
]);

const modifierLabels: Record<string, string> = {
	shift: 'Shift',
	control: 'Ctrl',
	alt: 'Alt',
	super: 'Win'
};

const keyLabel = (value: string) => {
	if (value.startsWith('Key')) return value.slice(3);
	if (value.startsWith('Digit')) return value.slice(5);
	return value;
};

export const captureShortcut = (event: KeyboardEvent): ShortcutCapture => {
	if (event.key === 'Escape') return { kind: 'cancel' };
	if (modifierCodes.has(event.code)) return { kind: 'wait' };

	const modifiers = [
		event.shiftKey ? 'shift' : null,
		event.ctrlKey ? 'control' : null,
		event.altKey ? 'alt' : null,
		event.metaKey ? 'super' : null
	].filter((value): value is string => value !== null);

	if (modifiers.length === 0) {
		return { kind: 'invalid', message: 'Include at least one modifier key' };
	}

	return { kind: 'candidate', value: [...modifiers, event.code].join('+') };
};

export const shortcutLabels = (value: string) =>
	value.split('+').map((part) => modifierLabels[part.toLowerCase()] ?? keyLabel(part));
