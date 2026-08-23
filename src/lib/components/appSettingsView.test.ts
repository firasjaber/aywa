import { describe, expect, it } from 'vitest';
import {
	HISTORY_PREFERENCE_DESCRIPTION,
	beginHistoryPreferenceSave,
	historyPreferenceLoadFailed,
	historyPreferenceLoaded,
	historyPreferenceSaveFailed,
	historyPreferenceSaved,
	initialHistoryPreferenceState
} from './appSettingsView';

describe('history preference view state', () => {
	it('starts disabled until the persisted setting loads', () => {
		const state = initialHistoryPreferenceState();
		expect(state).toEqual({
			loaded: false,
			saving: false,
			confirmed: true,
			displayed: true,
			error: null
		});
	});

	it('loads the persisted value', () => {
		expect(historyPreferenceLoaded(false)).toMatchObject({
			loaded: true,
			confirmed: false,
			displayed: false
		});
	});

	it('disables the control while saving and confirms success', () => {
		const saving = beginHistoryPreferenceSave(historyPreferenceLoaded(true), false);
		expect(saving).toMatchObject({ saving: true, confirmed: true, displayed: false });
		expect(historyPreferenceSaved(saving, false)).toMatchObject({
			saving: false,
			confirmed: false,
			displayed: false
		});
	});

	it('reverts to the last confirmed value after a failed save', () => {
		const saving = beginHistoryPreferenceSave(historyPreferenceLoaded(true), false);
		expect(historyPreferenceSaveFailed(saving, 'disk full')).toMatchObject({
			saving: false,
			confirmed: true,
			displayed: true,
			error: 'disk full'
		});
	});

	it('reports load failure while preserving the safe displayed default', () => {
		expect(historyPreferenceLoadFailed('unavailable')).toMatchObject({
			loaded: false,
			displayed: true,
			error: 'unavailable'
		});
	});

	it('explains that disabling applies only to future transcripts', () => {
		expect(HISTORY_PREFERENCE_DESCRIPTION).toBe(
			'When off, future transcriptions are not saved. Existing History remains until you clear it.'
		);
	});
});
