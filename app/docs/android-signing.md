# Neural Nova experimental Android signing (automatic)

All experimental APK builds now use **the same pinned development/debug key**, generated once for Neural Nova and stored at `app/keys/experimental-debug.jks.base64`. No GitHub secrets, PC, keystore configuration, or manual signing steps are required.

## What this is (and isn't)

- This is a **public debug signing key**, not a private production or release key. It is included in this open-source repository so CI builds are reproducible and all experimental APKs share one Android signature.
- Anyone with the repository can use this certificate to sign an app with Neural Nova's experimental package ID. **Only sideload builds you trust. Never use this key to sign a production or Play Store release.**
- Identifier: `com.qtech.neuralnova.experimental`
- Certificate SHA-256 fingerprint: `07:0B:1E:68:4D:A8:08:7D:6C:30:87:4C:16:C9:BE:83:1B:7C:BC:E4:92:64:D6:10:40:EE:73:EC:B5:65:2A:DE`
- Alias: `nova-experimental-dev`; password: `android` (deliberately public standard debug password).

## Installation and updating

The *first* time you switch to this pinned key, Android may reject an in-place update from an older Nova APK signed by a different automatically generated debug key. Android never permits regular update-over-install between unrelated signing certificates.

**Back up Nova's .nhyb joint neural checkpoint and training data first**, then uninstall the old APK only if Android reports a signature mismatch. Install the new experimental APK. After that one-time switch, subsequent experimental builds using this key and an increasing version code can install as normal updates without clearing app data.

The debug key must NEVER be regenerated/rotated casually. To replace it intentionally, plan a new application identifier or a documented migration.

## How CI works

The Android job initializes the Tauri project, compiles native code, then `app/scripts/configure_android_ci.py` decodes the pinned PKCS#12/JKS keystore into the generated Android folder. The script checks the signing-certificate fingerprint before configuring Gradle's **debug** signingConfig and the training foreground service manifest. CI verifies the built APK signature with Android's `apksigner`.

The repository key is strictly for local/experimental testing; publication builds should have a separate privately managed signing key and package identifier.
