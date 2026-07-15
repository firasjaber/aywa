# Aywa third-party notices and distribution policy

## Scope

The MIT License in this repository applies only to Aywa-owned source code and documentation. It does not relicense Rust or JavaScript dependencies, CPython, `get-pip.py`, PyTorch/CUDA packages, NeMo, Python wheels, or Hugging Face model weights. Each third-party component remains subject to its own license or governing terms.

This document records the components Aywa bundles and the policy that applies before a public installer is distributed. It is an engineering inventory, not legal advice or a substitute for legal review.

## Bundled components

| Component | Version or integrity record | Source | Delivery | Terms / release status |
| --- | --- | --- | --- | --- |
| CPython embeddable distribution | 3.12.10; SHA-256 `4acbed6dd1c744b0376e3b1cf57ce906f9dc9e95e68824584c8099a63025a3c3` | `https://www.python.org/ftp/python/3.12.10/python-3.12.10-embed-amd64.zip` | Bundled in the Windows application | Python Software Foundation License. Retain all required notices and confirm redistribution terms before installer release. |
| get-pip.py | Bundled pip 26.0.1; SHA-256 `feba1c697df45be1b539b40d93c102c9ee9dde1d966303323b830b06f3fbca3c` | `https://bootstrap.pypa.io/get-pip.py` | Bundled in the Windows application | Upstream pip/get-pip terms. Retain all required notices and confirm redistribution terms before installer release. |

## First-run downloads

Aywa currently obtains Python wheels, PyTorch/CUDA packages, NeMo dependencies, and Hugging Face model weights during first-run runtime or model preparation. These files are not Aywa-licensed material. Aywa must not directly redistribute any of them until its complete dependency graph is pinned with hashes, model revisions are immutable rather than resolved from `main`, an SBOM is generated, and a maintainer has reviewed the applicable terms and required notices.

## Network endpoints

- `python.org` provides the CPython embeddable runtime.
- `bootstrap.pypa.io` provides `get-pip.py`.
- PyPI provides Python packages and transitive dependencies.
- `download.pytorch.org` provides the selected PyTorch runtime-profile packages.
- `huggingface.co` provides model metadata and model files.

Every released runtime or model configuration must record its immutable source or revision and its integrity-verification method.

## Release gate

Before publishing a public installer, a maintainer must verify all of the following:

1. License metadata, `LICENSE`, and `NOTICE` identify Aywa as MIT-licensed work.
2. This file records every bundled non-Aywa component with version or revision, source, integrity record, terms, and required attribution.
3. An SBOM covers the shipped application and every bundled runtime artifact.
4. Directly distributed Python dependencies and model files are immutable, hash-pinned, and reviewed for redistribution rights.
5. Required third-party notices are included with the release.
