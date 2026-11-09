package dev.diamond.hud;

import net.fabricmc.api.EnvType;
import net.fabricmc.api.Environment;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;

import java.nio.file.Path;
import java.nio.file.Paths;

/** Mod Menu integration: provides a config screen factory. */
@Environment(EnvType.CLIENT)
public final class ModMenuIntegration {

    private static final Path CONFIG_PATH = Paths.get("diamond-hud.properties");

    private ModMenuIntegration() {}

    public static Screen createConfigScreen(Screen parent) {
        return new HudConfigScreen(parent, CONFIG_PATH);
    }

    public static Component getModName() {
        return Component.literal("Diamond HUD");
    }

    public static String getModId() {
        return "diamondhud";
    }
}