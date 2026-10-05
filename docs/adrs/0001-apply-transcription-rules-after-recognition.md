# ADR 0001: Apply transcription rules after recognition

Status: Accepted
Date: 2026-10-05

## Context

Aywa recognizes speech locally, then saves the text when History is enabled, copies it, and attempts to paste it. Users need repeatable corrections and filler word removal across these outputs.

## Decision

Store the ordered rule list in Aywa's existing local `state/settings.json` document. Older settings files load an empty list by default.

Apply rules in the Rust app after the Python worker returns text and before History, clipboard, or paste receives it. Match complete words or phrases without regard to letter case. Use the longest overlapping match and process the original text once. An empty replacement removes the match.

## Consequences

History receives the same processed text as the clipboard and paste attempt when saving is enabled. Existing History entries do not change. Rule text stays in the local settings file. Rules cannot change what the speech model recognizes.

## Alternatives

Passing rules to the Python worker or speech model would couple this feature to recognition behavior. Post-processing gives predictable replacements and one shared output path.

## References

- [Approved spec](../specs/2026-10-05-transcription-rules.md)
- [Implementation plan](../implementation-plans/2026-10-05-transcription-rules.md)
- [Settings](../../src-tauri/src/settings.rs)
- [Output path](../../src-tauri/src/lib.rs)
