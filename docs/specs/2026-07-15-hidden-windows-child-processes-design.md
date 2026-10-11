# Hidden Windows Child Processes Design

## Problem

The packaged Aywa executable is built as a Windows GUI application and correctly does not create its own console. Its internal child processes, however, are started with the Rust standard-library defaults. When the GUI application starts console-based executables such as `python.exe`, `pip`, and `nvidia-smi`, Windows can create visible terminal windows for those children. Runtime provisioning launches several such processes and model activation launches the long-running Python worker, so the terminals can interrupt the user's focused window.

Runtime provisioning and model activation already run on background threads. The change must preserve that behavior and must not move any blocking process wait, download, installation, probe, or worker startup onto the UI thread.

## Chosen Approach

Create a small Windows-specific command-configuration helper in the Rust backend. It will apply `CREATE_NO_WINDOW` to commands that Aywa starts for its own internal work. On non-Windows builds it will be a no-op, keeping the backend source portable enough for existing checks.

Use the helper at every internal process launch:

- bundled Python version checks and pip bootstrap;
- managed runtime package installations;
- Python runtime and CUDA probes;
- Python smoke tests;
- the persistent Python transcription worker; and
- `nvidia-smi` machine detection.

The helper will not be applied to the explicit `explorer.exe` action that opens the diagnostics-log location, because that command intentionally opens a user-facing Windows application.

## Behavior

Internal subprocesses remain child processes with the same arguments, environment, standard-input configuration, and stdout/stderr capture as before. Only their Windows process-creation flags change. Command output remains available to the existing diagnostics pipeline and protocol readers; failures continue to surface through Settings and model state instead of a terminal window.

The existing background bootstrap thread continues to perform runtime preparation. The existing background model-activation thread continues to start the worker and wait for its readiness. UI commands keep their current quick-return and progress-event behavior.

## Verification

Unit tests will verify that the helper is callable for a configured `Command` and that Windows builds select the no-console flag. The Rust test suite and TypeScript checks will run after the change. A release build will confirm the packaged executable still builds with the existing GUI subsystem setting.
