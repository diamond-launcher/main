package dev.diamond.hud;

import com.mojang.blaze3d.platform.InputConstants;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keymapping.v1.KeyMappingHelper;
import net.fabricmc.fabric.api.client.rendering.v1.hud.HudElementRegistry;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.GameRenderer;
import net.minecraft.client.renderer.Lightmap;
import net.minecraft.client.renderer.state.LightmapRenderState;
import net.minecraft.resources.Identifier;

import java.lang.reflect.Field;
import java.nio.file.Path;
import java.nio.file.Paths;

/** Registers the HUD element and keybind. */
public final class DiamondHud implements ClientModInitializer {

    private static final String MOD_ID = "diamondhud";
    private static final Path CONFIG_PATH = Paths.get("diamond-hud.properties");
    private static final HudConfig CONFIG = new HudConfig(CONFIG_PATH);

    // Keybind to open config screen
    public static KeyMapping OPEN_CONFIG;

    // Fullbright keybind
    public static KeyMapping FULLBRIGHT;

    private static boolean wasPressed = false;
    /** Forces the first in-game frame to re-apply the config key over options.txt. */
    private static int lastKnownKeybind = Integer.MIN_VALUE;

    /** Keyboard type constant, differs between 26.2 (KEYSYM) and 26.3 (KEYBOARD). */
    private static InputConstants.Type KEYBOARD_TYPE;

    // Fullbright state
    private static boolean fullbrightEnabled = false;
    private static boolean fullbrightWasPressed = false;
    /** The regular lightmap, saved so fullbright can be reverted instantly. */
    private static Lightmap originalLightmap;
    /** Whether a FullbrightLightmap is currently swapped into the renderer. */
    private static boolean lightmapSwapped = false;

    @Override
    public void onInitializeClient() {
        // Detect keyboard type constant for this MC version
        initKeyboardType();

        CONFIG.refresh();
        int keybind = CONFIG.get().keybind;

        // Create the keybinding with the config value so the Controls menu shows it.
        OPEN_CONFIG = new KeyMapping(
                "key.diamondhud.open_config",
                keybind,
                KeyMapping.Category.MISC
        );

        // Fullbright keybinding
        FULLBRIGHT = new KeyMapping(
                "key.diamondhud.fullbright",
                InputConstants.UNKNOWN.getValue(),
                KeyMapping.Category.MISC
        );

        HudElementRegistry.addLast(Identifier.fromNamespaceAndPath(MOD_ID, "hud"), new HudRenderer(CONFIG_PATH));
        KeyMappingHelper.registerKeyMapping(OPEN_CONFIG);
        KeyMappingHelper.registerKeyMapping(FULLBRIGHT);

        // Client-only time of day override. Restore before the level tick and apply
        // after it, so the world renders the configured time without touching the server.
        ClientTickEvents.START_CLIENT_TICK.register(client -> TimeChanger.beforeTick());
        ClientTickEvents.END_CLIENT_TICK.register(client -> TimeChanger.afterTick(CONFIG.get()));

        // Themed main menu, pause menu and mod menu button (Fabric Screen API).
        try {
            ClientTheme.register();
        } catch (Throwable ignored) {
            // Keep the HUD usable even if a minimal Fabric API build lacks the screen module.
        }
    }

    /** Detects the correct InputConstants.Type for keyboard (KEYSYM in 26.2, KEYBOARD in 26.3+). */
    private static void initKeyboardType() {
        try {
            // Try 26.3+ name first
            Field f = InputConstants.Type.class.getDeclaredField("KEYBOARD");
            KEYBOARD_TYPE = (InputConstants.Type) f.get(null);
        } catch (NoSuchFieldException e) {
            // 26.2 uses KEYSYM
            try {
                Field f = InputConstants.Type.class.getDeclaredField("KEYSYM");
                KEYBOARD_TYPE = (InputConstants.Type) f.get(null);
            } catch (Exception ex) {
                throw new IllegalStateException("Cannot find keyboard type constant", ex);
            }
        } catch (Exception e) {
            throw new IllegalStateException("Cannot access keyboard type constant", e);
        }
    }

    /** Config file shared with the in-game menu and the launcher. */
    public static Path configPath() {
        return CONFIG_PATH;
    }

    /** The live config instance shared by the tick handlers and the in-game menu. */
    static HudConfig config() {
        return CONFIG;
    }

