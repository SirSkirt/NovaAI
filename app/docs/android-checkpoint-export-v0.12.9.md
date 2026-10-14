# Neural Nova v0.12.9 — verified Android checkpoint export and restore

## Bug and cause

The export API was serializing to `neural-nova-hybrid-0.checkpoint-export.tmp`. `HybridBrain::save()` atomically stages its output by changing the destination extension to `.tmp`. Since the destination was **already** `.tmp`, the source and destination became the **same path**. The saver renamed that file into `.bak` and then failed to rename a now-missing staging file, reporting `No such file or directory (os error 2)`. The issue can affect desktop export too.

The serializer now stages a distinct `.nhyb` destination and cleans it up after reading it into a bounded export buffer. The common serializer also rejects `.tmp` destinations before any file I/O. Rust tests verify this fail-fast rule and a real `.nhyb` save/load roundtrip.

## Android export

The existing `Export checkpoint` button opens Android Files' **Create Document** picker via `window.__TAURI__.dialog.save()`. The user selects a location and filename. The `content://` URI is sent to Rust, which temporarily stages the immutable snapshot inside the private application sandbox. An existing app-owned, non-exported Java ContentProvider now offers `writeVerifiedCheckpoint`: it checks that the source is an app-private `.nhyb`, copies bytes through Android's `ContentResolver.openOutputStream(uri,"wt")`, flushes and **closes** the stream, then reopens the document with `openInputStream` and checks byte length and SHA-256. Only then does Nova report export successful.

This avoids a known Tauri fs plugin Android bug: direct writes to `content://` URIs can leave document-provider metadata reporting zero bytes if the Java ParcelFileDescriptor is detached too soon. The native stream remains open until writing finishes. No broad storage permission, hidden external service, model execution, file sharing or checkpoint-format modification is involved.

The operation is cancellable in the save picker; cancelling releases the in-memory serialized export. Incomplete or unverifiable saves return a descriptive error instead of claiming success.

## Android restore

The adjacent Restore control now offers a native **Android Files** picker, reads the SAF content URI through the Tauri filesystem plugin, and sends the bytes through the existing bounded neural checkpoint import/validation commands. The original pre-import checkpoint is backed up before replacing live weights. No existing data or current neural weights are changed by merely opening a picker.

## Windows and compatibility

The Windows export retains the existing browser/Blob download method; its temp-file collision is fixed in the same Rust serializer. The `.nhyb` structure, two cooperating brain pathways, optimizer, tokenizer, learned memory, original visualizer, application identifier and permanently pinned experimental debug signing key remain unchanged.

The new Android JNI bridge uses the application ClassLoader and provider already registered by `app/scripts/build_android_keepalive.py`, so it needs no additional Android permissions or Gradle dependencies. GitHub Actions checks provider injection, tests the mocked Android UI, runs Rust tests, builds Windows NSIS and Android APK and verifies all four ABI adapters and the pinned signer. A successful build does not replace a real-world phone export test: after installation, export to Downloads and confirm the `.nhyb` file is nonempty and can be imported back (after keeping a separate backup).
