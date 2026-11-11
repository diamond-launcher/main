# Diamond Launcher

A minimalist Minecraft launcher: pick a version, install, press Play. Built with Tauri 2, React, TypeScript (strict), Rust and Vite.

Offline profile only: there is **no** Microsoft/Xbox authentication and no ownership verification. The launcher passes a locally stored username and a deterministic offline UUID.

Some servers still ask the client to authenticate, or protect premium names, even when they accept offline players. Those reject an offline profile with `Invalid session` (or `You cannot join the server`). That is a server-side policy and cannot be worked around without a real Microsoft account: use an offline-mode server or pick a different player name.

## Run

```
npm install
npm run tauri dev                      # development
npm run tauri build -- --no-bundle     # release exe in src-tauri/target/release
npm run typecheck                      # TypeScript strict check
npm run test:rust                      # Rust unit tests
```

Requires Node 18+, Rust (stable, MSVC on Windows) and WebView2.

End-to-end check (downloads from Mojang, installs, starts the real game for 20 s):

```
$env:E2E_VERSION="1.21.4"; cargo test --manifest-path src-tauri/Cargo.toml --lib e2e -- --ignored --nocapture
```

## Architecture

Frontend (`src/`) only renders state and calls the backend; all system work is in Rust.

| Path | Responsibility |
| --- | --- |
| `src/hooks/useLauncher.ts` | Single state hook: settings, versions, install progress, game status, errors |
| `src/api/` | Typed `invoke`/event wrappers and shared types (mirror Rust serde models, camelCase) |
| `src/components/` | Presentational components (PlayButton, VersionPicker, SettingsPanel, ...) |
| `src/styles/` | Design tokens, base, component CSS |
| `src-tauri/src/commands.rs` | Tauri commands (thin) |
| `manifest.rs` | Official version manifest (cached for offline) and per-version JSON |
| `meta.rs`, `rules.rs` | Serde models of version JSON; OS/arch/feature rule evaluation |
| `plan.rs` | Metadata -> files: libraries, classpath, natives, asset objects, log config |
| `download.rs` | Concurrent downloads, SHA-1/size verification, retries, byte-accurate progress |
| `java.rs` | Java probing, system discovery, Mojang-managed runtime download |
| `install.rs` | Install/repair pipeline, emits `install-progress` events |
| `launch.rs` | Argument construction from metadata, process supervision, emits `game-status` |
| `system.rs`, `settings.rs`, `paths.rs` | RAM detection/recommendation, persisted settings, directory layout |

Events: `install-progress` (phase: metadata, java, verify, download, finalize, done) and `game-status` (starting, running, exited, crashed + log tail).

Data lives in `%APPDATA%\Diamond` (`settings.json`, manifest cache) and, by default, `%APPDATA%\Diamond\minecraft` (standard `versions/ libraries/ assets/ runtime/` layout). Both game dir and Java are overridable in Settings.

