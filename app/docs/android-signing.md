# Neural Nova Android development signing — one-time setup

Neural Nova intentionally uses a **single persistent debug/development signing key** so future APKs can update the existing installation. The signing key is private; it must NOT be committed into this public GitHub repository or posted in an issue, chat, or workflow log.

## Why Android builds currently fail

The GitHub Actions workflow requires two secrets and refuses to sign with a throwaway CI/debug key:
- `NOVA_ANDROID_KEYSTORE_BASE64` — single-line Base64 encoding of the ORIGINAL `.jks` keystore
- `NOVA_ANDROID_KEYSTORE_PASSWORD` — the password supplied with that keystore (used for both store and key)

The keystore's alias is `nova-experimental-dev`. Its SHA-256 signing certificate fingerprint is pinned:

`21:5D:D8:EF:18:BC:79:4B:4E:52:D6:A3:69:D2:2D:C1:06:A9:84:12:3F:FC:AD:A3:53:33:97:41:29:E2:4A:E3`

Do **not** generate a replacement key if you have the original: a different certificate cannot update an APK signed by the original.

## Setup in GitHub (repository owner action)

1. Find and safely store the `.jks` file and password previously generated for Neural Nova. Keep an offline backup of both.
2. On a trusted **Windows PowerShell** terminal, generate the Base64 value from the original file, without printing it in a build log:
   ```powershell
   [Convert]::ToBase64String([IO.File]::ReadAllBytes("C:\\path\\to\\nova-development.jks")) | Set-Clipboard
   ```
   On Linux/macOS, use `base64 -w 0 nova-development.jks` (Linux), or `base64 < nova-development.jks | tr -d '\\n'` (macOS). Keep clipboard or terminal contents private.
3. Open **SirSkirt/NovaAI → Settings → Secrets and variables → Actions → New repository secret**.
4. Create `NOVA_ANDROID_KEYSTORE_BASE64` and paste the entire Base64 value. Then create `NOVA_ANDROID_KEYSTORE_PASSWORD` and enter the supplied password.
5. Open **Actions → Neural Nova - Experimental App Builds → Run workflow** on branch `experimental/neural-nova`.
6. After a successful Android build, confirm the `Verify APK signed by persistent Neural Nova development certificate` step passes and download the APK artifact.

Alternatively with GitHub CLI on a trusted computer, authenticate with `gh auth login` and set the secrets locally (never commit a keystore):
```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes("C:\\path\\to\\nova-development.jks")) | gh secret set NOVA_ANDROID_KEYSTORE_BASE64 --repo SirSkirt/NovaAI
gh secret set NOVA_ANDROID_KEYSTORE_PASSWORD --repo SirSkirt/NovaAI
```
GitHub CLI will prompt you for the second value if standard input is interactive.

## Updating an installed APK

For subsequent versions, keep **both** the Android application ID `com.qtech.neuralnova.experimental` and signing certificate unchanged, and increment the Android version code. The version 0.10.0 build uses this identifier. Installing an APK signed with a different debug certificate over an older APK will fail; export Nova's checkpoints first if an uninstall is needed for the one-time switch.

## Troubleshooting

- **Signing secrets missing:** the early preflight step fails with instructions. Do not retry until both secrets exist.
- **Certificate mismatch:** the setup script intentionally rejects a different key to protect update compatibility.
- **Alias/password incorrect:** verify the original keystore using `keytool -list -v -keystore nova-development.jks -alias nova-experimental-dev` on your own device. Do not send the password or private keystore into chat.
- **Lost the original key:** a replacement can be created, but APKs signed with the previous key will not be updatable in place. Export checkpoints before uninstalling the older app.
