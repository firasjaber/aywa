import sys
from pathlib import Path

import pytest

sys.path.append(str(Path(__file__).resolve().parents[1]))

from worker_protocol import encode_message, parse_line
from transcriber import build_transcriber, extract_transcript_text, is_cuda_load_error


def test_parse_transcribe_request():
    message = parse_line('{"type":"transcribe","request_id":"req-1","audio_path":"C:/temp/audio.wav"}')
    assert message["type"] == "transcribe"
    assert message["request_id"] == "req-1"


def test_encode_ready_message():
    line = encode_message(
        {
            "type": "ready",
            "model_id": "parakeet-v0.2-en",
            "model_path": "C:/models/model.nemo",
        }
    )
    assert line == '{"type":"ready","model_id":"parakeet-v0.2-en","model_path":"C:/models/model.nemo"}\n'


def test_build_transcriber_rejects_missing_model(tmp_path):
    with pytest.raises(FileNotFoundError):
        build_transcriber("parakeet-v0.2-en", str(tmp_path / "missing.nemo"))


class _HypothesisLike:
    def __init__(self, text: str):
        self.text = text


def test_extract_transcript_text_reads_hypothesis_text():
    assert extract_transcript_text([_HypothesisLike("hello world")]) == "hello world"


def test_extract_transcript_text_strips_plain_strings():
    assert extract_transcript_text(["  hello world  "]) == "hello world"


def test_is_cuda_load_error_detects_kernel_image_failure():
    assert is_cuda_load_error(RuntimeError("CUDA error: no kernel image is available for execution on the device"))


def test_build_transcriber_retries_on_cpu_when_cuda_load_fails(monkeypatch, tmp_path):
    calls: list[object] = []

    class _FakeTorchCuda:
        @staticmethod
        def is_available():
            return True

    class _FakeTorch:
        cuda = _FakeTorchCuda()

        @staticmethod
        def device(name: str):
            return name

    class _FakeModel:
        @staticmethod
        def restore_from(model_path, map_location=None):
            calls.append(map_location)
            if map_location is None:
                raise RuntimeError("CUDA error: no kernel image is available for execution on the device")
            return object()

    monkeypatch.setitem(sys.modules, "torch", _FakeTorch)
    monkeypatch.setitem(sys.modules, "nemo.collections.asr.models", type("_FakeModule", (), {"EncDecMultiTaskModel": _FakeModel}))

    model_path = tmp_path / "model.nemo"
    model_path.write_text("stub")

    transcriber = build_transcriber("parakeet-v0.2-en", str(model_path))

    assert transcriber.device == "cpu"
    assert calls == [None, "cpu"]
