# Help and issue reports

Check [Troubleshooting](TROUBLESHOOTING.md) before opening an issue. A focused report with safe diagnostics is easier to investigate.

## Before collecting diagnostics

1. Update to the newest available Aywa release.
2. Reproduce the problem once and note the exact steps.
3. Record what you expected and what happened instead.
4. Note your Aywa version or source commit, Windows version, and whether you installed a release or built from source.

## Preview and copy the safe issue report

1. Click the Aywa system-tray icon to open Settings.
2. Open **Debug**.
3. Select **Preview issue report**.
4. Review the exact text, then select **Copy reviewed report** and paste it into the issue.

The report is generated locally and never sent automatically. It excludes recordings, transcripts, file paths, usernames, command arguments, and raw subprocess output.

## Find a local log

1. In **Debug**, select **Open local log**.
2. Windows Explorer selects the current per-launch log.
3. Make a copy only if you need to redact it; do not edit Aywa’s active log in place.

Logs are local, one per launch, and retain at most 1 GB total. Aywa removes the oldest sessions first. Review a raw log before sharing it: it can still contain machine-specific context.

## Submit the issue

1. Open the [Aywa bug report form](https://github.com/firasjaber/aywa/issues/new?template=bug_report.yml).
2. Search existing issues first to avoid duplicates.
3. Complete every required field and paste the reviewed report into the diagnosis box.
4. Attach a reviewed/redacted session log only if support asks for it.
5. Submit the report and respond if a maintainer asks for a focused follow-up test.

If the app cannot open the log, submit the issue without it and explain what happened when you selected **Open local log**.
