package dev.diamond.hud;

import net.fabricmc.fabric.api.client.screen.v1.ScreenEvents;
import net.fabricmc.fabric.api.client.screen.v1.Screens;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.PauseScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.network.chat.Component;

import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.List;

/**
 * Restyles all buttons on TitleScreen and PauseScreen to match the
 * Diamond Client dark/cyan design language, removes unwanted buttons,
 * re-lays out remaining buttons, and customizes specific buttons.
 */
public final class ClientTheme {

    private static final String MODS_LABEL = "Diamond Client";
    private static final String QUIT_LABEL = "X";
    private static final int ACCENT = 0xFF6EE7F0;
    private static final int TEXT = 0xFFEDEEF0;
    private static final int TEXT_MUTED = 0xFF9A9DA4;
    private static final int BG = 0xFF1D1F22;
    private static final int BG_HOVER = 0xFF2B2D31;
    private static final int BORDER = 0xFF3A3D42;
    private static final int BORDER_HOVER = ACCENT;

    private ClientTheme() {
    }

    /** Called once from the mod entrypoint. */
    public static void register() {
        ScreenEvents.AFTER_INIT.register((client, screen, scaledWidth, scaledHeight) -> {
            if (screen instanceof TitleScreen || screen instanceof PauseScreen) {
                restyleAllButtons(screen);
                ensureModsButton(screen);
            }
        });
    }

    /** Buttons to remove from TitleScreen (offline launcher doesn't need these). */
    private static final String[] REMOVED_TITLE_BUTTONS = {
            "Accessibility Options",
            "Accessibility",
            "Language",
            "Language...",
            "Friends",
            "Realms",
            "Social Interactions",
            "Report Player"
    };

    /** Buttons to remove from PauseScreen (offline/singleplayer doesn't need these). */
    private static final String[] REMOVED_PAUSE_BUTTONS = {
            "Friends",
            "Feedback",
            "Report Bugs",
            "Report Bug",
            "Give Feedback",
            "Social Interactions",
            "Accessibility Options",
            "Accessibility",
            "Statistics",
            "Player Report",
            "Advancements"
    };

    /** Replaces every vanilla Button on the screen with a StyledButton preserving its action. */
    private static void restyleAllButtons(Screen screen) {
        List<AbstractWidget> widgets = Screens.getWidgets(screen);
        List<Button> vanillaButtons = new ArrayList<>();

        // Collect all vanilla buttons (not already styled)
        for (AbstractWidget widget : widgets) {
            if (widget instanceof Button btn && !(btn instanceof StyledButton)) {
                vanillaButtons.add(btn);
            }
        }

        if (vanillaButtons.isEmpty()) {
            return;
        }

        // Handle screen-specific customizations
        if (screen instanceof TitleScreen titleScreen) {
            handleTitleScreen(titleScreen, widgets, vanillaButtons);
        } else if (screen instanceof PauseScreen pauseScreen) {
            handlePauseScreen(pauseScreen, widgets, vanillaButtons);
        }

        // Replace remaining with styled versions
        for (Button original : vanillaButtons) {
            if (!widgets.contains(original)) {
                continue; // was removed
            }
            Button.OnPress onPress = getOnPress(original);
            if (onPress == null) {
                continue;
            }

            StyledButton styled = new StyledButton(
                    original.getX(),
                    original.getY(),
                    original.getWidth(),
                    original.getHeight(),
                    original.getMessage(),
                    onPress
            );
            styled.active = original.active;
            styled.visible = original.visible;

            widgets.remove(original);
            try {
                widgets.add(styled);
            } catch (UnsupportedOperationException ignored) {
                widgets.add(original);
            }
        }
    }

    private static void handleTitleScreen(TitleScreen screen, List<AbstractWidget> widgets, List<Button> vanillaButtons) {
        Button quitButton = null;
        Button copyrightButton = null;

        for (Button btn : vanillaButtons) {
            String label = btn.getMessage().getString();
            if (shouldRemoveTitleButton(label)) {
                widgets.remove(btn);
            } else if (label.equals("Quit Game")) {
                quitButton = btn;
            } else if (label.contains("Copyright") || label.contains("Mojang")) {
                copyrightButton = btn;
            }
        }

        // Re-layout remaining buttons (excluding quit and copyright which we'll reposition)
        relayoutTitleButtons(screen, widgets, vanillaButtons, quitButton, copyrightButton);

        // Reposition quit button next to Diamond Client button (top-left)
        if (quitButton != null) {
            quitButton.setMessage(Component.literal(QUIT_LABEL));
            quitButton.setWidth(30);
            quitButton.setHeight(22);
            quitButton.setX(10 + 130 + 8); // right of Diamond Client button
            quitButton.setY(10);
        }

        // Reposition copyright button to bottom-right
        if (copyrightButton != null) {
            copyrightButton.setX(screen.width - copyrightButton.getWidth() - 10);
            copyrightButton.setY(screen.height - copyrightButton.getHeight() - 10);
        }
    }

    private static void handlePauseScreen(PauseScreen screen, List<AbstractWidget> widgets, List<Button> vanillaButtons) {
        // Remove unwanted buttons
        for (Button btn : vanillaButtons) {
            String label = btn.getMessage().getString();
            if (shouldRemovePauseButton(label)) {
                widgets.remove(btn);
            }
        }

        // Re-layout remaining buttons (no special handling for quit button - keep it normal)
        relayoutPauseButtons(screen, widgets, vanillaButtons, null);
    }

