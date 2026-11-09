package dev.diamond.hud;

import net.fabricmc.fabric.api.client.rendering.v1.hud.HudElementRegistry;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.resources.Identifier;

/** Registers the overlay, isolated so the mod can run without Fabric API present. */
final class HudRegistrar {
    private HudRegistrar() {
    }

    static void register() {
        HudElementRegistry.addLast(
                Identifier.fromNamespaceAndPath("diamondhud", "overlay"),
                new HudRenderer(FabricLoader.getInstance().getGameDir().resolve("diamond-hud.properties"))
        );
        System.out.println("[Diamond HUD] overlay registered");
    }
}
