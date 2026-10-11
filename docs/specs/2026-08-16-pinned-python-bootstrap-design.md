# Pinned Python bootstrap design

## Goal

Make embedded-Python preparation reproducible so a PyPA update cannot break release CI between commits.

## Design

Replace the mutable `https://bootstrap.pypa.io/get-pip.py` manifest URL with a raw GitHub URL that includes a full, immutable `pypa/get-pip` commit SHA. Refresh `runtime-lock.json` from that exact source and retain existing checksum verification; a failed download or checksum mismatch stops preparation before build steps run.

The release-policy verifier rejects the mutable bootstrap URL and requires a raw GitHub URL with a 40-character commit SHA. Documentation defines updates as: choose and review a new upstream commit, change the manifest URL, delete the cached script, refresh the lock, review the manifest and checksum, and commit both files.

## Verification

Unit tests cover allowed and rejected bootstrap URLs. The runtime-preparation script is run from a clean resource directory and its resulting resources pass the release-policy verifier.
