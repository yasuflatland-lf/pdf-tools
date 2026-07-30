#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/.." && pwd)"
cd "${repo_root}"

cargo run -p pdf-tools-core --example make_fixtures

# encrypted.pdf is committed and is regenerated only manually with this command:
# qpdf --encrypt test test 128 -- crates/core/tests/fixtures/multi_page.pdf crates/core/tests/fixtures/encrypted.pdf
echo "Note: encrypted.pdf is committed and regenerated only manually with:"
echo "qpdf --encrypt test test 128 -- crates/core/tests/fixtures/multi_page.pdf crates/core/tests/fixtures/encrypted.pdf"

if command -v qpdf >/dev/null 2>&1 && [[ ! -f crates/core/tests/fixtures/encrypted.pdf ]]; then
    if ! qpdf --encrypt test test 128 -- crates/core/tests/fixtures/multi_page.pdf \
        crates/core/tests/fixtures/encrypted.pdf; then
        # Current qpdf releases require explicit permission to create 128-bit RC4 fixtures.
        qpdf --allow-weak-crypto --encrypt test test 128 -- \
            crates/core/tests/fixtures/multi_page.pdf crates/core/tests/fixtures/encrypted.pdf
    fi
fi
