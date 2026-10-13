#!/usr/bin/env python3
"""Configure stable development signing + foreground-service manifest for generated Tauri Android project.
Runs ONLY after: npm run android:init -- --ci --skip-targets-install
No keystores/passwords/private keys are present in the public repository.
"""
import base64
import os
import pathlib
import re
import sys

root=pathlib.Path(__file__).resolve().parents[1]/"src-tauri"/"gen"/"android"
gradle=root/"app"/"build.gradle.kts"
manifest=root/"app"/"src"/"main"/"AndroidManifest.xml"
if not gradle.is_file() or not manifest.is_file():
    sys.exit("Android generator output absent: run npm run android:init first")
secret=os.environ.get("NOVA_ANDROID_KEYSTORE_BASE64","").strip()
password=os.environ.get("NOVA_ANDROID_KEYSTORE_PASSWORD","").strip()
if not secret or not password:
    sys.exit("Stable Android development signing is not configured. Set repository Actions secrets NOVA_ANDROID_KEYSTORE_BASE64 and NOVA_ANDROID_KEYSTORE_PASSWORD. Refusing ephemeral debug signing.")
if any(c in password for c in "\n\r\\"):
    sys.exit("Invalid Android signing password for .properties format")
try: keystore=base64.b64decode(secret,validate=True)
except Exception as exc: sys.exit(f"Invalid base64 keystore data: {exc}")
if len(keystore)<100:
    sys.exit("Signing key contents are unexpectedly short")
file=root/"nova-development.jks"
file.write_bytes(keystore)
file.chmod(0o600)
prop=root/"keystore.properties"
prop.write_text(f"keyAlias=nova-experimental-dev\nkeyPassword={password}\nstorePassword={password}\nstoreFile={file.as_posix()}\n")
prop.chmod(0o600)
text=gradle.read_text()
if "create(\"novaDev\")" not in text:
    config='''signingConfigs {
        create("novaDev") {
            val properties = Properties().apply {
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
    if not re.search(match,text):
        sys.exit("Generated Gradle file lacks debug build configuration")
    text=re.sub(match,r'\1\n            signingConfig = signingConfigs.getByName("novaDev")',text,count=1)
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
print("Configured stable Android development signing, foreground specialUse service and runtime notification permission.")
