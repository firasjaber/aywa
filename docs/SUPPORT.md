# Help and issue reports

Please check [Troubleshooting](TROUBLESHOOTING.md) before opening an issue. A focused report with safe diagnostics is much easier to investigate.

## Before collecting diagnostics

1. Update to the newest available Aywa release.
2. Reproduce the problem once and note the exact steps.
3. Record what you expected and what happened instead.
4. Note your Aywa version or source commit, Windows version, and whether you installed a release or built from source.

## Copy the diagnosis shown in Aywa

1. Click the Aywa system-tray icon to open Settings.
2. Open **Debug**.
3. Copy the visible values under **System tools**, **Acceleration**, **Runtime**, and **Model** into your issue.

The current **Copy issue report** button is disabled and marked **Coming later**, so copy the visible diagnosis manually. This limitation is tracked in the [public-release audit](PUBLIC_RELEASE_AUDIT.md).

## Find the debug log

1. In **Debug**, select **Open log**.
2. Windows Explorer selects `aywa-debug.log`.
3. Make a copy only if you need to redact it; do not edit Aywa’s active log in place.

The log is normally stored below Aywa’s Windows local application-data folder. Use the in-app action instead of guessing its exact path.

## Review private information before uploading

Aywa’s log does not intentionally record transcript text, but it does contain sensitive context. Review the entire file and remove anything you do not want to publish, including:

- Your Windows username and local filesystem paths.
- Python, model, recording, and application-data locations.
- GPU and driver details.
- Commands and their complete output, which may include machine-specific information.
- Any dictated text or other private data if a future version or dependency happens to emit it.

Keep the timestamps and error lines around the failure when redacting. Never upload a log without reviewing it.

## Submit the issue

1. Open the [Aywa bug report form](https://github.com/firasjaber/stt/issues/new?template=bug_report.yml).
2. Search existing issues first to avoid duplicates.
3. Complete every required field and paste the Debug diagnosis into the diagnosis box.
4. Drag the reviewed/redacted `aywa-debug.log` into the log attachment area. GitHub uploads it to the issue.
5. Submit the report and respond if a maintainer asks for a focused follow-up test.

If the app cannot open the log, submit the issue without it and explain what happened when you selected **Open log**.
