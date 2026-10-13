#!/usr/bin/env python3
"""Build a JNI lifecycle-only adapter for the upstream foreground-service plugin.

The plugin 1.0.1 assumes a separate host native libapp_core.so at service start.
Neural Nova's actual trainer is the existing Tauri Rust worker, not a headless
libapp_core. This adapter only acknowledges foreground lifecycle and holds an
Android PARTIAL_WAKE_LOCK; it never trains or generates text.
"""
import os
from pathlib import Path
import platform
import subprocess
import sys

app = Path(__file__).resolve().parents[1]
project = app / "src-tauri/gen/android"
source = app / "android/nova_service_bridge.c"
if not (project / "app/build.gradle.kts").is_file():
    sys.exit("Android project not generated; run android:init first")
sdk = Path(os.environ.get("ANDROID_SDK_ROOT") or os.environ.get("ANDROID_HOME", ""))
if not sdk.is_dir():
    sys.exit("Android SDK not found (ANDROID_HOME / ANDROID_SDK_ROOT)")
ndk_requested = os.environ.get("ANDROID_NDK_VERSION")
ndks = sdk / "ndk"
ndk = ndks / ndk_requested if ndk_requested else None
if not ndk or not ndk.is_dir():
    found = sorted((p for p in ndks.glob("*") if p.is_dir()), reverse=True)
    if not found:
        sys.exit("Android NDK not installed")
    ndk = found[0]
host = {"Linux": "linux-x86_64", "Windows": "windows-x86_64",
        "Darwin": "darwin-x86_64"}.get(platform.system())
if not host:
    sys.exit("Unsupported Android native compiler host: " + platform.system())
bin_dir = ndk / "toolchains/llvm/prebuilt" / host / "bin"
extension = ".cmd" if platform.system() == "Windows" else ""
# Android API 24 matches the app minimum and supports ARMv7/AArch64/x86 targets.
targets = [
    ("arm64-v8a", "aarch64-linux-android24"),
    ("armeabi-v7a", "armv7a-linux-androideabi24"),
    ("x86", "i686-linux-android24"),
    ("x86_64", "x86_64-linux-android24"),
]
for abi, triple in targets:
    compiler = bin_dir / (triple + "-clang" + extension)
    if not compiler.is_file():
        sys.exit("Missing Android NDK clang: " + str(compiler))
    destination = project / "app/src/main/jniLibs" / abi / "libapp_core.so"
    destination.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run([str(compiler), "-std=c11", "-O2", "-fPIC", "-shared",
                    "-Wall", "-Wextra", "-Werror", "-o", str(destination),
                    str(source), "-lpthread"], check=True)
    if destination.stat().st_size < 1024:
        sys.exit("JNI lifecycle adapter unexpectedly small: " + str(destination))
    print(f"Built foreground lifecycle JNI adapter {abi}: {destination.stat().st_size} bytes")
print("Nova's JNI adapter does not contain a model or answer logic.")
