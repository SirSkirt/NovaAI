#!/usr/bin/env python3
"""Register the app-private Android SAF checkpoint document writer.
Training foreground services are deliberately not installed by Nova Messenger.
"""
from pathlib import Path
import shutil
import sys

app = Path(__file__).resolve().parents[1]
android = app / "src-tauri/gen/android"
manifest = android / "app/src/main/AndroidManifest.xml"
if not manifest.is_file():
    sys.exit("Generate the Tauri Android project first")
source = app / "android/NovaTrainingContextProvider.java"
target = (android / "app/src/main/java/com/qtech/neuralnova/experimental"
          / "NovaTrainingContextProvider.java")
target.parent.mkdir(parents=True, exist_ok=True)
shutil.copyfile(source, target)
contents = manifest.read_text()
provider = """<provider
            android:name="com.qtech.neuralnova.experimental.NovaTrainingContextProvider"
            android:authorities="com.qtech.neuralnova.experimental.checkpoint-context"
            android:exported="false"
            android:initOrder="100" />
    """
if "NovaTrainingContextProvider" not in contents:
    if "</application>" not in contents:
        sys.exit("Generated Android manifest has no application element")
    manifest.write_text(contents.replace("</application>", provider+"</application>", 1))
print("Registered private checkpoint Storage Access Framework provider.")
