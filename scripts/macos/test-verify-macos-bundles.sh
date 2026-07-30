#!/usr/bin/env bash
# Exercise macOS bundle verification with ad-hoc-signed fixtures.
# Run on macOS: ./scripts/macos/test-verify-macos-bundles.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERIFY_SCRIPT="${SCRIPT_DIR}/verify-macos-bundles.sh"
TEST_DIR="$(mktemp -d)"
FAILURES=0

cleanup() {
  rm -rf "${TEST_DIR}"
}
trap cleanup EXIT

pass() {
  printf 'PASS: %s\n' "$1"
}

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  FAILURES=$((FAILURES + 1))
}

make_app() {
  local app="$1"
  local pdfium_dir="${app}/Contents/Resources/resources/pdfium"

  mkdir -p "${app}/Contents/MacOS" "${pdfium_dir}"
  cp /bin/echo "${app}/Contents/MacOS/Sample"
  cp /bin/echo "${pdfium_dir}/libpdfium.dylib"
  touch "${pdfium_dir}/LicenseRef-PdfiumThirdParty.txt"
  printf '%s\n' \
    '<?xml version="1.0" encoding="UTF-8"?>' \
    '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">' \
    '<plist version="1.0"><dict>' \
    '<key>CFBundleExecutable</key><string>Sample</string>' \
    '<key>CFBundleIdentifier</key><string>com.pdftools.verify-test</string>' \
    '</dict></plist>' >"${app}/Contents/Info.plist"
}

sign_app() {
  local app="$1"
  local library="${app}/Contents/Resources/resources/pdfium/libpdfium.dylib"

  codesign --force --sign - --timestamp=none "${library}" >/dev/null 2>&1
  codesign --force --deep --sign - --timestamp=none "${app}" >/dev/null 2>&1
}

expect_success() {
  local label="$1"
  shift
  if "$@" >"${TEST_DIR}/last-command.log" 2>&1; then
    pass "${label}"
  else
    fail "${label}"
    sed -n '1,120p' "${TEST_DIR}/last-command.log" >&2
  fi
}

# A non-zero exit alone is a weak assertion: a missing or broken verify script
# fails every negative case for the wrong reason. Each negative case therefore
# also pins the message that proves the intended check is what rejected it.
expect_failure() {
  local label="$1"
  local expected="$2"
  shift 2
  if "$@" >"${TEST_DIR}/last-command.log" 2>&1; then
    fail "${label} (expected a non-zero exit)"
    return
  fi
  if ! grep -qF -- "${expected}" "${TEST_DIR}/last-command.log"; then
    fail "${label} (expected output containing: ${expected})"
    sed -n '1,120p' "${TEST_DIR}/last-command.log" >&2
    return
  fi
  pass "${label}"
}

# POSITIVE: a signed app with signed PDFium and its license verifies.
SIGNED_DIR="${TEST_DIR}/signed"
SIGNED_APP="${SIGNED_DIR}/Sample.app"
mkdir -p "${SIGNED_DIR}"
make_app "${SIGNED_APP}"
sign_app "${SIGNED_APP}"
expect_success "signed app verifies" "${VERIFY_SCRIPT}" "${SIGNED_APP}"

# POSITIVE: the same signed app also verifies from inside a disk image.
if hdiutil create -volname Sample -srcfolder "${SIGNED_DIR}" -ov -format UDZO \
  "${TEST_DIR}/signed.dmg" >/dev/null; then
  expect_success "signed app inside a disk image verifies" \
    "${VERIFY_SCRIPT}" "${SIGNED_APP}" "${TEST_DIR}/signed.dmg"
else
  fail "signed app disk image fixture is created"
fi

# NEGATIVE: an unsigned app embedded in a disk image is rejected.
UNSIGNED_DIR="${TEST_DIR}/unsigned"
UNSIGNED_APP="${UNSIGNED_DIR}/Sample.app"
mkdir -p "${UNSIGNED_DIR}"
make_app "${UNSIGNED_APP}"
codesign --force --sign - --timestamp=none \
  "${UNSIGNED_APP}/Contents/Resources/resources/pdfium/libpdfium.dylib" >/dev/null 2>&1
if hdiutil create -volname SampleUnsigned -srcfolder "${UNSIGNED_DIR}" -ov -format UDZO \
  "${TEST_DIR}/unsigned.dmg" >/dev/null; then
  # The rejection must happen after the image is mounted, on the embedded copy.
  expect_failure "unsigned app inside a disk image is rejected" \
    "Verifying embedded .app" \
    "${VERIFY_SCRIPT}" "${SIGNED_APP}" "${TEST_DIR}/unsigned.dmg"
else
  fail "unsigned app disk image fixture is created"
fi

# NEGATIVE: a signed app cannot hide an unsigned bundled PDFium library.
UNSIGNED_LIBRARY_APP="${TEST_DIR}/unsigned-library/Sample.app"
make_app "${UNSIGNED_LIBRARY_APP}"
UNSIGNED_LIBRARY="${UNSIGNED_LIBRARY_APP}/Contents/Resources/resources/pdfium/libpdfium.dylib"
codesign --force --sign - --timestamp=none "${UNSIGNED_LIBRARY}" >/dev/null 2>&1
codesign --remove-signature "${UNSIGNED_LIBRARY}" >/dev/null 2>&1
codesign --force --deep --sign - --timestamp=none "${UNSIGNED_LIBRARY_APP}" >/dev/null 2>&1
expect_failure "unsigned bundled PDFium library is rejected" \
  "code object is not signed at all" \
  "${VERIFY_SCRIPT}" "${UNSIGNED_LIBRARY_APP}"

# NEGATIVE: a signed app without the PDFium license is rejected.
MISSING_LICENSE_APP="${TEST_DIR}/missing-license/Sample.app"
make_app "${MISSING_LICENSE_APP}"
rm "${MISSING_LICENSE_APP}/Contents/Resources/resources/pdfium/LicenseRef-PdfiumThirdParty.txt"
sign_app "${MISSING_LICENSE_APP}"
expect_failure "missing PDFium license is rejected" \
  "Bundled LicenseRef-PdfiumThirdParty.txt was not found" \
  "${VERIFY_SCRIPT}" "${MISSING_LICENSE_APP}"

# NEGATIVE: a signed app without the PDFium library is rejected.
MISSING_LIBRARY_APP="${TEST_DIR}/missing-library/Sample.app"
make_app "${MISSING_LIBRARY_APP}"
rm "${MISSING_LIBRARY_APP}/Contents/Resources/resources/pdfium/libpdfium.dylib"
codesign --force --deep --sign - --timestamp=none "${MISSING_LIBRARY_APP}" >/dev/null 2>&1
expect_failure "missing PDFium library is rejected" \
  "Bundled libpdfium.dylib was not found" \
  "${VERIFY_SCRIPT}" "${MISSING_LIBRARY_APP}"

if [[ "${FAILURES}" -gt 0 ]]; then
  printf 'FAIL: %d test case(s) failed\n' "${FAILURES}" >&2
  exit 1
fi

printf 'PASS: all macOS bundle verification tests passed\n'
