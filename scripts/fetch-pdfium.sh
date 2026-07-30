#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
VERSION_FILE="${SCRIPT_DIR}/pdfium-version.txt"
CHECKSUMS_FILE="${SCRIPT_DIR}/pdfium-checksums.txt"
OUTPUT_DIR="${REPO_ROOT}/src-tauri/resources/pdfium"
STAMP_FILE="${OUTPUT_DIR}/.pdfium-stamp"

TAG="$(awk '!/^[[:space:]]*(#|$)/ { print $1; exit }' "${VERSION_FILE}")"
if [[ -z "${TAG}" ]]; then
  printf 'PDFium version file does not contain a release tag: %s\n' "${VERSION_FILE}" >&2
  exit 1
fi

UNAME_S="$(uname -s)"
case "${UNAME_S}" in
  Darwin)
    ARCHIVE_NAME="pdfium-mac-univ.tgz"
    LIBRARY_SOURCE="lib/libpdfium.dylib"
    LIBRARY_NAME="libpdfium.dylib"
    ;;
  MINGW* | MSYS* | CYGWIN*)
    ARCHIVE_NAME="pdfium-win-x64.tgz"
    LIBRARY_SOURCE="bin/pdfium.dll"
    LIBRARY_NAME="pdfium.dll"
    ;;
  *)
    case "${OSTYPE:-}" in
      darwin*)
        ARCHIVE_NAME="pdfium-mac-univ.tgz"
        LIBRARY_SOURCE="lib/libpdfium.dylib"
        LIBRARY_NAME="libpdfium.dylib"
        ;;
      msys* | mingw* | cygwin*)
        ARCHIVE_NAME="pdfium-win-x64.tgz"
        LIBRARY_SOURCE="bin/pdfium.dll"
        LIBRARY_NAME="pdfium.dll"
        ;;
      *)
        printf 'PDFium fetch supports only macOS and Windows; Linux is not supported (detected: %s).\n' "${UNAME_S}" >&2
        exit 1
        ;;
    esac
    ;;
esac

EXPECTED_SHA256="$(awk -v archive="${ARCHIVE_NAME}" '
  !/^[[:space:]]*#/ && $2 == archive { print tolower($1); exit }
' "${CHECKSUMS_FILE}")"
if [[ ! "${EXPECTED_SHA256}" =~ ^[0-9a-f]{64}$ ]]; then
  printf 'No valid SHA-256 checksum found for %s in %s\n' "${ARCHIVE_NAME}" "${CHECKSUMS_FILE}" >&2
  exit 1
fi

CURRENT_STAMP="${TAG}|${ARCHIVE_NAME}|${EXPECTED_SHA256}"
if [[ -f "${STAMP_FILE}" &&
      "$(<"${STAMP_FILE}")" == "${CURRENT_STAMP}" &&
      -f "${OUTPUT_DIR}/${LIBRARY_NAME}" &&
      -f "${OUTPUT_DIR}/LICENSE" &&
      -f "${OUTPUT_DIR}/LicenseRef-PdfiumThirdParty.txt" ]]; then
  printf 'PDFium %s is already up to date.\n' "${TAG}"
  exit 0
fi

mkdir -p "${OUTPUT_DIR}"

TEMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "${TEMP_DIR}"
}
trap cleanup EXIT

ENCODED_TAG="${TAG//\//%2F}"
DOWNLOAD_URL="https://github.com/bblanchon/pdfium-binaries/releases/download/${ENCODED_TAG}/${ARCHIVE_NAME}"
ARCHIVE_PATH="${TEMP_DIR}/${ARCHIVE_NAME}"
EXTRACT_DIR="${TEMP_DIR}/extracted"

printf 'Downloading PDFium %s for %s...\n' "${TAG}" "${ARCHIVE_NAME}"
curl -fL --retry 3 "${DOWNLOAD_URL}" -o "${ARCHIVE_PATH}"

if command -v shasum >/dev/null 2>&1; then
  ACTUAL_SHA256="$(shasum -a 256 "${ARCHIVE_PATH}" | awk '{ print tolower($1) }')"
elif command -v sha256sum >/dev/null 2>&1; then
  ACTUAL_SHA256="$(sha256sum "${ARCHIVE_PATH}" | awk '{ print tolower($1) }')"
else
  printf 'Neither shasum nor sha256sum is available for checksum verification.\n' >&2
  exit 1
fi

if [[ "${ACTUAL_SHA256}" != "${EXPECTED_SHA256}" ]]; then
  printf 'PDFium checksum mismatch for %s\n' "${ARCHIVE_NAME}" >&2
  printf 'Expected: %s\n' "${EXPECTED_SHA256}" >&2
  printf 'Actual:   %s\n' "${ACTUAL_SHA256}" >&2
  exit 1
fi

mkdir -p "${EXTRACT_DIR}"
tar -xzf "${ARCHIVE_PATH}" -C "${EXTRACT_DIR}"

if [[ ! -f "${EXTRACT_DIR}/${LIBRARY_SOURCE}" || ! -f "${EXTRACT_DIR}/LICENSE" ]]; then
  printf 'PDFium archive is missing required files.\n' >&2
  exit 1
fi

THIRD_PARTY_LICENSE="${EXTRACT_DIR}/LicenseRef-PdfiumThirdParty.txt"
if [[ ! -f "${THIRD_PARTY_LICENSE}" ]]; then
  if [[ ! -d "${EXTRACT_DIR}/licenses" ]]; then
    printf 'PDFium archive is missing third-party license files.\n' >&2
    exit 1
  fi

  THIRD_PARTY_LICENSE="${TEMP_DIR}/LicenseRef-PdfiumThirdParty.txt"
  : >"${THIRD_PARTY_LICENSE}"
  LICENSE_COUNT=0
  # Newer archives ship a licenses directory instead of one combined license file.
  while IFS= read -r license_file; do
    relative_path="${license_file#"${EXTRACT_DIR}/"}"
    {
      printf '===== %s =====\n' "${relative_path}"
      cat "${license_file}"
      printf '\n\n'
    } >>"${THIRD_PARTY_LICENSE}"
    LICENSE_COUNT=$((LICENSE_COUNT + 1))
  done < <(find "${EXTRACT_DIR}/licenses" -type f -print | LC_ALL=C sort)

  if [[ "${LICENSE_COUNT}" -eq 0 ]]; then
    printf 'PDFium archive contains no third-party license files.\n' >&2
    exit 1
  fi
fi

# Only clear the output directory once the download passed verification, so a
# failed fetch never destroys a previously installed, working library.
rm -f \
  "${OUTPUT_DIR}/libpdfium.dylib" \
  "${OUTPUT_DIR}/pdfium.dll" \
  "${OUTPUT_DIR}/LICENSE" \
  "${OUTPUT_DIR}/LicenseRef-PdfiumThirdParty.txt" \
  "${STAMP_FILE}"

cp "${EXTRACT_DIR}/${LIBRARY_SOURCE}" "${OUTPUT_DIR}/${LIBRARY_NAME}"
cp "${EXTRACT_DIR}/LICENSE" "${OUTPUT_DIR}/LICENSE"
cp "${THIRD_PARTY_LICENSE}" "${OUTPUT_DIR}/LicenseRef-PdfiumThirdParty.txt"
printf '%s\n' "${CURRENT_STAMP}" >"${STAMP_FILE}"

printf 'PDFium %s installed in %s\n' "${TAG}" "${OUTPUT_DIR}"
