package dev.diamond.hud;

import net.fabricmc.fabric.api.client.rendering.v1.hud.HudElement;
import net.minecraft.client.DeltaTracker;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.world.entity.EquipmentSlot;
import net.minecraft.world.item.ItemStack;

import java.nio.file.Path;
import java.util.ArrayDeque;

/** Draws every enabled Diamond HUD module in Minecraft style: text with semi-transparent black background, no borders. */
public final class HudRenderer implements HudElement {
    private static final int PANEL = 0x000000; // pure black
    private static final int GAP = 3;
    private static final int PAD = 6;
    /// Durability percentage is drawn below each icon, scaled down for a small, clean, elegant look.
    private static final float DURABILITY_SCALE = 0.5f;
    /// Gap between the bottom of an armor icon and its durability percentage.
    private static final int DURABILITY_GAP = 2;

    public static final String FPS = "fps";
    public static final String CPS = "cps";
    public static final String KEYSTROKES = "keystrokes";
    public static final String COORDS = "coords";
    public static final String DAY = "day";
    public static final String IP = "ip";
    public static final String ARMOR = "armor";
    // FULLBRIGHT is not a HUD module - it's a background gamma toggle
    public static final String[] TYPES = {FPS, CPS, KEYSTROKES, COORDS, DAY, IP, ARMOR};

    private final HudConfig config;
    private final ArrayDeque<Long> leftClicks = new ArrayDeque<>();
    private final ArrayDeque<Long> rightClicks = new ArrayDeque<>();
    private boolean leftWasDown;
    private boolean rightWasDown;

    HudRenderer(Path configPath) {
        this.config = new HudConfig(configPath);
    }

    @Override
    public void extractRenderState(GuiGraphicsExtractor graphics, DeltaTracker delta) {
        Minecraft mc = Minecraft.getInstance();
        if (mc == null || mc.player == null || mc.options == null) {
            return;
        }
        config.refresh();
        HudConfig.Settings settings = config.get();

        // Check keybind to open config screen
        DiamondHud.checkKeybind();

        if (!settings.enabled) {
            return;
        }

        updateClicks(mc);
        float w = graphics.guiWidth();
        float h = graphics.guiHeight();
        drawModuleAt(graphics, FPS, settings, settings.fps, mc, (float) (settings.fps.x * w), (float) (settings.fps.y * h), 1.0f);
        drawModuleAt(graphics, CPS, settings, settings.cps, mc, (float) (settings.cps.x * w), (float) (settings.cps.y * h), 1.0f);
        drawModuleAt(graphics, KEYSTROKES, settings, settings.keystrokes, mc,
                (float) (settings.keystrokes.x * w), (float) (settings.keystrokes.y * h), 1.0f);
        drawModuleAt(graphics, COORDS, settings, settings.coords, mc, (float) (settings.coords.x * w), (float) (settings.coords.y * h), 1.0f);
        drawModuleAt(graphics, DAY, settings, settings.day, mc, (float) (settings.day.x * w), (float) (settings.day.y * h), 1.0f);
        drawModuleAt(graphics, IP, settings, settings.ip, mc, (float) (settings.ip.x * w), (float) (settings.ip.y * h), 1.0f);
        drawModuleAt(graphics, ARMOR, settings, settings.armor, mc, (float) (settings.armor.x * w), (float) (settings.armor.y * h), 1.0f);
    }

    /**
     * Draws one module with its top-left corner at {@code (x, y)} in GUI coordinates.
     *
     * <p>{@code extraScale} lets the config screen render a true miniature of the real
     * screen. The module's own {@code c.scale} is always applied, so preview and
     * in-game output are identical apart from that single multiplier.</p>
     *
     * @param type one of {@link #FPS}, {@link #CPS}, {@link #KEYSTROKES}, {@link #COORDS}
     * @param mc   {@code null} while drawing the preview, so simulated values are used
     */
    public void drawModuleAt(
            GuiGraphicsExtractor g,
            String type,
            HudConfig.Settings c,
            HudConfig.Module module,
            Minecraft mc,
            float x,
            float y,
            float extraScale
    ) {
        if (!module.enabled) {
            return;
        }
        g.pose().pushMatrix();
        g.pose().translate(x, y);
        float scale = c.scale * extraScale;
        if (scale != 1.0f) {
            g.pose().scale(scale, scale);
        }
        switch (type) {
            case FPS -> fpsContent(g, c, mc);
            case CPS -> cpsContent(g, c, mc);
            case KEYSTROKES -> keystrokesContent(g, c, mc);
            case COORDS -> coordsContent(g, c, mc);
            case DAY -> dayContent(g, c, mc);
            case IP -> ipContent(g, c, mc);
            case ARMOR -> armorContent(g, c, mc);
            default -> { }
        }
        g.pose().popMatrix();
    }

