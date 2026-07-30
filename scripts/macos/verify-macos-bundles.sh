#!/usr/bin/env bash
# Verify the standalone macOS app and, when provided, the copy shipped inside
# its disk image. Uses codesign only; self-signed certificates cannot pass a
# Gatekeeper assessment.
#
# Args: <app-path> [dmg-path]
set -euo pipefail

app="${1:?usage: verify-macos-bundles.sh <app-path> [dmg-path]}"
dmg="${2:-}"

verify_app() {
  local label="$1"
  local app_path="$2"
  local library
  local license

  echo "Verifying ${label}: ${app_path}"
  codesign --verify --deep --strict --verbose=2 "${app_path}"

  # Tauri maps `resources/pdfium/*` to Contents/Resources/resources/pdfium,
  # so the payload is located by name instead of by a hard-coded path.
  library="$(find "${app_path}/Contents/Resources" -type f \
    -name 'libpdfium.dylib' -print -quit 2>/dev/null || true)"
  if [[ -z "${library}" ]]; then
    echo "::error::Bundled libpdfium.dylib was not found in ${app_path}"
    exit 1
  fi
  # `codesign --deep` seals a loose dylib under Contents/Resources as a resource
  # without signing it, so the bundled library is verified on its own.
  echo "Verifying bundled PDFium library: ${library}"
  codesign --verify --strict --verbose=2 "${library}"

  license="$(find "${app_path}/Contents/Resources" -type f \
    -name 'LicenseRef-PdfiumThirdParty.txt' -print -quit 2>/dev/null || true)"
  if [[ -z "${license}" ]]; then
    echo "::error::Bundled LicenseRef-PdfiumThirdParty.txt was not found in ${app_path}"
    exit 1
  fi
  echo "Found bundled PDFium license: ${license}"
}

verify_app "standalone .app" "${app}"

if [[ -n "${dmg}" ]]; then
  echo "Verifying .dmg wrapper: ${dmg}"
  codesign --verify --verbose=2 "${dmg}" ||
    echo "note: .dmg wrapper signature not present (continuing)"

  echo "Mounting .dmg to verify the embedded .app"
  mount_point="$(mktemp -d)"
  detach() {
    hdiutil detach "${mount_point}" >/dev/null 2>&1 || true
    rmdir "${mount_point}" 2>/dev/null || true
  }
  trap detach EXIT
  hdiutil attach -nobrowse -readonly -mountpoint "${mount_point}" "${dmg}" >/dev/null

  inner_app="$(find "${mount_point}" -maxdepth 1 -type d \
    -name '*.app' -print -quit)"
  if [[ -z "${inner_app}" ]]; then
    echo "::error::No .app found inside the mounted .dmg"
    exit 1
  fi
  verify_app "embedded .app" "${inner_app}"
fi

echo "macOS signature verification complete."
