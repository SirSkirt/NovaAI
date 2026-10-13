#!/usr/bin/env python3
"""Configure persistent EXPLICITLY PUBLIC Android debug signing + foreground service.
The pinned experimental development keystore is checked into this repository.
Never use this key for release or production distribution.
Runs only after Tauri android init has generated the Gradle project.
"""
import base64
import os
import pathlib
import re
import subprocess
import sys

root=pathlib.Path(__file__).resolve().parents[1]/"src-tauri"/"gen"/"android"
gradle=root/"app"/"build.gradle.kts"
manifest=root/"app"/"src"/"main"/"AndroidManifest.xml"
if not gradle.is_file() or not manifest.is_file():
    sys.exit("Android generator output absent: run npm run android:init first")
# Intentionally PUBLIC development-only key. Persistent across machines and CI
# runs; convenient for sideloaded experimental builds but insecure for release.
encoded=pathlib.Path(__file__).resolve().parents[1]/"keys"/"experimental-debug.jks.base64"
if not encoded.is_file():
    sys.exit("The checked-in experimental debug key is missing: "+str(encoded))
password="android"
try:
    keystore=base64.b64decode(encoded.read_text().strip(),validate=True)
except Exception as exc:
    sys.exit(f"Invalid checked-in experimental debug key: {exc}")
if len(keystore)<100:
    sys.exit("Experimental debug key unexpectedly short")
file=root/"nova-development.jks"
file.write_bytes(keystore)
file.chmod(0o600)
# The certificate must match the one used for every future Nova update.
# Verify before touching Gradle so users can't accidentally rotate the key.
EXPECTED_SHA256="070b1e684da8087d6c30874c16c9be831b7cbce49264d61040ee73ecb5652ade"
try:
    listed=subprocess.run([
        "keytool","-list","-v","-keystore",str(file),
        "-alias","nova-experimental-dev",
        "-storepass",password,
    ],capture_output=True,text=True,check=True,timeout=30)
except (OSError,subprocess.SubprocessError) as exc:
    file.unlink(missing_ok=True)
    sys.exit("Pinned experimental debug keystore could not be opened. "+str(exc))
found=re.search(r"SHA256:\s*([0-9a-fA-F:]+)",listed.stdout)
digest=found.group(1).replace(":","").lower() if found else ""
if digest!=EXPECTED_SHA256:
    file.unlink(missing_ok=True)
    sys.exit("Pinned experimental debug key does not match its expected certificate; refusing to silently change update signature. Expected fingerprint: "+EXPECTED_SHA256)
prop=root/"keystore.properties"
prop.write_text(f"keyAlias=nova-experimental-dev\nkeyPassword={password}\nstorePassword={password}\nstoreFile={file.as_posix()}\n")
prop.chmod(0o600)
text=gradle.read_text()
if "create(\"novaDev\")" not in text:
    config='''signingConfigs {
        create("novaDev") {
            val properties = java.util.Properties().apply {
                rootProject.file("keystore.properties").inputStream().use { load(it) }
            }
            keyAlias = properties.getProperty("keyAlias")
            keyPassword = properties.getProperty("keyPassword")
            storePassword = properties.getProperty("storePassword")
            storeFile = file(properties.getProperty("storeFile"))
        }
    }
    '''
    if "    buildTypes {" not in text:
        sys.exit("Generated Gradle file lacks expected buildTypes block")
    text=text.replace("    buildTypes {", "    "+config+"buildTypes {",1)
    match=r'(getByName\("debug"\)\s*\{)'
    if re.search(match,text):
        text=re.sub(match,r'\1\n            signingConfig = signingConfigs.getByName("novaDev")',text,count=1)
    else:
        # Tauri's generated Gradle template may only define release explicitly.
        # The Android Gradle plugin creates debug implicitly; configuring it
        # here is supported even when there's no preexisting debug block.
        text=text.replace('buildTypes {','''buildTypes {
        getByName("debug") {
            signingConfig = signingConfigs.getByName("novaDev")
        }
        ''',1)
    gradle.write_text(text)

text=manifest.read_text()
for perm in (
    "android.permission.FOREGROUND_SERVICE",
    "android.permission.FOREGROUND_SERVICE_SPECIAL_USE",
    "android.permission.POST_NOTIFICATIONS",
):
    if perm not in text:
        text=text.replace("<application",f'<uses-permission android:name="{perm}" />\n    <application',1)
# The library manifest defines LifecycleService; host manifest supplies the
# specifically declared special-use subtype for user initiated offline training.
property_name="android.app.PROPERTY_SPECIAL_USE_FGS_SUBTYPE"
if property_name not in text:
    service='''<service android:name="app.tauri.backgroundservice.LifecycleService">
            <property android:name="android.app.PROPERTY_SPECIAL_USE_FGS_SUBTYPE"
                      android:value="On-device neural network training initiated by the user" />
        </service>
        '''
    text=text.replace("</application>",service+"</application>",1)
manifest.write_text(text)
print("Configured PINNED public experimental debug signing, foreground specialUse service and notification permission.")
