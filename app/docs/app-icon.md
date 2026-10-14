# Neural Nova application icon

The icon is the user's cyan/purple two-hemisphere brain artwork.

The canonical checked-in source is `app/assets/nova-brain-icon.png`, an unmodified square, high-resolution PNG. `npm run icons:generate` runs Tauri's official icon generator:

```shell
npm run tauri -- icon ./assets/nova-brain-icon.png
```

Run the command from `app/`. It generates desktop `src-tauri/icons/icon.png`, `icon.ico`, `icon.icns`, and other standard icon sizes. Android icon generation must happen **after `tauri android init`** so Tauri can write the `ic_launcher*.png` resources directly into its generated Android Gradle project.

For a local original icon file, run from `app/`:

```powershell
npm install
npm run icons:apply -- "C:\\Users\\YOU\\Downloads\\Neon Dual-Hemisphere AI Brain(1).png" --push
```

The script validates PNG dimensions, copies the exact bytes to the canonical path, calls Tauri's icon generator, stages **only icon files**, commits and pushes the `experimental/neural-nova` branch, triggering the already existing **Neural Nova - Experimental App Builds** GitHub Actions workflow. Without `--push`, it generates locally without modifying the remote repository.

In GitHub Actions, the Windows job calls Tauri's icon command before building, and the Android job calls it after initialization. If the source PNG has not yet been committed, both jobs retain the previously existing icons; they never silently fabricate a different brand icon.

This is solely a packaging/branding change. Neural architecture, weights, checkpoints, the original brain visualizer, Android app identifier and pinned debug signing certificate are unaffected.