    /**
     * Pixel size of a module after {@code c.scale} and {@code extraScale} are applied.
     * Used by the config screen so hit-testing matches exactly what is drawn.
     */
    public int[] moduleSize(String type, HudConfig.Settings c, HudConfig.Module module, Minecraft mc, float extraScale) {
        Font font = font(mc);
        int width = 0;
        int height = 0;
        switch (type) {
            case FPS -> {
                width = textWidth(font, "FPS", fpsValue(mc));
                height = textHeight(font, 1);
            }
            case CPS -> {
                width = Math.max(textWidth(font, "LMB", cpsLeft(mc)), textWidth(font, "RMB", cpsRight(mc)));
                height = textHeight(font, 2) + GAP;
            }
            case COORDS -> {
                width = textWidth(font, "XYZ", coordsValue(mc));
                height = textHeight(font, 1);
            }
            case KEYSTROKES -> {
                int cell = 20;
                int row = cell + GAP;
                int gridWidth = cell * 3 + GAP * 2;
                int pad = 5;
                width = pad + 2 + gridWidth + pad;
                height = pad + row * 4 - GAP + pad;
            }
            case DAY -> {
                width = textWidth(font, "DAY", dayValue(mc));
                height = textHeight(font, 1);
            }
            case IP -> {
                width = textWidth(font, "IP", ipValue(mc));
                height = textHeight(font, 1);
            }
            case ARMOR -> {
                int cell = 16;
                int pad = 4;
                int slots = 4;
                int durHeight = Math.max(1, Math.round(font.lineHeight * DURABILITY_SCALE));
                width = pad + slots * cell + (slots - 1) * GAP + pad;
                height = pad + cell + DURABILITY_GAP + durHeight + pad; // matches the live panel
            }
            default -> { }
        }
        float scale = c.scale * extraScale;
        return new int[]{
                Math.max(1, Math.round(width * scale)),
                Math.max(1, Math.round(height * scale))
        };
    }

    /** FPS module content, drawn at the current pose origin. */
    private void fpsContent(GuiGraphicsExtractor g, HudConfig.Settings c, Minecraft mc) {
        Font font = font(mc);
        String label = "FPS";
        String value = fpsValue(mc);
        int width = textWidth(font, label, value);
        int height = textHeight(font, 1);

        panel(g, width, height, c);
        int textY = PAD + font.lineHeight / 2 + 1;
        g.text(font, label, PAD, textY, c.accent, false);
        g.text(font, value, PAD + font.width(label) + 5, textY, c.text, false);
    }

    /** CPS module content, drawn at the current pose origin. */
    private void cpsContent(GuiGraphicsExtractor g, HudConfig.Settings c, Minecraft mc) {
        Font font = font(mc);
        String left = cpsLeft(mc);
        String right = cpsRight(mc);
        int width = Math.max(textWidth(font, "LMB", left), textWidth(font, "RMB", right));
        int height = textHeight(font, 2) + GAP;

        panel(g, width, height, c);
        int line1Y = PAD + font.lineHeight / 2 + 1;
        int line2Y = line1Y + font.lineHeight + GAP;
        g.text(font, "LMB", PAD, line1Y, c.accent, false);
        g.text(font, left, PAD + font.width("LMB") + 5, line1Y, c.text, false);
        g.text(font, "RMB", PAD, line2Y, c.accent, false);
        g.text(font, right, PAD + font.width("RMB") + 5, line2Y, c.text, false);
    }