UI font: [Monocraft](https://github.com/IdreesInc/Monocraft) by Idrees Hassan, licensed under SIL OFL 1.1. The woff2 files and the license live in `src/assets/fonts/` and are declared with `@font-face` in `src/styles/tokens.css`. Monocraft has a taller x-height than a regular UI face, so component font sizes are set one step below the previous values to keep the same visual weight.

Design rules: launch arguments come from version JSON rules (legacy `minecraftArguments` gets only the universal `-Djava.library.path`/`-cp`), with one exception: `--offlineDeveloperMode` is added when the client jar declares it, because 26.2+ uses that flag instead of `--userType` to select the offline user API. Progress is only shown from real byte counts, other phases are indeterminate.

Error codes returned to the UI (`AppError.code`): `network`, `checksum`, `not_installed`, `java_missing`, `java_too_old`, `mods_outdated`, `invalid_*`, `busy`, `launch_failed`, ... The UI auto-repairs on `not_installed`/`java_missing`/`mods_outdated` when pressing Play.

## Diamond identity and mods

- Fabric + performance mods (Sodium, Lithium, Zoomify, …) are installed per version and can be toggled in Settings → Mods (`src-tauri/src/mods.rs`).
- The game reports itself as **Diamond Launcher**: `--versionType` and `launcher_name` are set in the generated arguments, so the version list and logs show "Diamond Launcher".
- Intro splash and staggered entrance animation live in `App.tsx` / `components.css` (disabled under `prefers-reduced-motion`).

### Client HUD

The launcher ships its own Fabric client mod (`hud/`, mod id `diamondhud`) that draws an in-game overlay: FPS, CPS, keystrokes (WASD, mouse buttons, Shift, Space) and block coordinates. It is available for Minecraft **26.2+**; older versions run without it.

- Everything is configurable in Settings → HUD: per-module switches, drag-and-drop positions on a 16:9 preview, global scale, background opacity, accent and text colours.
- Settings are stored in `settings.json` and written to `diamond-hud.properties` in the game directory before each launch. The mod re-reads that file once per second, so changes apply without restarting the game.
- The compiled jar lives at `src-tauri/resources/diamond-hud.jar` and is embedded into the launcher. Rebuild it with `hud/build.ps1` (needs JDK 25+ and an installed 26.2/26.3 version with Fabric).
- When the HUD is enabled the launcher also ensures Fabric API is present, even if performance mods are turned off.

## School networks (MEB / filtered networks)

The installer is per-user (`installMode: currentUser`), so no admin rights are needed.

Hosts the launcher contacts (all HTTPS):

| Host | Used for | Required? |
| --- | --- | --- |
| `piston-meta.mojang.com`, `launchermeta.mojang.com` | Version manifest, version JSON, Java runtime index | Yes |
| `piston-data.mojang.com` | Java runtime files | Yes (for managed Java) |
| `libraries.minecraft.net`, `resources.download.minecraft.net` | Libraries, client jar, assets | Yes |
| `meta.fabricmc.net`, `maven.fabricmc.net` | Fabric loader profile and libraries | Only for mods |
| `api.modrinth.com` and the file CDN it returns (`cdn.modrinth.com`) | Mod metadata and jars | Only for mods |

Mods are optional. If Fabric or Modrinth is blocked, the install does **not** fail: the version runs as vanilla and the reason is shown in Settings → Mods. Press "Verify selected version" (Settings → Advanced) to retry once the network allows it.

Check reachability from PowerShell:

```
"piston-meta.mojang.com","launchermeta.mojang.com","piston-data.mojang.com","libraries.minecraft.net","resources.download.minecraft.net","meta.fabricmc.net","maven.fabricmc.net","api.modrinth.com" | % { "{0}: {1}" -f $_, (Test-NetConnection $_ -Port 443 -WarningAction SilentlyContinue).TcpTestSucceeded }
```

Code signing: the installer is unsigned, so SmartScreen or AppLocker policies may warn or block it. A signing certificate is required to remove that.

## Releasing (GitHub)

The public repository only carries the installer, not the source code. Each release is a GitHub Release with a single asset named `DiamondLauncher-<version>-x64-setup.exe`. The launcher's update check reads `releases/latest` and looks for an asset ending in `setup.exe`.

1. Bump the version in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml` (all three must match).
2. Build the installer on Windows: `npx tauri build --bundles nsis`. The output is in `src-tauri/target/release/bundle/nsis/`.
3. Create a GitHub Release with tag `v<version>` and attach the installer, renamed to `DiamondLauncher-<version>-x64-setup.exe`. Do not attach or push source files.

The update flow: the launcher checks GitHub about 4 seconds after start and every 30 minutes. If the latest tag is newer, a banner offers "Update now". The installer is downloaded, verified against the asset's SHA-256 digest when GitHub provides one, and run silently. Updates are refused while a game or install is running. Installs made before this feature (0.1.0) must be upgraded once by hand.

## Status

Done and verified (36 unit tests; real install + launch of 1.8.9, 1.21.4 and 26.2, including a real offline-mode server join on 26.2; release build runs and shows the live version list):

1. Shell and project structure
2. Visual interface (main screen, version picker, settings drawer, progress, errors)
3. Version manifest, release/snapshot selection, last version restored
4. Install engine: client jar, libraries, natives, asset index + objects (incl. legacy virtual assets), log4j config, Mojang Java runtime, SHA-1 verification and repair
5. Java detection (custom > managed runtime > system), automatic RAM (40% of RAM, 1-8 GB, leaves headroom)
6. Launching, process tracking, closure/crash detection with log tail, Stop
7. Settings: username, RAM, Java path, game dir, JVM args; persisted
8. Update check against GitHub Releases with one-click silent install (`src-tauri/src/updater.rs`); unit tested only, not run end-to-end
9. Custom in-game HUD (FPS, CPS, keystrokes, coordinates) with a live layout editor (`hud/`, `src-tauri/src/hud.rs`); verified in-game on 26.2 and 26.3

Not implemented / known limits:

- No cancel button for downloads.
- Versions using `inheritsFrom` (Forge/Fabric profiles) are rejected with a clear message; only official versions are listed. Old alpha/beta are not listed.
- Frontend has no automated tests (type-checked only); the UI was only smoke-tested visually, not every interaction.
- Tested on Windows x64 only; macOS/Linux code paths are written but untested.
