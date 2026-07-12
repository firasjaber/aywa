# Aywa user guide

This guide explains how to install and use Aywa on Windows. You do not need developer tools or Python knowledge.

## Download and install Aywa

Aywa does not have a published installer yet. When a release is available:

1. Open the [Aywa releases page](https://github.com/firasjaber/stt/releases).
2. Open the newest release and download its Windows installer.
3. Open the downloaded installer and follow its instructions.
4. If Windows shows a security warning, confirm that the publisher and download source are what you expect before continuing. Early unsigned builds may trigger Microsoft Defender SmartScreen.

Only download Aywa from the project’s GitHub Releases page.

## Open Aywa

Open **Aywa** from the Windows Start menu. Aywa runs in the notification area (system tray), near the clock. If the Settings window is not visible, click the Aywa tray icon. You may need to open the tray’s hidden-icons menu first.

Closing the Settings window does not quit Aywa. To quit, right-click the tray icon and select **Quit**.

## Prepare Aywa for the first time

Aywa needs a local transcription environment and a speech model. The first model activation can take several minutes because Aywa may install Python, PyTorch, and speech-recognition components. These downloads are large. Keep Aywa open and keep the computer connected to the internet.

1. Open the **App** tab.
2. Under **Models**, choose a model:
   - **Parakeet English v0.2** for English dictation.
   - **Parakeet Multilingual v0.3** for supported multilingual speech.
3. Select **Download** and wait for the progress to reach 100%.
4. Select **Activate**.
5. Wait while Aywa shows **Preparing…**. Do not quit Aywa during this step.
6. Aywa is ready when the status says **Ready to dictate** and the model is marked **Active**.

Later launches reuse the installed environment and model, so they should be faster. Aywa may need to prepare it again after an update, a runtime-mode change, or damaged files.

## CPU and NVIDIA GPU modes

Aywa can transcribe in two ways:

- **CPU mode** works on compatible Windows computers without a supported NVIDIA GPU. It is the most broadly compatible choice, but transcription can take longer.
- **NVIDIA CUDA mode** uses a supported NVIDIA graphics card. It can make transcription substantially faster, especially with larger models or longer recordings.

Aywa normally detects your hardware and selects a compatible mode automatically. Its current profiles cover older, current, and newest NVIDIA GPU generations, with CPU as the fallback. Not every NVIDIA GPU or driver is compatible.

For the best chance of CUDA working, install a current driver directly from NVIDIA and restart Windows before opening Aywa. A newer driver improves compatibility, but it cannot add CUDA support to unsupported hardware. Open **Debug** and check **Acceleration** to see whether Aywa reports CUDA as available and which device it is using.

If CUDA is not available, Aywa should fall back to CPU. See [CUDA troubleshooting](TROUBLESHOOTING.md#cuda-is-not-available) if you expected GPU acceleration.

## Dictate text

1. Activate the text box or document where the words should appear.
2. Hold `Ctrl` + `Alt` + `Space`.
3. Speak while the shortcut is held.
4. Release the shortcut when finished.
5. Wait while Aywa transcribes the recording.

Aywa copies the result to the Windows clipboard and then tries to paste it into the active app. If automatic pasting is blocked, the text is still usually on the clipboard: return to the destination and press `Ctrl` + `V`.

The small overlay shows whether Aywa is listening, transcribing, finished, or encountered a problem. For best results, speak clearly and avoid very short or silent recordings.

## Open Settings again

Click the Aywa icon in the Windows system tray, or right-click it and choose **Settings**. Use:

- **App** to see readiness and manage models.
- **Debug** to inspect the system, acceleration, runtime, and model status or locate the debug log.

## Need help?

Start with [Troubleshooting](TROUBLESHOOTING.md). If the problem remains, use [Help and issue reports](SUPPORT.md) to collect diagnostics safely.
