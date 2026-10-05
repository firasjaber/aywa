# Transcription Rules

## Goal and context

Aywa transcribes speech locally, saves successful text to History when enabled, copies it, and tries to paste it. Users need a way to correct words that Aywa often transcribes incorrectly and remove filler words from the final text. This includes product names and words in languages other than English.

## User flow

Add a `Rules` tab to the main window. The tab contains a two-column list with `Heard as` and `Use instead` headings. Users can add, edit, and delete rows. For example, `Hawfa` in `Heard as` and `Haufe` in `Use instead` corrects that name. An empty `Use instead` value removes the matched text, such as `uhm`.

The `Heard as` value must contain a word or phrase. The tab makes the empty replacement behavior clear. Saved changes apply to the next dictation and remain available after the app restarts. Rules and their contents stay on the user's computer.

## Matching and output

Aywa applies rules to the recognized text before it saves the text to History, copies it, or pastes it. Rules affect new dictations only. Existing History entries do not change.

A rule matches a complete word or phrase without regard to letter case. It does not match part of another word. Matching supports words from languages other than English. Aywa inserts replacement text exactly as the user entered it, including its letter case.

Aywa tidies spaces and punctuation around deleted words or phrases. For example, deleting `uhm` changes `Uhm, I agree` to `I agree`. If rules overlap, the longest match wins. Aywa applies rules once to the original recognized text. Replacement text does not trigger another rule.

If rules remove all text, Aywa shows `No text after applying Rules`. It does not add a History entry, change the clipboard, or paste into the active app.

## Scope and success criteria

The first version uses one list for corrections and deletions. It does not change the transcription model or old History entries.

The feature is complete when a user can save, edit, and delete local rules. A complete-word `Hawfa` match becomes `Haufe`. An `uhm` deletion leaves clean surrounding text. History and the clipboard receive the same final text. Aywa attempts to paste that text into the destination app. A rule must not change an unrelated word that merely contains its `Heard as` value. If the final text is empty, Aywa leaves all three destinations unchanged and shows the stated message.
