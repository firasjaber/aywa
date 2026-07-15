# Aywa licensing and third-party distribution design

## Purpose

Define Aywa's first public-release licensing and third-party distribution baseline. This design resolves GitHub issue #2 without claiming legal review of upstream components.

## Decision

Aywa-owned source code and documentation will be licensed under the MIT License. The copyright notice will identify `Firas Jaber` as the copyright owner.

MIT is intentionally chosen for its short, permissive terms. It permits commercial and closed-source derivative works when the required copyright and permission notice are retained. It does not provide Apache-2.0's express patent grant; that trade-off is accepted in favor of simplicity.

## Repository artifacts

- Add a root `LICENSE` containing the canonical MIT License text.
- Set Cargo package metadata to `license = "MIT"` and the public repository URL.
- Set the frontend package metadata to `"license": "MIT"`.
- Replace the README's no-license notice with the MIT designation and a link to `LICENSE`.
- Add `THIRD_PARTY_NOTICES.md` as the public inventory and attribution record for all software or data redistributed with an Aywa release.

`THIRD_PARTY_NOTICES.md` must separate Aywa-owned work from third-party material and, for each distributed third-party component, record its name, version or immutable revision, source URL, license or governing terms, how it is delivered, and required attribution.

## Distribution policy

Aywa's MIT license applies only to Aywa-owned source and documentation. It does not relicense Python, Rust/JavaScript dependencies, PyTorch/CUDA packages, NeMo, `get-pip.py`, CPython, model weights, or other third-party content.

Release artifacts may bundle a third-party component only after its redistribution permission, provenance, version or revision, integrity data, and required notices have been recorded in `THIRD_PARTY_NOTICES.md` and reviewed. A release must not bundle a component whose terms cannot be verified.

The current application bundles the CPython embeddable runtime and `get-pip.py`; both therefore require entries and verified redistribution terms before public installer distribution. Rust and JavaScript dependency attributions will be captured from lockfiles during release preparation.

Python wheels, PyTorch/CUDA packages, NeMo dependencies, and Hugging Face model weights are currently acquired at first run from upstream services. They remain subject to their own licenses and terms. Before Aywa distributes any of them directly, the project must pin the complete dependency graph with hashes, record model revisions rather than the mutable `main` reference, generate an SBOM, and review their terms for the intended distribution.

## Network transparency

The distribution documentation will disclose the current first-run endpoints:

- `python.org` for the CPython embeddable runtime.
- `bootstrap.pypa.io` for `get-pip.py`.
- PyPI and `download.pytorch.org` for Python packages and selected PyTorch runtime profiles.
- `huggingface.co` for model metadata and model files.

Each released runtime/model configuration must identify its immutable source or revision and integrity verification method.

## Release gate and verification

Public installer release requires all of the following:

1. MIT metadata and the root license file agree.
2. `THIRD_PARTY_NOTICES.md` covers every bundled non-Aywa component.
3. An SBOM is generated for the shipped application and any bundled runtime content.
4. Python dependency hashes and model revisions are immutable and documented before their direct redistribution.
5. A maintainer has reviewed upstream terms and required notices for every artifact included in the release.

Automated checks will validate license metadata, required policy files, and the documented release inventory. They do not replace legal review.

## Non-goals

- Determining the legal compatibility of every existing or future third-party dependency.
- Publishing an installer, model, Python wheel, or SBOM in this change.
- Changing runtime installation behavior beyond documenting the requirements for a future public release.