    /** Coordinates module content, drawn at the current pose origin. */
    private void coordsContent(GuiGraphicsExtractor g, HudConfig.Settings c, Minecraft mc) {
        Font font = font(mc);
        String value = coordsValue(mc);
        int width = textWidth(font, "XYZ", value);
        int height = textHeight(font, 1);

        panel(g, width, height, c);
        int textY = PAD + font.lineHeight / 2 + 1;
        g.text(font, "XYZ", PAD, textY, c.accent, false);
        g.text(font, value, PAD + font.width("XYZ") + 5, textY, c.text, false);
    }

    /** Day Counter module content. */
    private void dayContent(GuiGraphicsExtractor g, HudConfig.Settings c, Minecraft mc) {
        Font font = font(mc);
        String value = dayValue(mc);
        int width = textWidth(font, "DAY", value);
        int height = textHeight(font, 1);

        panel(g, width, height, c);
        int textY = PAD + font.lineHeight / 2 + 1;
        g.text(font, "DAY", PAD, textY, c.accent, false);
        g.text(font, value, PAD + font.width("DAY") + 5, textY, c.text, false);
    }

    /** IP Information module content. */
    private void ipContent(GuiGraphicsExtractor g, HudConfig.Settings c, Minecraft mc) {
        Font font = font(mc);
        String value = ipValue(mc);
        int width = textWidth(font, "IP", value);
        int height = textHeight(font, 1);

        panel(g, width, height, c);
        int textY = PAD + font.lineHeight / 2 + 1;
        g.text(font, "IP", PAD, textY, c.accent, false);
        g.text(font, value, PAD + font.width("IP") + 5, textY, c.text, false);
    }

    /** Armor HUD module content. */
    private void armorContent(GuiGraphicsExtractor g, HudConfig.Settings c, Minecraft mc) {
        if (mc == null || mc.player == null) {
            // Preview mode: show representative armor with mock durability
            drawArmorPreview(g, c, true);
            return;
        }
        Font font = font(mc);
        int cell = 16;
        int pad = 4;
        int slots = 4;
        int GAP_SMALL = 2; // small gap between icon and durability text

        panel(g, pad + slots * cell + (slots - 1) * GAP + pad, pad + cell + GAP_SMALL + 8 + pad, c);

        net.minecraft.world.item.ItemStack helmet = mc.player.getItemBySlot(EquipmentSlot.HEAD);
        net.minecraft.world.item.ItemStack chestplate = mc.player.getItemBySlot(EquipmentSlot.CHEST);
        net.minecraft.world.item.ItemStack leggings = mc.player.getItemBySlot(EquipmentSlot.LEGS);
        net.minecraft.world.item.ItemStack boots = mc.player.getItemBySlot(EquipmentSlot.FEET);

        net.minecraft.world.item.ItemStack[] armor = {helmet, chestplate, leggings, boots};
        String[] labels = {"H", "C", "L", "B"};

        // Durability text sits below the icons; reserve space for it in the panel.
        int durHeight = Math.max(1, Math.round(font.lineHeight * DURABILITY_SCALE));
        panel(g, pad + slots * cell + (slots - 1) * GAP + pad, pad + cell + DURABILITY_GAP + durHeight + pad, c);

        for (int i = 0; i < slots; i++) {
            int x = pad + i * (cell + GAP);
            int y = pad;
            net.minecraft.world.item.ItemStack stack = armor[i];
            if (!stack.isEmpty()) {
                // Draw item icon
                g.item(stack, x, y);
                // Draw durability text BELOW the icon: small, centered, elegant.
                if (stack.isDamageableItem()) {
                    int maxDamage = stack.getMaxDamage();
                    int damage = stack.getDamageValue();
                    int remaining = maxDamage - damage;
                    int percent = (int) ((remaining / (float) maxDamage) * 100);
                    drawDurabilityText(g, font, x, y + cell, cell, percent, c);
                }
            } else {
                // Empty slot indicator - centered in the cell
                g.text(font, labels[i], x + (cell - font.width(labels[i])) / 2, y + (cell - font.lineHeight) / 2 + 1, 0xFF555555, false);
            }
        }
    }

