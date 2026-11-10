# Changelog

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
