import json


def parse_line(line: str) -> dict:
    message = json.loads(line)
    if "type" not in message:
        raise ValueError("protocol message missing type")
    return message


def encode_message(message: dict) -> str:
    return json.dumps(message, separators=(",", ":")) + "\n"