    /**
     * Draws an armor durability percentage below an icon: small, centered, and elegant.
     * <p>Used identically by the live in-game HUD and the layout editor preview so both
     * always match.</p>
     */
    private static void drawDurabilityText(
            GuiGraphicsExtractor g,
            Font font,
            int iconX,
            int iconBottom,
            int cell,
            int percent,
            HudConfig.Settings c
    ) {
        // Healthy armor is subtle; low durability is harder to miss.
        int durColor = percent > 60 ? withAlpha(c.text, 0.75f) : (percent > 25 ? c.accent : 0xFFFF5555);
        String durText = percent + "%";
        int scaledWidth = Math.round(font.width(durText) * DURABILITY_SCALE);
        int textX = iconX + Math.max(0, (cell - scaledWidth) / 2);
        int textY = iconBottom + DURABILITY_GAP;
        g.pose().pushMatrix();
        g.pose().translate(textX, textY);
        g.pose().scale(DURABILITY_SCALE, DURABILITY_SCALE);
        g.text(font, durText, 0, 0, durColor, false);
        g.pose().popMatrix();
    }

    /** Draws a representative armor preview for the layout editor. */
    private void drawArmorPreview(GuiGraphicsExtractor g, HudConfig.Settings c, boolean preview) {
        Font font = font(null);
        int cell = 16;
        int pad = 4;
        int slots = 4;
        int GAP_SMALL = DURABILITY_GAP;

        panel(g, pad + slots * cell + (slots - 1) * GAP + pad, pad + cell + GAP_SMALL + Math.max(1, Math.round(font.lineHeight * DURABILITY_SCALE)) + pad, c);

        // Mock armor items with varying durability for preview
        int[] mockDurability = {78, 45, 92, 23}; // helmet, chest, legs, boots
        String[] labels = {"H", "C", "L", "B"};

        for (int i = 0; i < slots; i++) {
            int x = pad + i * (cell + GAP);
            int y = pad;
            // Draw mock item icon (colored rectangle representing armor piece)
            int iconColor = new int[]{0xFF4A90D9, 0xFFD9A04A, 0xFFD94A4A, 0xFF4AD97A}[i];
            g.fill(x, y, x + cell, y + cell, iconColor);
            g.fill(x + 1, y + 1, x + cell - 1, y + cell - 1, 0xFF1A1B1E); // inner dark
            
            // Draw durability text below - identical styling to the live HUD.
            drawDurabilityText(g, font, x, y + cell, cell, mockDurability[i], c);
        }
    }

    /** Keystrokes module content, drawn at the current pose origin. */
    private void keystrokesContent(GuiGraphicsExtractor g, HudConfig.Settings c, Minecraft mc) {
        Font font = font(mc);
        int cell = 20;
        int row = cell + GAP;
        int gridWidth = cell * 3 + GAP * 2;
        int pad = 5;
        int contentX = pad + 2;
        int contentY = pad;
        int width = contentX + gridWidth + pad;
        int height = contentY + row * 4 - GAP + pad;

        boolean w = (mc != null) && mc.options.keyUp.isDown();
        boolean a = (mc != null) && mc.options.keyLeft.isDown();
        boolean s = (mc != null) && mc.options.keyDown.isDown();
        boolean d = (mc != null) && mc.options.keyRight.isDown();
        boolean shift = (mc != null) && mc.options.keyShift.isDown();
        boolean space = (mc != null) && mc.options.keyJump.isDown();
        boolean lmb = (mc != null) && mc.mouseHandler.isLeftPressed();
        boolean rmb = (mc != null) && mc.mouseHandler.isRightPressed();

        panel(g, width, height, c);

        int top = contentY;
        key(g, font, contentX + row, top, cell, cell, "W", w, c);
        int middle = top + row;
        key(g, font, contentX, middle, cell, cell, "A", a, c);
        key(g, font, contentX + row, middle, cell, cell, "S", s, c);
        key(g, font, contentX + row * 2, middle, cell, cell, "D", d, c);

        int lower = middle + row;
        int half = (gridWidth - GAP) / 2;
        key(g, font, contentX, lower, half, cell, "LMB", lmb, c);
        key(g, font, contentX + half + GAP, lower, half, cell, "RMB", rmb, c);

        int bottom = lower + row;
        key(g, font, contentX, bottom, half, cell, "SHIFT", shift, c);
        key(g, font, contentX + half + GAP, bottom, half, cell, "SPACE", space, c);
    }

    private static String fpsValue(Minecraft mc) {
        return (mc != null) ? Integer.toString(mc.getFps()) : "144";
    }

    private String cpsLeft(Minecraft mc) {
        return (mc != null) ? Integer.toString(currentClicks(true)) : "12";
    }

