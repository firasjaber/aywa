import argparse
import json
import sys


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile-id", required=True)
    parser.add_argument("--expect-gpu", required=True)
    args = parser.parse_args()
    expect_gpu = args.expect_gpu.lower() == "true"

    result = {
        "success": False,
        "device": "cpu_fallback",
        "torch_version": None,
        "cuda_version": None,
        "gpu_name": None,
        "error": None,
    }

    try:
        import torch

        result["torch_version"] = getattr(torch, "__version__", None)
        result["cuda_version"] = getattr(getattr(torch, "version", None), "cuda", None)

        if expect_gpu:
            if not torch.cuda.is_available():
                raise RuntimeError("torch.cuda.is_available() returned false")
            tensor = torch.tensor([1.0], device="cuda")
            result["gpu_name"] = torch.cuda.get_device_name(0)
            result["device"] = "gpu"
            result["success"] = bool(float(tensor.sum().item()) == 1.0)
            if not result["success"]:
                raise RuntimeError("CUDA tensor smoke test returned unexpected value")
        else:
            tensor = torch.tensor([1.0])
            result["device"] = "cpu_fallback"
            result["success"] = bool(float(tensor.sum().item()) == 1.0)
            if not result["success"]:
                raise RuntimeError("CPU tensor smoke test returned unexpected value")
    except Exception as exc:
        result["error"] = str(exc)

    print(json.dumps(result))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
