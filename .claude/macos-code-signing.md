# macOS code signing (L3)

> Doc layers: L1 = [`/CLAUDE.md`](../CLAUDE.md) / L2 = [`docs/development.md`](../docs/development.md) / L3 = this file.

How the macOS release bundle gets signed, how to produce the certificate it is signed with, and
how to check the certificate before a release build spends ten minutes discovering it is wrong.

## What the release job needs

`.github/workflows/release.yml` fails its `Preflight macOS signing secrets` step unless all three
repository secrets are set. This is deliberate: an unsigned macOS bundle must never reach a
release by accident.

| Secret                       | Value                                                     |
| ---------------------------- | --------------------------------------------------------- |
| `APPLE_CERTIFICATE`          | the `.p12`, base64-encoded, no line breaks                |
| `APPLE_CERTIFICATE_PASSWORD` | the export password of that `.p12`                        |
| `KEYCHAIN_PASSWORD`          | any random string; only unlocks the temporary CI keychain |

The signing identity itself is not a secret — it is hard-coded in the workflow as
`APPLE_SIGNING_IDENTITY: "Developer ID Application: PDF Tools"` and must equal the certificate's
Common Name exactly.

## What the certificate is, and is not

The certificate is **self-signed**. It is not issued by Apple, the app is **not notarized**, and
Gatekeeper still blocks the first launch until the user clears it by hand (see the installation
section of the README). Signing buys a stable, verifiable signature on the shipped `.app` and
`.dmg` — which `scripts/macos/verify-macos-bundles.sh` asserts in CI — not a Gatekeeper pass.

Two naming rules are forced on us by Tauri, not by Apple:

- **The Common Name must start with an Apple prefix** (`Developer ID Application: `). Tauri's
  identity discovery only looks at certificates whose CN starts with one of a fixed set of
  prefixes.
- **The certificate must carry an Organizational Unit.** Tauri parses the certificate and reads
  the OU as the Team ID:

  ```rust
  // crates/tauri-macos-sign/src/keychain/identity.rs
  let id = cert
    .subject_name()
    .iter_organizational_unit()
    .next()
    .and_then(|v| v.to_string().ok())
    .ok_or_else(|| Error::CertificateMissingOrganizationUnit { ... })?;
  ```

Neither rule asserts any relationship with Apple. They are the shape Tauri's self-signed code
path expects.

## Generating the certificate

```sh
P12_PASSWORD='choose-a-password' ./scripts/macos/generate-self-signed-cert.sh
```

Writes `cert.pem`, `key.pem`, `cert.p12` and `cert.p12.base64` into `./cert-out/`, which is
git-ignored along with `*.p12` and `*.p12.base64`. Defaults: CN
`Developer ID Application: PDF Tools`, OU `PDFTOOLS01`, O `PDF Tools`, 10-year validity; override
with `CERT_CN` / `CERT_OU` / `CERT_O` / `OUT_DIR`.

The script prefers Homebrew's OpenSSL 3 and adds `-legacy` to the `pkcs12` export, because a
`.p12` written with OpenSSL 3 defaults is not importable by macOS `security`. It falls back to the
system LibreSSL, which must not be given `-legacy`.

**Keep `cert.p12` and its password.** Regenerating produces a different key, so every future
release would be signed by a different certificate.

## Registering the secrets

```sh
gh secret set APPLE_CERTIFICATE          < cert-out/cert.p12.base64
gh secret set APPLE_CERTIFICATE_PASSWORD   # paste the P12_PASSWORD
gh secret set KEYCHAIN_PASSWORD            # paste any random string
```

## Checking the certificate before a release run

A release build reaches the signing step several minutes in, so verify locally first. This
imports the `.p12` into a throwaway keychain, signs a real binary, and reads the OU back the same
way Tauri does.

```sh
cd cert-out
KC="$PWD/verify.keychain-db"
security create-keychain -p testpw "$KC"
security unlock-keychain -p testpw "$KC"
security import cert.p12 -k "$KC" -P "$P12_PASSWORD" -T /usr/bin/codesign -f pkcs12
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k testpw "$KC"

cp /bin/echo ./testbin
codesign --force --sign "Developer ID Application: PDF Tools" --keychain "$KC" --timestamp=none ./testbin
codesign --verify --strict --verbose=2 ./testbin

# what Tauri reads to resolve the Team ID
security find-certificate -c "Developer ID Application: PDF Tools" -p "$KC" |
  openssl x509 -noout -subject

security delete-keychain "$KC"
rm -f testbin
```

Expect `valid on disk`, `satisfies its Designated Requirement`, and a subject containing
`OU=PDFTOOLS01`.

Two results look alarming and are not:

- `security find-identity -v -p codesigning` reports **0 valid identities**. A self-signed
  certificate has no trusted chain, so it is never "valid" in that sense; `codesign` signs with it
  regardless.
- `codesign --display` reports `TeamIdentifier=not set`. That field is only populated for a
  certificate that chains to Apple. It is unrelated to the OU that Tauri reads, so it is not a
  useful pre-flight check.

## Failure modes seen in CI

| Symptom                                                                                              | Cause                                                                                                              |
| ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `Missing required macOS signing secrets: …` in `Preflight macOS signing secrets`                     | one or more of the three secrets is unset                                                                          |
| `certificate missing organization unit for common name …`, then `failed to resolve signing identity` | the certificate has no OU                                                                                          |
| Tauri never finds the certificate at all                                                             | the CN does not start with `Developer ID Application: `, or `APPLE_SIGNING_IDENTITY` does not match the CN exactly |