    private String cpsRight(Minecraft mc) {
        return (mc != null) ? Integer.toString(currentClicks(false)) : "8";
    }

    private static String coordsValue(Minecraft mc) {
        if (mc != null && mc.player != null) {
            return mc.player.getBlockX() + " " + mc.player.getBlockY() + " " + mc.player.getBlockZ();
        }
        return "100 64 -200";
    }

    private String dayValue(Minecraft mc) {
        if (mc != null && mc.player != null && mc.player.level() != null) {
            long dayTime = mc.player.level().getOverworldClockTime();
            long days = dayTime / 24000L;
            return Long.toString(days);
        }
        return "0";
    }

    private String ipValue(Minecraft mc) {
        if (mc != null && mc.getSingleplayerServer() == null) {
            // Multiplayer
            if (mc.getConnection() != null && mc.getConnection().getConnection() != null) {
                return mc.getConnection().getConnection().getRemoteAddress().toString();
            }
        } else if (mc != null && mc.getSingleplayerServer() != null) {
            return "Singleplayer";
        }
        return "Disconnected";
    }

    private int currentClicks(boolean left) {
        // Preview uses simulated counts; the real HUD tracks click timestamps.
        return left ? leftClicks.size() : rightClicks.size();
    }

    private static Font font(Minecraft mc) {
        return (mc != null) ? mc.font : Minecraft.getInstance().font;
    }

    private static void key(
            GuiGraphicsExtractor g,
            Font font,
            int x,
            int y,
            int width,
            int height,
            String label,
            boolean down,
            HudConfig.Settings c
    ) {
        int fill = down ? c.accent : withAlpha(c.text, 0.12f);
        int labelColor = down ? 0xFF000000 : c.text; // black text when pressed, normal text color otherwise
        g.fill(x, y, x + width, y + height, fill);
        // No outline - Minecraft style
        g.text(
                font,
                label,
                x + (width - font.width(label)) / 2,
                y + (height - font.lineHeight) / 2 + 1,
                labelColor,
                false
        );
    }

    private static void panel(GuiGraphicsExtractor g, int width, int height, HudConfig.Settings c) {
        int alpha = Math.round(c.background * 255.0f) << 24;
        int color = alpha | PANEL;
        int radius = Math.min(c.cornerRadius, Math.min(width, height) / 2);

        if (radius <= 0) {
            // No rounding - simple fill
            g.fill(0, 0, width, height, color);
        } else {
            // Draw rounded rectangle using 9-slice approach
            // Center
            g.fill(radius, radius, width - radius, height - radius, color);
            // Top edge
            g.fill(radius, 0, width - radius, radius, color);
            // Bottom edge
            g.fill(radius, height - radius, width - radius, height, color);
            // Left edge
            g.fill(0, radius, radius, height - radius, color);
            // Right edge
            g.fill(width - radius, radius, width, height - radius, color);
            // Corners are left empty to create rounded effect
        }
        // No accent bar, no outline - pure Minecraft style
    }

    private static int textWidth(Font font, String label, String value) {
        return PAD + font.width(label) + 5 + font.width(value) + PAD;
    }

    private static int textHeight(Font font, int lines) {
        return PAD + font.lineHeight * lines + GAP * (lines - 1) + PAD;
    }

    private void updateClicks(Minecraft mc) {
        boolean inGame = mc.gui != null && mc.gui.screen() == null;
        boolean left = inGame && mc.mouseHandler.isLeftPressed();
        boolean right = inGame && mc.mouseHandler.isRightPressed();
        long now = System.currentTimeMillis();
        if (left && !leftWasDown) {
            leftClicks.add(now);
        }
        if (right && !rightWasDown) {
            rightClicks.add(now);
        }
        leftWasDown = left;
        rightWasDown = right;
        prune(leftClicks, now - 1000L);
        prune(rightClicks, now - 1000L);
    }

    private static void prune(ArrayDeque<Long> clicks, long cutoff) {
        while (!clicks.isEmpty() && clicks.peekFirst() < cutoff) {
            clicks.removeFirst();
        }
    }

    private static int withAlpha(int color, float alpha) {
        return (Math.round(alpha * 255.0f) << 24) | (color & 0x00FFFFFF);
    }
}
