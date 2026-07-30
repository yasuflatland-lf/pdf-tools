#!/usr/bin/env bash

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
FETCH_SCRIPT="${SCRIPT_DIR}/fetch-pdfium.sh"
CHECKSUMS_FILE="${SCRIPT_DIR}/pdfium-checksums.txt"
OUTPUT_DIR="${REPO_ROOT}/src-tauri/resources/pdfium"
BACKUP_FILE="$(mktemp)"
TEST_DIR="$(mktemp -d)"
FAILURES=0
CHECKSUMS_EXISTED=0

if [[ -f "${CHECKSUMS_FILE}" ]]; then
  cp "${CHECKSUMS_FILE}" "${BACKUP_FILE}"
  CHECKSUMS_EXISTED=1
fi

restore_checksums() {
  if [[ "${CHECKSUMS_EXISTED}" -eq 1 ]]; then
    cp "${BACKUP_FILE}" "${CHECKSUMS_FILE}"
  else
    rm -f "${CHECKSUMS_FILE}"
  fi
  rm -f "${BACKUP_FILE}"
  rm -rf "${TEST_DIR}"
}
trap restore_checksums EXIT

pass() {
  printf 'PASS: %s\n' "$1"
}

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  FAILURES=$((FAILURES + 1))
}

case "$(uname -s)" in
  Darwin)
    ARCHIVE_NAME="pdfium-mac-univ.tgz"
    LIBRARY_NAME="libpdfium.dylib"
    ;;
  MINGW* | MSYS* | CYGWIN*)
    ARCHIVE_NAME="pdfium-win-x64.tgz"
    LIBRARY_NAME="pdfium.dll"
    ;;
  *)
    printf 'FAIL: unsupported test platform: %s\n' "$(uname -s)" >&2
    exit 1
    ;;
esac

if [[ ! -x "${FETCH_SCRIPT}" || ! -f "${CHECKSUMS_FILE}" ]]; then
  fail "fetch script and checksum fixture exist"
  exit 1
fi

mkdir -p "${OUTPUT_DIR}"
rm -f \
  "${OUTPUT_DIR}/libpdfium.dylib" \
  "${OUTPUT_DIR}/pdfium.dll" \
  "${OUTPUT_DIR}/LICENSE" \
  "${OUTPUT_DIR}/LicenseRef-PdfiumThirdParty.txt" \
  "${OUTPUT_DIR}/.pdfium-stamp"

awk -v archive="${ARCHIVE_NAME}" '
  $2 == archive {
    replacement = substr($1, 1, 1) == "0" ? "1" : "0"
    $1 = replacement substr($1, 2)
  }
  { print }
' "${BACKUP_FILE}" >"${CHECKSUMS_FILE}"

if "${FETCH_SCRIPT}" >"${TEST_DIR}/tampered.log" 2>&1; then
  fail "tampered checksum exits non-zero"
else
  if [[ ! -e "${OUTPUT_DIR}/${LIBRARY_NAME}" ]]; then
    pass "tampered checksum is rejected without placing a library"
  else
    fail "tampered checksum leaves no shared library"
  fi
fi

cp "${BACKUP_FILE}" "${CHECKSUMS_FILE}"

if "${FETCH_SCRIPT}" >"${TEST_DIR}/clean.log" 2>&1; then
  if [[ -f "${OUTPUT_DIR}/${LIBRARY_NAME}" &&
        -f "${OUTPUT_DIR}/LICENSE" &&
        -f "${OUTPUT_DIR}/LicenseRef-PdfiumThirdParty.txt" ]]; then
    pass "clean run places the library and both license files"
  else
    fail "clean run places every required output"
  fi
else
  fail "clean run exits zero"
  sed -n '1,120p' "${TEST_DIR}/clean.log" >&2
fi

MTIME_BEFORE="$(stat -f '%m' "${OUTPUT_DIR}/${LIBRARY_NAME}" 2>/dev/null ||
  stat -c '%Y' "${OUTPUT_DIR}/${LIBRARY_NAME}")"

if "${FETCH_SCRIPT}" >"${TEST_DIR}/idempotent.log" 2>&1; then
  MTIME_AFTER="$(stat -f '%m' "${OUTPUT_DIR}/${LIBRARY_NAME}" 2>/dev/null ||
    stat -c '%Y' "${OUTPUT_DIR}/${LIBRARY_NAME}")"
  if grep -qi "up to date" "${TEST_DIR}/idempotent.log" &&
    [[ "${MTIME_BEFORE}" == "${MTIME_AFTER}" ]]; then
    pass "second run is idempotent and does not re-download"
  else
    fail "second run reports up to date and leaves the library untouched"
  fi
else
  fail "second run exits zero"
  sed -n '1,120p' "${TEST_DIR}/idempotent.log" >&2
fi

if [[ "${FAILURES}" -gt 0 ]]; then
  printf 'FAIL: %d test case(s) failed\n' "${FAILURES}" >&2
  exit 1
fi

printf 'PASS: all fetch-pdfium tests passed\n'
