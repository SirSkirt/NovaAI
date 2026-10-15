# Bundled Nova-2 model (19,000 optimizer steps)

The only default Nova checkpoint until the user selects a fully trained model is:

`app/src-tauri/resources/nova2-step19000.nnova2`

Source: user-provided `Nova2-lessons-step19000.zip`, member
`Nova2-lessons-step19000/nova2-lessons-step19000.nnova2`.

Expected model properties:

- File size: **14,188,203 bytes**
- SHA-256: `cdba02dc7058d6cf79f6d4e9ac633fbdbd87ef718316c2db7f242466824519ea`
- Checkpoint magic: `NNOVA2\0\0`
- Optimizer steps: **19,000**
- Parameters: **1,182,144**
- Architecture: four Transformer blocks, two shared-weight rounds, Dense path and trainable interconnect

The native Rust build script embeds this exact binary if present, without
changing model mathematics, learned weights, tokenizer, optimizer or Android
identity. At first launch, `nova2_boot` restores a previously imported/saved
.nnova2 checkpoint first. It only installs the embedded model on a fresh
installation with no saved checkpoint. Existing users' weights are **never
replaced** by a build upgrade.

Developer builds without this file still compile and can use the checkpoint
importer, but they show "Import model" and do not generate random untrained
responses as if they were trained. The GitHub Actions prerelease job deliberately
**withholds publication** when this model is absent or the checksum differs.

To enable releases, upload the extracted model directly to this path on
`experimental/neural-nova`. Do not commit the outer ZIP or rename a different
model to satisfy the file name. GitHub's web UI supports uploading a file of
this size to a branch; use `Add file -> Upload files` from this directory.

Once present, every successful experimental app workflow publishes an
automatically tagged GitHub **prerelease** containing the signed
`Nova-[Version].apk` and Windows installer. The unique preview build tag
prevents multiple commits with the same app version from overwriting each other.