    private static void relayoutTitleButtons(TitleScreen screen, List<AbstractWidget> widgets, List<Button> vanillaButtons,
                                             Button quitButton, Button copyrightButton) {
        // Collect buttons that remain (not removed, not quit, not copyright)
        List<Button> keep = new ArrayList<>();
        for (Button btn : vanillaButtons) {
            if (widgets.contains(btn)
                    && !shouldRemoveTitleButton(btn.getMessage().getString())
                    && btn != quitButton
                    && btn != copyrightButton) {
                keep.add(btn);
            }
        }
        if (keep.isEmpty()) return;

        int btnWidth = 200;
        int btnHeight = 20;
        int gap = 8;
        int totalHeight = keep.size() * btnHeight + (keep.size() - 1) * gap;
        // Keep the group closer to the logo (which occupies roughly y=30..74) while
        // still centering it on tall screens and never running off the bottom edge.
        int startY = Math.max(90, Math.min(screen.height / 4 + 48, screen.height - 12 - totalHeight));
        int centerX = screen.width / 2 - btnWidth / 2;

        for (int i = 0; i < keep.size(); i++) {
            Button btn = keep.get(i);
            btn.setX(centerX);
            btn.setY(startY + i * (btnHeight + gap));
            btn.setWidth(btnWidth);
            btn.setHeight(btnHeight);
        }
    }

    private static void relayoutPauseButtons(PauseScreen screen, List<AbstractWidget> widgets, List<Button> vanillaButtons,
                                             Button quitButton) {
        // Collect buttons that remain (not removed, not quit)
        List<Button> keep = new ArrayList<>();
        for (Button btn : vanillaButtons) {
            if (widgets.contains(btn)
                    && !shouldRemovePauseButton(btn.getMessage().getString())
                    && btn != quitButton) {
                keep.add(btn);
            }
        }
        if (keep.isEmpty()) return;

        int btnWidth = 200;
        int btnHeight = 20;
        int gap = 8;
        int totalHeight = keep.size() * btnHeight + (keep.size() - 1) * gap;
        int startY = (screen.height - totalHeight) / 2;
        int centerX = screen.width / 2 - btnWidth / 2;

        for (int i = 0; i < keep.size(); i++) {
            Button btn = keep.get(i);
            btn.setX(centerX);
            btn.setY(startY + i * (btnHeight + gap));
            btn.setWidth(btnWidth);
            btn.setHeight(btnHeight);
        }
    }

    private static boolean shouldRemoveTitleButton(String label) {
        String lower = label.toLowerCase();
        for (String removed : REMOVED_TITLE_BUTTONS) {
            if (removed.equalsIgnoreCase(label) || lower.contains(removed.toLowerCase())) {
                return true;
            }
        }
        return false;
    }

    private static boolean shouldRemovePauseButton(String label) {
        String lower = label.toLowerCase();
        for (String removed : REMOVED_PAUSE_BUTTONS) {
            if (removed.equalsIgnoreCase(label) || lower.contains(removed.toLowerCase())) {
                return true;
            }
        }
        return false;
    }

    private static Button.OnPress getOnPress(Button button) {
        try {
            Field field = Button.class.getDeclaredField("onPress");
            field.setAccessible(true);
            return (Button.OnPress) field.get(button);
        } catch (ReflectiveOperationException e) {
            return null;
        }
    }

    /** Adds the "Diamond Client" button if not already present. */
    private static void ensureModsButton(Screen screen) {
        List<AbstractWidget> widgets = Screens.getWidgets(screen);
        for (AbstractWidget widget : widgets) {
            if (widget instanceof Button && MODS_LABEL.equals(widget.getMessage().getString())) {
                return;
            }
        }

        int x = 10;
        int y = (screen instanceof TitleScreen) ? 10 : Math.max(10, screen.height - 30);

        Button button = new StyledButton(x, y, 130, 22, Component.literal(MODS_LABEL),
                b -> Minecraft.getInstance().gui.setScreen(
                        new HudConfigScreen(screen, DiamondHud.configPath())));

        try {
            widgets.add(button);
        } catch (UnsupportedOperationException ignored) {
        }
    }

    /** Button that renders with the same dark/cyan style as HudConfigScreen. */
    private static final class StyledButton extends Button {

        StyledButton(int x, int y, int width, int height, Component message, OnPress onPress) {
            super(x, y, width, height, message, onPress, DEFAULT_NARRATION);
        }

        @Override
        protected void extractContents(GuiGraphicsExtractor g, int mouseX, int mouseY, float partialTick) {
            boolean hovered = isHoveredOrFocused() && active;

            int background = hovered ? BG_HOVER : BG;
            int border = hovered ? BORDER_HOVER : BORDER;
            int foreground = hovered ? ACCENT : TEXT;
            if (!active) {
                background = 0xFF15171A;
                foreground = TEXT_MUTED;
                border = 0xFF2A2D31;
            }

            g.fill(getX(), getY(), getX() + getWidth(), getY() + getHeight(), background);

            g.fill(getX(), getY(), getX() + getWidth(), getY() + 1, border);
            g.fill(getX(), getY() + getHeight() - 1, getX() + getWidth(), getY() + getHeight(), border);
            g.fill(getX(), getY() + 1, getX() + 1, getY() + getHeight() - 1, border);
            g.fill(getX() + getWidth() - 1, getY() + 1, getX() + getWidth(), getY() + getHeight() - 1, border);

            net.minecraft.client.gui.Font f = Minecraft.getInstance().font;
            String label = getMessage().getString();
            g.centeredText(f, label, getX() + getWidth() / 2,
                    getY() + (getHeight() - f.lineHeight) / 2 + 1, foreground);
        }
    }
}