from pathlib import Path
import sys
import time


class NemoTranscriber:
    def __init__(self, model_id: str, model_path: str):
        self.model_id = model_id
        self.model_path = model_path
        self.device = "cpu"
        self._model = self._load_model(model_path)

    def _load_model(self, model_path: str):
        import torch
        from nemo.collections.asr.models import EncDecMultiTaskModel

        if torch.cuda.is_available():
            try:
                model = EncDecMultiTaskModel.restore_from(model_path)
                self.device = "cuda"
                return model
            except Exception as exc:
                if not is_cuda_load_error(exc):
                    raise

                print(
                    f"[worker] CUDA startup failed, retrying on CPU error={exc}",
                    file=sys.stderr,
                    flush=True,
                )

        model = EncDecMultiTaskModel.restore_from(model_path, map_location=torch.device("cpu"))
        self.device = "cpu"
        return model

    def transcribe(self, audio_path: str) -> dict:
        started = time.perf_counter()
        print(f"[worker] transcribe start audio_path={audio_path}", file=sys.stderr, flush=True)
        output = self._model.transcribe([audio_path])
        text = extract_transcript_text(output)
        print(
            f"[worker] transcribe decoded device={self.device} output_type={type(output).__name__} text_chars={len(text)}",
            file=sys.stderr,
            flush=True,
        )
        if not text:
            raise ValueError("Transcription was empty. Recording may be silent.")
        return {
            "text": text,
            "duration_ms": int((time.perf_counter() - started) * 1000),
        }


def extract_transcript_text(output) -> str:
    item = output[0] if isinstance(output, list) and output else output
    if item is None:
        return ""

    text = getattr(item, "text", item)
    return str(text).strip()


def is_cuda_load_error(exc: Exception) -> bool:
    message = str(exc).lower()
    markers = [
        "cuda error",
        "no kernel image is available",
        "device-side assert",
        "cublas",
        "cudnn",
        "cuda driver",
        "cuda-capable device",
    ]
    return any(marker in message for marker in markers)


def build_transcriber(model_id: str, model_path: str):
    path = Path(model_path)
    if not path.exists():
        raise FileNotFoundError(model_path)
    return NemoTranscriber(model_id, str(path))