    /**
     * Reads the configured key and applies it to the live {@link KeyMapping}.
     *
     * <p>Using the public {@link KeyMapping#setKey} API keeps this compatible with both
     * 26.2 and 26.3, whose {@code InputConstants.isKeyDown} signatures differ.
     * Only {@code setKey} is called here; registration happens once in {@link #onInitializeClient}.</p>
     */
    public static void applyKeybind(int keyCode) {
        lastKnownKeybind = keyCode;
        wasPressed = false;
        if (OPEN_CONFIG != null && KEYBOARD_TYPE != null) {
            OPEN_CONFIG.setKey(KEYBOARD_TYPE.getOrCreate(keyCode));
            // Do NOT re-register; that can only happen once during GameOptions init.
        }
    }

    /** Called every HUD frame. Opens the config screen when the configured key is pressed. */
    public static void checkKeybind() {
        Minecraft mc = Minecraft.getInstance();
        if (mc.player == null || mc.gui.screen() != null) {
            wasPressed = false;
            fullbrightWasPressed = false;
            return;
        }

        CONFIG.refresh();
        int currentKeybind = CONFIG.get().keybind;
        if (currentKeybind != lastKnownKeybind) {
            applyKeybind(currentKeybind);
        }

        boolean pressed = OPEN_CONFIG.isDown();
        if (pressed && !wasPressed) {
            mc.gui.setScreen(new HudConfigScreen(null, CONFIG_PATH));
        }
        wasPressed = pressed;

        // Handle fullbright toggle
        if (FULLBRIGHT != null) {
            boolean fbPressed = FULLBRIGHT.isDown();
            if (fbPressed && !fullbrightWasPressed) {
                fullbrightEnabled = !fullbrightEnabled;
                CONFIG.get().fullbright.enabled = fullbrightEnabled;
                CONFIG.write(CONFIG.get()); // persist in-game toggles across restarts
                applyFullbright(mc);
            }
            fullbrightWasPressed = fbPressed;
        }

        // Keep fullbright in sync with the config file (edited by the launcher or the in-game menu).
        boolean wantedFullbright = CONFIG.get().fullbright.enabled;
        if (wantedFullbright != fullbrightEnabled) {
            fullbrightEnabled = wantedFullbright;
            applyFullbright(mc);
        }
    }

    /** Returns whether fullbright is currently enabled. */
    public static boolean isFullbrightEnabled() {
        return fullbrightEnabled;
    }

    /**
     * Applies or removes fullbright by manipulating the render-level lightmap brightness
     * (roughly 1000%), not the brightness slider. Dark caves and nighttime stay clearly
     * visible; disabling restores normal lighting immediately.
     */
    private static void applyFullbright(Minecraft mc) {
        if (mc == null || mc.gameRenderer == null) {
            return;
        }
        GameRenderer renderer = mc.gameRenderer;
        Field lightmapField;
        try {
            lightmapField = GameRenderer.class.getDeclaredField("lightmap");
            lightmapField.setAccessible(true);
        } catch (NoSuchFieldException ignored) {
            return;
        }
        try {
            if (fullbrightEnabled) {
                if (!lightmapSwapped) {
                    originalLightmap = (Lightmap) lightmapField.get(renderer);
                    lightmapField.set(renderer, new FullbrightLightmap());
                    lightmapSwapped = true;
                }
            } else if (lightmapSwapped) {
                // Close our own FullbrightLightmap, NEVER the game's original lightmap.
                // Closing the original lightmap makes its UBO unusable on the next render
                // frame, causing "Buffer already closed" (IllegalStateException) in
                // Lightmap.render -> a hard crash on the render thread.
                Lightmap current = (Lightmap) lightmapField.get(renderer);
                if (current instanceof FullbrightLightmap) {
                    try {
                        current.close();
                    } catch (Exception ignored) {
                    }
                }
                lightmapField.set(renderer, originalLightmap);
                lightmapSwapped = false;
                originalLightmap = null;
            }
        } catch (IllegalAccessException ignored) {
        }
    }

    /** Lightmap override that forces the render-state brightness to maximum (~1000%). */
    private static final class FullbrightLightmap extends Lightmap {
        @Override
        public void render(LightmapRenderState state) {
            state.brightness = Math.max(state.brightness, 10.0f);
            super.render(state);
        }
    }
}
