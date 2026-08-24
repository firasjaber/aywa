# Troubleshooting Aywa

Try the relevant safe recovery steps below. If the problem remains, collect evidence using the [support guide](SUPPORT.md).

## Aywa appears not to start

Aywa normally starts in the Windows system tray with its Settings window hidden.

1. Check the notification area near the clock, including hidden icons.
2. Click the Aywa icon or right-click it and choose **Settings**.
3. Check Task Manager for Aywa before launching a second copy.
4. If Aywa is not running, start it from the Start menu.
5. If it immediately exits, reinstall it and attach the debug log if one was created.

## First preparation is taking a long time

The first model activation may install Python, PyTorch, NVIDIA/CPU runtime packages, and NeMo dependencies. This is a large download and can take several minutes.

- Keep Aywa open and maintain a stable internet connection.
- Make sure at least 15 GB of free disk space is available during setup.
- Avoid switching runtime mode or activating another model while preparation is running.
- Open **Debug** and inspect **Runtime** for the current stage or error.

## The shortcut does nothing

Aywa uses `Ctrl` + `Alt` + `Space` as its default hold-to-talk shortcut. The active shortcut is shown on the **Settings** tab.

- Confirm the App status says **Ready to dictate** and a model is **Active**.
- Hold the shortcut shown on the Settings tab rather than tapping it, speak, and then release it.
- If Aywa reports that a new shortcut is unavailable, close software that may own it or choose a different combination. The previous shortcut remains active.
- Choose **Reset to default** on the Settings tab to restore `Ctrl` + `Alt` + `Space`.
- If the shortcut never works, open the debug log and look for `failed to register global hotkey`.

## Aywa cannot record the microphone

- In Windows **Settings > Privacy & security > Microphone**, allow microphone access for desktop apps.
- Confirm the intended microphone is the Windows default input device.
- Test the microphone in Windows Sound settings.
- Close applications holding the device exclusively, then restart Aywa.
- Include the microphone error from the overlay or log in a bug report.

## Transcription fails or is empty

- Confirm a model is marked **Active**.
- Speak clearly for more than a brief moment and avoid silence-only recordings.
- Open **Debug** and check that **Runtime** is **Ready**.
- Try activating the model again. If its row says **Needs attention**, download it again.
- Restart Aywa after changing audio devices or NVIDIA drivers.

## Text is not inserted into the active app

Aywa first copies the transcription and then simulates `Ctrl` + `V`.

1. Return to the destination app and press `Ctrl` + `V` manually.
2. Make sure a writable text field had focus when recording ended.
3. Try a standard app such as Notepad; elevated or protected applications may block simulated input from a non-elevated app.
4. Do not run Aywa as administrator merely to bypass another app’s input restrictions.

## A model download fails

- Confirm internet access to `huggingface.co`.
- Check available disk space; current model files are approximately 2.5 GB and temporary/cache copies require extra space.
- Disable neither antivirus nor firewall globally. If a security product blocks the download, review its event and allow only the verified Aywa process or domain according to your organization’s policy.
- Select **Download** again. Aywa verifies the model checksum before installation.
- Attach the log if the failure repeats.

## Runtime or Python preparation fails

- Reinstall Aywa if Debug says bundled Python runtime files are missing.
- Confirm access to Python/PyPI and `download.pytorch.org` is not blocked by a proxy or firewall.
- Ensure enough free space is available and that security software is not quarantining the runtime under Aywa’s local application data.
- Do not install a separate Python version to repair a packaged Aywa release; Aywa manages its own runtime.

## CUDA is not available

CPU fallback remains usable, but it is slower.

1. Confirm the computer has an NVIDIA GPU.
2. Install a current NVIDIA driver from NVIDIA, restart Windows, and reopen Aywa.
3. Open **Debug** and check **System tools** for the GPU and driver and **Acceleration** for CUDA status.
4. Aywa’s profiles currently require driver 522.06 or newer for legacy GPUs, 551.61 or newer for current GPUs, and 572.16 or newer for the newest preview profile. Hardware compatibility also depends on the GPU generation.
5. If Aywa reports CPU despite compatible hardware, include the Debug details and log in an issue.

## Transcription is slow

- Check **Debug > Acceleration** to see whether Aywa is using **CPU** or **CUDA**.
- Install a current NVIDIA driver if compatible NVIDIA hardware is present.
- Shorter recordings finish sooner. CPU transcription of large or long inputs can be noticeably slower.
- The first transcription after activation may be slower while the model warms up.

## Open local log does not work

The **Open local log** action currently uses Windows Explorer.

- Confirm Aywa has completed startup so its current session log exists.
- Restart Aywa and try **Debug > Open local log** again.
- If Explorer still does not open, mention that the log is unavailable when filing the issue; the bug form does not require an attachment.
