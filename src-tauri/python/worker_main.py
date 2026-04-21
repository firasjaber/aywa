import argparse
import os
import sys

from worker_protocol import encode_message, parse_line


def setup_protocol_stream():
    protocol_fd = os.dup(sys.stdout.fileno())
    os.dup2(sys.stderr.fileno(), sys.stdout.fileno())
    sys.stdout = sys.stderr
    return os.fdopen(protocol_fd, "w", encoding="utf-8", buffering=1, newline="\n")


PROTOCOL_STREAM = setup_protocol_stream()

from transcriber import build_transcriber


def emit(message: dict) -> None:
    PROTOCOL_STREAM.write(encode_message(message))
    PROTOCOL_STREAM.flush()


def log(message: str) -> None:
    print(f"[worker] {message}", file=sys.stderr, flush=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model-id", required=True)
    parser.add_argument("--model-path", required=True)
    args = parser.parse_args()

    try:
        transcriber = build_transcriber(args.model_id, args.model_path)
    except Exception as exc:
        log(f"startup failed error={exc}")
        emit({"type": "fatal", "message": str(exc)})
        return 1

    log(f"ready model_id={args.model_id} model_path={args.model_path}")
    emit(
        {
            "type": "ready",
            "model_id": args.model_id,
            "model_path": args.model_path,
            "device": transcriber.device,
        }
    )

    for raw_line in sys.stdin:
        try:
            message = parse_line(raw_line)
        except Exception as exc:
            emit({"type": "fatal", "message": f"protocol error: {exc}"})
            return 1

        if message["type"] == "shutdown":
            log("shutdown requested")
            return 0

        if message["type"] == "ping":
            emit({"type": "pong"})
            continue

        if message["type"] == "transcribe":
            log(
                f"transcribe request_id={message['request_id']} audio_path={message['audio_path']}"
            )
            try:
                result = transcriber.transcribe(message["audio_path"])
                log(
                    f"transcribe success request_id={message['request_id']} chars={len(result['text'])} duration_ms={result['duration_ms']}"
                )
                emit(
                    {
                        "type": "transcribe_result",
                        "request_id": message["request_id"],
                        "text": result["text"],
                        "duration_ms": result["duration_ms"],
                    }
                )
            except Exception as exc:
                log(f"transcribe error request_id={message.get('request_id')} error={exc}")
                emit(
                    {
                        "type": "error",
                        "request_id": message.get("request_id"),
                        "message": str(exc),
                    }
                )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
