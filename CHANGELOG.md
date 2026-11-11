# Changelog

## [0.6.6] - 2026-11-11

### Changed
- **UI font is now Monocraft.** The interface uses [Monocraft](https://github.com/IdreesInc/Monocraft) by Idrees Hassan, a pixel font modelled on Minecraft's, licensed under SIL OFL 1.1. The Regular and Bold faces ship as woff2 in `src/assets/fonts/` next to the license text, declared with `@font-face` in `src/styles/tokens.css`. Pixelify Sans and its `@fontsource` dependency were removed.
- Monocraft has a taller x-height than a regular UI face, so component font sizes are one step smaller than before (body 14 → 13 px, 12 → 11 px, and so on). Spacing, padding, radii and the layout are unchanged; only the text metrics moved.
- The About panel's Client row now reads "Fabric + performance mods + HUD" so it fits on one line in the wider font.
- README notes the font and its license.

### Verified
- Main screen, General, HUD and About checked in the running app at 900×700: no clipped or wrapped rows, the HUD module list keeps its alignment, and the About panel's Client row stays on one line.
- `npm run typecheck` clean; `cargo test --lib` 36 passed.

## [0.6.5] - 2026-11-11

### Fixed
- **HUD module list spacing.** Fullbright, Time Changer and Toggle Sprint sat outside the shared module container, so they were separated by the drawer's 22 px section gap while FPS, CPS, Keys, Coords, Day Counter, IP Info, Armor HUD and Ping were stacked together. They now use the same `.hud-modules` container and `.mod-row` layout as every other module: identical padding, row height, text alignment and switch position, with no extra gap before or after them.
- Module rows keep their natural height when a description wraps to two lines; the switch stays vertically centred, so nothing is stretched or clipped.

### Changed
- **Info panel.** The repository link now points to `https://github.com/diamond-launcher/main` (frontend fallback and the Rust `get_app_info` command), the row is labelled GitHub, and the description texts were rewritten in a shorter, plainer tone.

### Verified
- HUD tab checked in the running app at 900×640 and 900×1500: all eleven module rows share the same left text edge (x=970) and switch column (x=1382); the pitch between single-line rows is 54 px and multi-line rows add only their own text height.
- `cargo test --lib` 36 passed, `npm run typecheck` clean.

## [0.6.4] - 2026-11-11

### Fixed
- **"Invalid session" when joining offline-mode servers on Minecraft 26.2/26.3.** Minecraft 26.2 removed `--userType` and selects its offline user service with `--offlineDeveloperMode`. Without that flag the client treated the placeholder access token as a Microsoft session: `/player/attributes` and `/player/certificates` returned 401, Realms logged `Failed to parse into SignedJWT: 0`, and the profile key pair manager tried to upload a key. The launcher now passes the flag, so the client uses `UserApiService.OFFLINE`, `EMPTY_KEY_MANAGER` and an offline authentication service: no Mojang account calls, and multiplayer stays enabled.
- The flag is only added when the client jar actually declares the option, so 1.21.4 and older keep using `--userType legacy` and are never given an argument they would reject.

### Verified
- Real server test on 26.2 + Fabric (`msc.pika.host`): a fresh offline name connected, received the server's registration prompt, and the 401 / Realms / key-pair errors no longer appear in the log.

### Known limitation
- Servers that request authentication even in offline mode (premium name protection, e.g. `origin3l` on `msc.pika.host`) still answer with "Invalid session". The launcher has no Microsoft login, so use another player name or an offline-mode server.

## [0.6.3] - 2026-11-11

### Added
- **Toggle Sprint now has a HUD indicator.** While sprint is locked on, the HUD shows `[Sprint (Toggled)]` in the standard module style (semi-transparent black panel, accent text, no new decoration). It updates the moment the toggle changes and its position is stored in `toggleSprint.x/y`, so it can be dragged in the HUD Layout editor like every other module.
- The launcher's HUD preview lists the new indicator and the layout reset also restores the Time Changer, Ping and Toggle Sprint positions.

### Fixed
- **Time Changer: the slider appeared to do nothing.** The slider only changed the stored time, so unless the module's own switch had been turned on first, the world never changed. Moving the slider now enables Time Changer immediately, and the editor hint explains that turning it off restores the server time.
- Time Changer no longer depends on the HUD overlay master switch: it is a client-side world setting and draws nothing, so it keeps working even when the overlay itself is hidden.
- In-game HUD changes to Fullbright, Ping and Toggle Sprint are now preserved by the launcher's pre-launch config merge instead of being reset from `settings.json`.

### Verified
- In-game visual test: at `timeChangerTime=18000` the sky is night with stars, switching to `6000` while the game runs turns the world to bright noon, and the day counter keeps running (`DAY 2`). Nothing is sent to the server.
- `[Sprint (Toggled)]` was confirmed on screen after pressing the sprint key.

## [0.6.2] - 2026-11-11

### Changed
- **Time Changer now behaves like Lunar Client.** It no longer draws anything on the HUD. Enable it in Modules and open its settings to get a horizontal time slider that updates the displayed world time live while you drag.
- The time is applied only on the client: the sky, sun, moon and stars change for you, while the server, other players and the real world time are untouched. Disabling the module returns to the normal world time immediately.
- The selected time is stored in the HUD config (`timeChangerTime`, 0..23999 ticks) and survives restarts.

### Removed
- **Toggle Sneak module removed completely** (module, setting, config key and UI), including its old config entry. Toggle Sprint is unchanged.

### Fixed
- Time Changer no longer advances the day counter or fights incoming server time updates: the natural clock is restored before each level tick and the override is applied after it, so time keeps flowing normally underneath.

## [0.6.1] - 2026-11-10

### Fixed
- **Settings > HUD could not be opened and made the launcher unresponsive.** The 0.6.0 HUD modules were saved by the backend in camelCase (`timeChanger`, `toggleSneak`, `toggleSprint`) but the settings panel read snake_case (`time_changer`, `toggle_sneak`, `toggle_sprint`). Reading `.enabled` from the missing object threw during render and unmounted the whole React app. The panel now uses the backend's camelCase keys.
- HUD settings are now normalized against defaults before rendering, so a settings file from an older version (or one missing a module) can no longer crash the HUD tab.
- Layout editor: Time Changer and Ping now appear as draggable preview chips, matching the modules rendered in-game.

## [0.6.0] - 2026-11-10

### Added
- Time Changer module: client-side time of day override with slider in module editor. Does not affect server or other players.
- Ping module: shows current server ping on HUD.
- Toggle Sneak module: press Shift once to lock sneak on/off.
- Toggle Sprint module: press sprint key once to lock sprint on/off.

### Fixed
- HUD module text vertical alignment: truly centered with equal top/bottom padding.
- Armor HUD: removed duplicate panel background draw call.
- Corner Radius maximum reduced to 3 to prevent visual distortion.

## [0.5.4] - 2026-11-09

### Fixed
- Update alert: now shows a title ("Yeni Sürüm Mevcut"), action buttons at the bottom, and a darkened backdrop so it is clearly an alert centered on the screen.
- HUD module text vertical alignment: FPS, CPS, Coordinates, Day and IP texts are now truly vertically centered inside their panels (removed the extra top offset).

## [0.5.3] - 2026-11-09

### Fixed
- HUD module text vertical alignment: FPS, CPS, Coordinates, Day and IP texts are now truly vertically centered inside their panels (no top offset).
- Fullbright keybind label: `"key.diamondhud.fullbright"` now shows as "Fullbright Toggle" instead of the raw key name in controls/keybind menus.

## [0.5.2] - 2026-11-09

### Changed
- Update alert: release notes (açıklama) artık kontrol edilirken çekilmiyor; alert ekranın ortasında, kenarlardan bağımsız bir ortalanmış kutu olarak görünür.

## [0.5.1] - 2026-11-09

### Fixed
- **Critical crash: Left Shift causes game crash.** The Fullbright lightmap swap closed Minecraft's original `Lightmap` when disabling fullbright, making its UBO unusable on the next render frame and throwing `IllegalStateException: Buffer already closed` in `Lightmap.render()` (render thread). The original lightmap is now never closed; only the launcher's own `FullbrightLightmap` is closed on disable. Normal sneak (Left Shift) functionality is preserved.

## [0.5.0] - 2026-11-09

### Added
- Fullbright module: rendering-level lightmap brightness (~1000%), enable/disable toggle only, persists across restarts.
- Armor HUD: smaller, cleaner, elegant durability percentages; identical styling in live HUD and layout editor preview.
- Pause menu: Advancements button removed; remaining buttons repositioned.
- GitHub auto-updater: release notes, real download progress, SHA-256 verification, silent install and restart.
- Version aligned to 0.5.0 across all project files.
