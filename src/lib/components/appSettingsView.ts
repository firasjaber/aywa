export const HISTORY_PREFERENCE_DESCRIPTION =
	'When off, future transcriptions are not saved. Existing History remains until you clear it.';

export type HistoryPreferenceState = {
	loaded: boolean;
	saving: boolean;
	confirmed: boolean;
	displayed: boolean;
	error: string | null;
};

export const initialHistoryPreferenceState = (): HistoryPreferenceState => ({
	loaded: false,
	saving: false,
	confirmed: true,
	displayed: true,
	error: null
});

export const historyPreferenceLoaded = (value: boolean): HistoryPreferenceState => ({
	loaded: true,
	saving: false,
	confirmed: value,
	displayed: value,
	error: null
});

export const historyPreferenceLoadFailed = (error: string): HistoryPreferenceState => ({
	...initialHistoryPreferenceState(),
	error
});

export const beginHistoryPreferenceSave = (
	state: HistoryPreferenceState,
	displayed: boolean
): HistoryPreferenceState => ({ ...state, saving: true, displayed, error: null });

export const historyPreferenceSaved = (
	state: HistoryPreferenceState,
	confirmed: boolean
): HistoryPreferenceState => ({
	...state,
	saving: false,
	confirmed,
	displayed: confirmed,
	error: null
});

export const historyPreferenceSaveFailed = (
	state: HistoryPreferenceState,
	error: string
): HistoryPreferenceState => ({
	...state,
	saving: false,
	displayed: state.confirmed,
	error
});
