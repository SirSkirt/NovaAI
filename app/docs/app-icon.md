# Neural Nova application icon (v0.12.7)

The source artwork is the user's original cyan/purple two-hemisphere brain image, committed on `experimental/neural-nova` as:

`app/src-tauri/icons/icon.png`

**No second upload or new file name is needed.** The repository intentionally may contain only that PNG: Tauri creates all other icon formats at build time. On Windows, `npm run icons:generate` generates `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.ico`, and `icon.icns`. Android must run the generator **after** `npm run android:init` to insert `ic_launcher.png`, `ic_launcher_round.png`, and `ic_launcher_foreground.png` into native `res/mipmap-*` folders. The GitHub Actions Android job also generates the desktop assets *before* init, since the user removed obsolete icon files, and preserves an original temporary source copy for the second pass.

From the `app/` folder:

```sh
npm install
npm run icons:generate
```

The generator copies `src-tauri/icons/icon.png` into a temporary location before calling **`npm run tauri -- icon <temporary-copy>.png`** to avoid collisions with Tauri's generated `icon.png`. The original committed binary is unchanged by the build; in a local working copy Tauri may replace `src-tauri/icons/icon.png` with its generated 512px version, so keep the Git original or a backup if you make a manual edit.

To replace the source icon with another PNG in a local checkout, use `npm run icons:apply -- "C:\\path\\to\\another-square-image.png" --push`. This commits the original image and generated assets to the experimental branch and triggers CI. Otherwise simply changing the source in GitHub is enough: the push workflow automatically rebuilds every platform.

The Windows/Android jobs verify generated icon assets exist; signing key, application identifier, checkpoint format and live brain visualizer remain unchanged.
