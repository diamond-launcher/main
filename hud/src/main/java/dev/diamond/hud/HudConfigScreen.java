package dev.diamond.hud;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.EditBox;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.input.KeyEvent;
import net.minecraft.client.input.MouseButtonEvent;
import net.minecraft.network.chat.Component;

import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.function.BooleanSupplier;
import java.util.function.Consumer;
import java.util.function.DoubleConsumer;
import java.util.function.IntConsumer;

/**
 * Simple settings menu for Diamond HUD.
 *
 * <p>A text-only navigation rail and a clean list of rows keep the screen easy
 * to read. Editors are small vanilla-widget screens opened on top.</p>
 */
public final class HudConfigScreen extends Screen {

    private static final int ACCENT = 0xFF6EE7F0;
    private static final int TEXT = 0xFFEDEEF0;
    private static final int TEXT_MUTED = 0xFF9A9DA4;
    private static final int TEXT_FAINT = 0xFF62666D;
    private static final int GREEN = 0xFF1F9D63;
    private static final int RED = 0xFFB23A4B;
    private static final int HEADER_BG = 0xFF1A1B1E;
    private static final int RAIL_BG = 0xFF17181B;
    private static final int CARD_BG = 0xFF232427;
    private static final int CARD_HOVER = 0xFF2A2C30;
    private static final int BORDER = 0xFF2C2F34;
    private static final int CARD_BORDER = 0xFF33363B;
    private static final int ROW_H = 34;
    private static final int ROW_GAP = 6;
    private static final int SWITCH_W = 32;
    private static final int SWITCH_H = 16;
    private static final String VERSION = "1.1.0";

    private final Screen parent;
    private final HudConfig config;
    private HudConfig.Settings settings;
    private final HudRenderer previewRenderer;

    private Tab tab = Tab.MODULES;
    private double scroll = 0;
    private String draggingModule = null;
    private int dragOffsetX = 0;
    private int dragOffsetY = 0;

    public HudConfigScreen(Screen parent, Path configPath) {
        super(Component.literal("Diamond Client"));
        this.parent = parent;
        this.config = new HudConfig(configPath);
        this.config.refresh();
        this.settings = config.get();
        this.previewRenderer = new HudRenderer(configPath);
    }

    @Override
    protected void init() {
        clearWidgets();
        draggingModule = null;
    }

    // ------------------------------------------------------------------
    // Rendering
    // ------------------------------------------------------------------

    @Override
    public void extractBackground(GuiGraphicsExtractor g, int mouseX, int mouseY, float partialTick) {
        int w = g.guiWidth();
        int h = g.guiHeight();
        g.fillGradient(0, 0, w, h, 0xFF141518, 0xFF0E0F11);
    }

    @Override
    public void extractRenderState(GuiGraphicsExtractor g, int mouseX, int mouseY, float partialTick) {
        drawShell(g, mouseX, mouseY);
        if (tab == Tab.LAYOUT) {
            drawLayout(g, mouseX, mouseY);
        } else {
            drawList(g, mouseX, mouseY);
        }
        super.extractRenderState(g, mouseX, mouseY, partialTick);
    }

    private void drawShell(GuiGraphicsExtractor g, int mouseX, int mouseY) {
        int w = g.guiWidth();
        int h = g.guiHeight();

        g.fill(0, 0, w, headerH(), HEADER_BG);
        g.fill(0, headerH() - 1, w, headerH(), BORDER);
        g.fill(0, headerH(), railW(), h, RAIL_BG);
        g.fill(railW() - 1, headerH(), railW(), h, BORDER);

        drawBoldText(g, "Diamond Client", 20, headerH() / 2 - 4, TEXT);

        Rect close = closeRect();
        boolean closeHover = close.contains(mouseX, mouseY);
        if (closeHover) {
            g.fill(close.x, close.y, close.x + close.w, close.y + close.h, 0xFF3A1E22);
        }
        int closeColor = closeHover ? 0xFFFF7A7A : TEXT_MUTED;
        for (int i = 0; i < 8; i++) {
            g.fill(close.x + 8 + i, close.y + 8 + i, close.x + 9 + i, close.y + 9 + i, closeColor);
            g.fill(close.x + 15 - i, close.y + 8 + i, close.x + 16 - i, close.y + 9 + i, closeColor);
        }

        List<NavItem> nav = navItems();
        for (NavItem item : nav) {
            Rect r = item.rect;
            boolean active = tab == item.tab;
            boolean hover = r.contains(mouseX, mouseY);
            if (active) {
                g.fill(r.x, r.y, r.x + r.w, r.y + r.h, 0xFF24262B);
                g.fill(r.x, r.y, r.x + 2, r.y + r.h, ACCENT);
            } else if (hover) {
                g.fill(r.x, r.y, r.x + r.w, r.y + r.h, 0xFF1F2124);
            }
            int color = active ? ACCENT : hover ? TEXT : TEXT_MUTED;
            g.text(font, item.tab.label, r.x + 14,
                    r.y + (r.h - font.lineHeight) / 2 + 1, color, false);
        }

        Rect reset = railResetRect();
        boolean resetHover = reset.contains(mouseX, mouseY);
        g.fill(reset.x, reset.y, reset.x + reset.w, reset.y + reset.h,
                resetHover ? 0xFF2B2D31 : 0xFF1F2124);
        box(g, reset.x, reset.y, reset.w, reset.h, CARD_BORDER);
        g.centeredText(font, "RESET", reset.x + reset.w / 2,
                reset.y + (reset.h - font.lineHeight) / 2 + 1, resetHover ? ACCENT : TEXT_MUTED);

        int statusY = h - 74;
        int dotColor = settings.enabled ? GREEN : RED;
        int railCenter = railW() / 2;
        g.fill(railCenter - 20, statusY, railCenter - 16, statusY + 4, dotColor);
        g.text(font, settings.enabled ? "HUD ON" : "HUD OFF", railCenter - 12, statusY - 2, TEXT_MUTED, false);
        g.centeredText(font, "v" + VERSION, railCenter, h - 18, TEXT_FAINT);
    }

    private void drawContentHeader(GuiGraphicsExtractor g, String title) {
        drawBoldText(g, title, contentX(), headerH() + 16, TEXT);
    }

    private void drawList(GuiGraphicsExtractor g, int mouseX, int mouseY) {
        List<Row> rows = rows();
        drawContentHeader(g, tabTitle());

        int x = contentX();
        int w = contentW();
        int top = contentTop();
        int bottom = height - 18;
        int visible = Math.max(1, bottom - top);
        int total = rows.size() * ROW_H + Math.max(0, rows.size() - 1) * ROW_GAP;
        int maxScroll = Math.max(0, total - visible);
        scroll = clamp(scroll, 0, maxScroll);

        g.enableScissor(x, top, x + w, bottom);
        for (int i = 0; i < rows.size(); i++) {
            int y = top + i * (ROW_H + ROW_GAP) - (int) scroll;
            if (y + ROW_H < top || y > bottom) {
                continue;
            }
            drawRow(g, rows.get(i), x, y, w, mouseX, mouseY);
        }
        g.disableScissor();

        if (maxScroll > 0) {
            int sx = width - 9;
            g.fill(sx, top, sx + 2, bottom, 0xFF2A2C30);
            int thumbH = Math.max(18, (int) Math.round(visible * (visible / (double) (visible + maxScroll))));
            int thumbY = top + (int) Math.round((visible - thumbH) * (scroll / (double) maxScroll));
            g.fill(sx, thumbY, sx + 2, thumbY + thumbH, ACCENT);
        }
    }

    private void drawRow(GuiGraphicsExtractor g, Row row, int x, int y, int w, int mouseX, int mouseY) {
        boolean hover = mouseX >= x && mouseX < x + w && mouseY >= y && mouseY < y + ROW_H;
        g.fill(x, y, x + w, y + ROW_H, hover ? CARD_HOVER : CARD_BG);
        box(g, x, y, w, ROW_H, hover ? 0xFF4A4E55 : CARD_BORDER);

        int textY = y + (ROW_H - font.lineHeight) / 2 + 1;
        g.text(font, fit(row.label, w - 150), x + 14, textY, TEXT, false);

        if (row.onToggle != null) {
            drawSwitch(g, x + w - 14 - SWITCH_W, y + (ROW_H - SWITCH_H) / 2, row.on);
        } else if (row.value != null) {
            String value = fit(row.value, w / 2);
            g.text(font, value, x + w - 14 - font.width(value), textY, row.valueColor, false);
        }
    }

    private void drawSwitch(GuiGraphicsExtractor g, int x, int y, boolean on) {
        g.fill(x, y, x + SWITCH_W, y + SWITCH_H, on ? ACCENT : 0xFF3A3D42);
        box(g, x, y, SWITCH_W, SWITCH_H, on ? ACCENT : 0xFF4A4E55);
        int knob = SWITCH_H - 4;
        int knobX = on ? x + SWITCH_W - knob - 2 : x + 2;
        g.fill(knobX, y + 2, knobX + knob, y + 2 + knob, 0xFFE8EEF0);
    }

    private void drawLayout(GuiGraphicsExtractor g, int mouseX, int mouseY) {
        drawContentHeader(g, "Layout");

        Preview p = layoutPreview();
        g.fill(p.x - 2, p.y - 2, p.x + p.w + 2, p.y + p.h + 2, BORDER);
        g.fill(p.x, p.y, p.x + p.w, p.y + p.h, 0xFF0B0F10);
        for (int i = 1; i < 8; i++) {
            int gx = p.x + i * p.w / 8;
            g.fill(gx, p.y, gx + 1, p.y + p.h, 0x20FFFFFF);
        }
        for (int i = 1; i < 4; i++) {
            int gy = p.y + i * p.h / 4;
            g.fill(p.x, gy, p.x + p.w, gy + 1, 0x20FFFFFF);
        }

        if (settings.enabled) {
            for (String type : HudRenderer.TYPES) {
                HudConfig.Module mod = moduleByKey(type);
                if (!mod.enabled) {
                    continue;
                }
                int[] size = previewRenderer.moduleSize(type, settings, mod, null, p.scale);
                int mx = p.x + (int) Math.round(mod.x * p.w);
                int my = p.y + (int) Math.round(mod.y * p.h);
                boolean hovered = mouseX >= mx && mouseX <= mx + size[0] && mouseY >= my && mouseY <= my + size[1];
                if (type.equals(draggingModule)) {
                    outline(g, mx, my, size[0], size[1], ACCENT);
                } else if (hovered) {
                    outline(g, mx, my, size[0], size[1], 0x806EE7F0);
                }
                previewRenderer.drawModuleAt(g, type, settings, mod, null, mx, my, p.scale);
            }
        } else {
            g.centeredText(font, "HUD is disabled", p.x + p.w / 2, p.y + p.h / 2 - 4, TEXT_MUTED);
        }

        g.text(font, "Drag modules to move them.", contentX(), p.y + p.h + 8, TEXT_FAINT, false);

        Rect reset = layoutResetRect();
        boolean hover = reset.contains(mouseX, mouseY);
        g.fill(reset.x, reset.y, reset.x + reset.w, reset.y + reset.h,
                hover ? 0xFF2B2D31 : 0xFF1F2124);
        box(g, reset.x, reset.y, reset.w, reset.h, CARD_BORDER);
        g.centeredText(font, "RESET POSITIONS", reset.x + reset.w / 2,
                reset.y + (reset.h - font.lineHeight) / 2 + 1, hover ? ACCENT : TEXT_MUTED);
    }

    // ------------------------------------------------------------------
    // Row data
    // ------------------------------------------------------------------

    private List<Row> rows() {
        List<Row> list = new ArrayList<>();
        switch (tab) {
            case MODULES -> {
                addModuleRow(list, "fps", "FPS", settings.fps);
                addModuleRow(list, "cps", "CPS", settings.cps);
                addModuleRow(list, "keystrokes", "Keystrokes", settings.keystrokes);
                addModuleRow(list, "coords", "Coordinates", settings.coords);
                addModuleRow(list, "day", "Day Counter", settings.day);
                addModuleRow(list, "ip", "IP Info", settings.ip);
                addModuleRow(list, "armor", "Armor HUD", settings.armor);
                addModuleRow(list, "fullbright", "Fullbright", settings.fullbright);
            }
            case APPEARANCE -> {
                list.add(Row.value("scale", "Scale", fmt("%.2fx", settings.scale),
                        () -> openNumber("Scale", "Resize every HUD module.", settings.scale,
                                0.5, 2.0, 0.05, "%.2f", v -> settings.scale = (float) v, 1.0)));
                list.add(Row.value("opacity", "Panel Opacity",
                        Math.round(settings.background * 100) + "%",
                        () -> openNumber("Panel Opacity", "Background opacity of the HUD panels.",
                                settings.background * 100.0, 0, 100, 1, "%.0f",
                                v -> settings.background = (float) (v / 100.0), 55.0)));
                list.add(Row.value("accent", "Accent Color", hex(settings.accent),
                        () -> openColor("Accent Color", "Highlight color used by labels and active keys.",
                                settings.accent, c -> settings.accent = c, 0xFF6EE7F0)));
                list.add(Row.value("text", "Text Color", hex(settings.text),
                        () -> openColor("Text Color", "Color of the HUD values.",
                                settings.text, c -> settings.text = c, 0xFFE8F0F2)));
                list.add(Row.value("radius", "Corner Radius", settings.cornerRadius + " px",
                        () -> openNumber("Corner Radius", "Round the corners of the HUD panels.",
                                settings.cornerRadius, 0, 20, 1, "%.0f",
                                v -> settings.cornerRadius = (int) Math.round(v), 4)));
            }
            case GENERAL -> {
                list.add(Row.toggle("hud", "HUD Overlay", settings.enabled,
                        () -> {
                            settings.enabled = !settings.enabled;
                            save();
                        },
                        () -> openToggle("HUD Overlay", "Show or hide every HUD module.",
                                () -> settings.enabled, v -> settings.enabled = v)));
                list.add(Row.value("keybind", "Config Key", getKeyName(settings.keybind),
                        this::openKeybind));
            }
            case LAYOUT -> {
            }
        }
        return list;
    }

    private void addModuleRow(List<Row> list, String id, String name, HudConfig.Module mod) {
        list.add(Row.toggle(id, name, mod.enabled,
                () -> {
                    mod.enabled = !mod.enabled;
                    save();
                },
                () -> openModuleEditor(id, name, mod)));
    }

    // ------------------------------------------------------------------
    // Geometry
    // ------------------------------------------------------------------

    private boolean compact() {
        return width < 560;
    }

    private int headerH() {
        return compact() ? 36 : 44;
    }

    private int railW() {
        return width < 420 ? 84 : compact() ? 100 : 128;
    }

    private int navH() {
        return compact() ? 28 : 32;
    }

    private int contentX() {
        return railW() + 20;
    }

    private int contentW() {
        return Math.max(140, width - contentX() - 20);
    }

    private int contentTop() {
        return headerH() + 40;
    }

    private Rect closeRect() {
        return new Rect(width - 32, (headerH() - 22) / 2, 22, 22);
    }

    private Rect navRect(int index) {
        int x = 8;
        int y = headerH() + 16 + index * (navH() + 4);
        return new Rect(x, y, railW() - 16, navH());
    }

    private List<NavItem> navItems() {
        List<NavItem> items = new ArrayList<>();
        Tab[] tabs = Tab.values();
        for (int i = 0; i < tabs.length; i++) {
            items.add(new NavItem(tabs[i], navRect(i)));
        }
        return items;
    }

    private Rect railResetRect() {
        return new Rect(8, height - 58, railW() - 16, 24);
    }

    private Rect layoutResetRect() {
        return new Rect(contentX(), height - 42, 150, 24);
    }

    private Preview layoutPreview() {
        int cx = contentX();
        int cw = contentW();
        int top = contentTop();
        int bottom = height - 70;
        int availW = Math.max(40, cw - 10);
        int availH = Math.max(40, bottom - top - 6);
        float scale = Math.min(availW / (float) Math.max(1, width), availH / (float) Math.max(1, height));
        scale = Math.max(0.15f, scale);
        int pw = Math.round(width * scale);
        int ph = Math.round(height * scale);
        int px = cx + (cw - pw) / 2;
        int py = top + (bottom - top - ph) / 2;
        return new Preview(px, py, pw, ph, scale);
    }

    private String tabTitle() {
        return switch (tab) {
            case MODULES -> "Modules";
            case APPEARANCE -> "Appearance";
            case GENERAL -> "General";
            case LAYOUT -> "Layout";
        };
    }

    // ------------------------------------------------------------------
    // Records and enums
    // ------------------------------------------------------------------

    private record Row(String id, String label, String value, int valueColor, Boolean on,
                       Runnable onToggle, Runnable onOpen) {
        static Row value(String id, String label, String value, Runnable onOpen) {
            return new Row(id, label, value, TEXT_MUTED, null, null, onOpen);
        }

        static Row toggle(String id, String label, boolean on, Runnable onToggle, Runnable onOpen) {
            return new Row(id, label, null, 0, on, onToggle, onOpen);
        }
    }

    private record Rect(int x, int y, int w, int h) {
        boolean contains(double mx, double my) {
            return mx >= x && mx < x + w && my >= y && my < y + h;
        }
    }

    private record Preview(int x, int y, int w, int h, float scale) {
    }

    private record NavItem(Tab tab, Rect rect) {
    }

    private enum Tab {
        MODULES("Modules"),
        APPEARANCE("Appearance"),
        GENERAL("General"),
        LAYOUT("Layout");

        final String label;

        Tab(String label) {
            this.label = label;
        }
    }

    // ------------------------------------------------------------------
    // Input
    // ------------------------------------------------------------------

    @Override
    public boolean mouseClicked(MouseButtonEvent event, boolean bl) {
        double mx = event.x();
        double my = event.y();
        if (event.button() == 0) {
            if (closeRect().contains(mx, my)) {
                onClose();
                return true;
            }
            for (NavItem item : navItems()) {
                if (item.rect.contains(mx, my)) {
                    tab = item.tab;
                    scroll = 0;
                    draggingModule = null;
                    return true;
                }
            }
            if (railResetRect().contains(mx, my)) {
                resetPositions();
                return true;
            }

            if (tab == Tab.LAYOUT) {
                if (layoutResetRect().contains(mx, my)) {
                    resetPositions();
                    return true;
                }
                if (settings.enabled && tryStartDrag(mx, my)) {
                    return true;
                }
            } else {
                List<Row> rows = rows();
                int x = contentX();
                int w = contentW();
                int top = contentTop();
                int bottom = height - 18;
                if (mx >= x && mx < x + w && my >= top && my < bottom) {
                    for (int i = 0; i < rows.size(); i++) {
                        int y = top + i * (ROW_H + ROW_GAP) - (int) scroll;
                        if (my < y || my >= y + ROW_H) {
                            continue;
                        }
                        Row row = rows.get(i);
                        if (row.onToggle != null) {
                            int switchX = x + w - 14 - SWITCH_W;
                            if (mx >= switchX && mx < switchX + SWITCH_W) {
                                row.onToggle.run();
                                return true;
                            }
                        }
                        if (row.onOpen != null) {
                            row.onOpen.run();
                        }
                        return true;
                    }
                }
            }
        }
        return super.mouseClicked(event, bl);
    }

    private boolean tryStartDrag(double mx, double my) {
        Preview p = layoutPreview();
        for (String type : HudRenderer.TYPES) {
            HudConfig.Module mod = moduleByKey(type);
            if (!mod.enabled) {
                continue;
            }
            int[] size = previewRenderer.moduleSize(type, settings, mod, null, p.scale);
            int x = p.x + (int) Math.round(mod.x * p.w);
            int y = p.y + (int) Math.round(mod.y * p.h);
            if (mx >= x && mx <= x + size[0] && my >= y && my <= y + size[1]) {
                draggingModule = type;
                dragOffsetX = (int) mx - x;
                dragOffsetY = (int) my - y;
                return true;
            }
        }
        return false;
    }

    @Override
    public boolean mouseDragged(MouseButtonEvent event, double dragX, double dragY) {
        if (event.button() == 0 && draggingModule != null && tab == Tab.LAYOUT) {
            Preview p = layoutPreview();
            HudConfig.Module mod = moduleByKey(draggingModule);
            int[] size = previewRenderer.moduleSize(draggingModule, settings, mod, null, p.scale);
            int newX = (int) event.x() - dragOffsetX - p.x;
            int newY = (int) event.y() - dragOffsetY - p.y;
            double maxX = Math.max(0, (p.w - size[0]) / (double) p.w);
            double maxY = Math.max(0, (p.h - size[1]) / (double) p.h);
            mod.x = Math.max(0, Math.min(maxX, newX / (double) p.w));
            mod.y = Math.max(0, Math.min(maxY, newY / (double) p.h));
            save();
            return true;
        }
        return super.mouseDragged(event, dragX, dragY);
    }

    @Override
    public boolean mouseReleased(MouseButtonEvent event) {
        if (event.button() == 0) {
            draggingModule = null;
        }
        return super.mouseReleased(event);
    }

    @Override
    public boolean mouseScrolled(double mouseX, double mouseY, double scrollX, double scrollY) {
        if (tab != Tab.LAYOUT && scrollY != 0) {
            List<Row> rows = rows();
            int visible = Math.max(1, (height - 18) - contentTop());
            int total = rows.size() * ROW_H + Math.max(0, rows.size() - 1) * ROW_GAP;
            int maxScroll = Math.max(0, total - visible);
            if (mouseX >= contentX() && mouseX < contentX() + contentW()
                    && mouseY >= contentTop() && mouseY < height - 18) {
                scroll = clamp(scroll - scrollY * 28.0, 0, maxScroll);
                return true;
            }
        }
        return super.mouseScrolled(mouseX, mouseY, scrollX, scrollY);
    }

    // ------------------------------------------------------------------
    // Actions
    // ------------------------------------------------------------------

    private void save() {
        config.write(settings);
    }

    private void resetPositions() {
        settings.fps = new HudConfig.Module(settings.fps.enabled, 0.015, 0.015);
        settings.cps = new HudConfig.Module(settings.cps.enabled, 0.015, 0.075);
        settings.keystrokes = new HudConfig.Module(settings.keystrokes.enabled, 0.015, 0.150);
        settings.coords = new HudConfig.Module(settings.coords.enabled, 0.015, 0.850);
        settings.day = new HudConfig.Module(settings.day.enabled, 0.015, 0.225);
        settings.ip = new HudConfig.Module(settings.ip.enabled, 0.015, 0.300);
        settings.armor = new HudConfig.Module(settings.armor.enabled, 0.850, 0.015);
        settings.fullbright = new HudConfig.Module(settings.fullbright.enabled, 0.850, 0.075);
        save();
    }

    private void openNumber(String title, String hint, double current, double min, double max,
                            double step, String format, DoubleConsumer setter, double defaultValue) {
        Minecraft.getInstance().gui.setScreen(
                new NumberEditor(title, hint, current, min, max, step, format, setter, defaultValue));
    }

    private void openColor(String title, String hint, int current, IntConsumer setter, int defaultValue) {
        Minecraft.getInstance().gui.setScreen(new ColorEditor(title, hint, current, setter, defaultValue));
    }

    private void openToggle(String title, String hint, BooleanSupplier getter, Consumer<Boolean> setter) {
        Minecraft.getInstance().gui.setScreen(new ToggleEditor(title, hint, getter, setter));
    }

    private void openKeybind() {
        Minecraft.getInstance().gui.setScreen(new KeybindEditor());
    }

    private void openModuleEditor(String key, String name, HudConfig.Module module) {
        Minecraft.getInstance().gui.setScreen(new ModuleEditor(key, name, module));
    }

    private HudConfig.Module moduleByKey(String key) {
        return switch (key) {
            case "fps" -> settings.fps;
            case "cps" -> settings.cps;
            case "keystrokes" -> settings.keystrokes;
            case "coords" -> settings.coords;
            case "day" -> settings.day;
            case "ip" -> settings.ip;
            case "armor" -> settings.armor;
            case "fullbright" -> settings.fullbright;
            default -> settings.fps;
        };
    }

    @Override
    public void onClose() {
        Minecraft.getInstance().gui.setScreen(parent);
    }

    @Override
    public boolean isPauseScreen() {
        return true;
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    private static boolean isBindable(int key) {
        return key != 0 && key != 256 && key != 257 && key != 258 && key != 259
                && key != 260 && key != 261 && key != 340 && key != 341 && key != 342
                && key != 343 && key != 344 && key != 345 && key != 346 && key != 347
                && key != 348;
    }

    private static String getKeyName(int keyCode) {
        return switch (keyCode) {
            case 32 -> "SPACE";
            case 340 -> "LCTRL";
            case 341 -> "LSHIFT";
            case 342 -> "LALT";
            case 343 -> "LSUPER";
            case 344 -> "RCTRL";
            case 345 -> "RSHIFT";
            case 346 -> "RALT";
            case 347 -> "RSUPER";
            case 348 -> "MENU";
            case 256 -> "ESCAPE";
            case 257 -> "ENTER";
            case 258 -> "TAB";
            case 259 -> "BACKSPACE";
            case 260 -> "INSERT";
            case 261 -> "DELETE";
            case 262 -> "RIGHT";
            case 263 -> "LEFT";
            case 264 -> "DOWN";
            case 265 -> "UP";
            case 266 -> "PAGE_UP";
            case 267 -> "PAGE_DOWN";
            case 268 -> "HOME";
            case 269 -> "END";
            case 280 -> "CAPS_LOCK";
            case 281 -> "SCROLL_LOCK";
            case 282 -> "NUM_LOCK";
            case 283 -> "PRINT_SCREEN";
            case 284 -> "PAUSE";
            case 290 -> "F1";
            case 291 -> "F2";
            case 292 -> "F3";
            case 293 -> "F4";
            case 294 -> "F5";
            case 295 -> "F6";
            case 296 -> "F7";
            case 297 -> "F8";
            case 298 -> "F9";
            case 299 -> "F10";
            case 300 -> "F11";
            case 301 -> "F12";
            case 302 -> "F13";
            case 303 -> "F14";
            case 304 -> "F15";
            case 305 -> "F16";
            case 306 -> "F17";
            case 307 -> "F18";
            case 308 -> "F19";
            case 309 -> "F20";
            case 310 -> "F21";
            case 311 -> "F22";
            case 312 -> "F23";
            case 313 -> "F24";
            case 320 -> "KP_0";
            case 321 -> "KP_1";
            case 322 -> "KP_2";
            case 323 -> "KP_3";
            case 324 -> "KP_4";
            case 325 -> "KP_5";
            case 326 -> "KP_6";
            case 327 -> "KP_7";
            case 328 -> "KP_8";
            case 329 -> "KP_9";
            case 330 -> "KP_DECIMAL";
            case 331 -> "KP_DIVIDE";
            case 332 -> "KP_MULTIPLY";
            case 333 -> "KP_SUBTRACT";
            case 334 -> "KP_ADD";
            case 335 -> "KP_ENTER";
            case 336 -> "KP_EQUAL";
            default -> {
                if (keyCode >= 65 && keyCode <= 90) {
                    yield Character.toString((char) keyCode);
                }
                if (keyCode >= 48 && keyCode <= 57) {
                    yield Character.toString((char) keyCode);
                }
                yield "KEY_" + keyCode;
            }
        };
    }

    private String fit(String text, int maxWidth) {
        if (text == null) {
            return "";
        }
        if (maxWidth <= 0) {
            return "";
        }
        if (font.width(text) <= maxWidth) {
            return text;
        }
        String dots = "...";
        int available = Math.max(0, maxWidth - font.width(dots));
        return font.plainSubstrByWidth(text, available) + dots;
    }

    private static double clamp(double value, double min, double max) {
        return Math.max(min, Math.min(max, value));
    }

    private static int clamp(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
    }

    private static String fmt(String format, double value) {
        return String.format(Locale.ROOT, format, value);
    }

    private static String hex(int color) {
        return String.format(Locale.ROOT, "#%06X", color & 0xFFFFFF);
    }

    private static Integer parseHex(String text) {
        String value = text.trim();
        if (value.startsWith("#")) {
            value = value.substring(1);
        }
        if (value.length() != 6) {
            return null;
        }
        try {
            return Integer.parseInt(value, 16) | 0xFF000000;
        } catch (NumberFormatException ignored) {
            return null;
        }
    }

    private void drawBoldText(GuiGraphicsExtractor g, String text, int x, int y, int color) {
        g.text(font, text, x, y, color, false);
        g.text(font, text, x + 1, y, color, false);
    }

    static void roundedFill(GuiGraphicsExtractor g, int x, int y, int width, int height, int radius, int color) {
        g.fill(x, y, x + width, y + height, color);
    }

    static void roundedBorder(GuiGraphicsExtractor g, int x, int y, int width, int height, int radius, int color) {
        box(g, x, y, width, height, color);
    }

    static void box(GuiGraphicsExtractor g, int x, int y, int width, int height, int color) {
        g.fill(x, y, x + width, y + 1, color);
        g.fill(x, y + height - 1, x + width, y + height, color);
        g.fill(x, y + 1, x + 1, y + height - 1, color);
        g.fill(x + width - 1, y + 1, x + width, y + height - 1, color);
    }

    private static void outline(GuiGraphicsExtractor g, int x, int y, int w, int h, int color) {
        g.fill(x - 1, y - 1, x + w + 1, y, color);
        g.fill(x - 1, y + h, x + w + 1, y + h + 1, color);
        g.fill(x - 1, y, x, y + h, color);
        g.fill(x + w, y, x + w + 1, y + h, color);
    }

    // ------------------------------------------------------------------
    // Editors
    // ------------------------------------------------------------------

    private abstract class BaseEditor extends Screen {
        protected final String heading;
        protected final String hint;
        protected int panelX;
        protected int panelY;
        protected int panelW;
        protected int panelH;

        BaseEditor(String heading, String hint) {
            super(Component.literal(heading));
            this.heading = heading;
            this.hint = hint;
        }

        protected abstract int panelHeight();

        protected abstract void buildWidgets();

        @Override
        protected void init() {
            clearWidgets();
            panelW = Math.min(360, width - 32);
            panelH = panelHeight();
            panelX = (width - panelW) / 2;
            panelY = (height - panelH) / 2;
            buildWidgets();
        }

        protected void addDoneCancel(Runnable onDone) {
            int buttonW = (panelW - 52) / 2;
            int buttonY = panelY + panelH - 32;
            addRenderableWidget(new MenuButton(panelX + 20, buttonY, buttonW, 22,
                    Component.literal("Done"), b -> onDone.run(), true));
            addRenderableWidget(new MenuButton(panelX + 32 + buttonW, buttonY, buttonW, 22,
                    Component.literal("Cancel"), b -> onClose(), false));
        }

        @Override
        public void extractBackground(GuiGraphicsExtractor g, int mouseX, int mouseY, float partialTick) {
            g.fillGradient(0, 0, g.guiWidth(), g.guiHeight(), 0xE60A0B0D, 0xF20A0B0D);
        }

        @Override
        public void extractRenderState(GuiGraphicsExtractor g, int mouseX, int mouseY, float partialTick) {
            g.fill(panelX, panelY, panelX + panelW, panelY + panelH, 0xFF1D1F22);
            box(g, panelX, panelY, panelW, panelH, 0xFF3A3D42);
            g.fill(panelX, panelY, panelX + panelW, panelY + 2, ACCENT);
            drawBoldText(g, heading, panelX + 20, panelY + 16, TEXT);
            g.text(font, fit(hint, panelW - 40), panelX + 20, panelY + 32, TEXT_MUTED, false);
            g.fill(panelX + 20, panelY + 46, panelX + panelW - 20, panelY + 47, 0xFF2C2F34);
            super.extractRenderState(g, mouseX, mouseY, partialTick);
        }

        @Override
        public void onClose() {
            Minecraft.getInstance().gui.setScreen(HudConfigScreen.this);
        }

        @Override
        public boolean isPauseScreen() {
            return true;
        }
    }

    private final class NumberEditor extends BaseEditor {
        private final double min;
        private final double max;
        private final double step;
        private final double defaultValue;
        private final String pattern;
        private final DoubleConsumer setter;
        private double value;
        private EditBox input;
        private boolean draggingSlider;
        private int trackX;
        private int trackY;
        private int trackW;

        NumberEditor(String heading, String hint, double current, double min, double max,
                     double step, String pattern, DoubleConsumer setter, double defaultValue) {
            super(heading, hint);
            this.min = min;
            this.max = max;
            this.step = step;
            this.pattern = pattern;
            this.setter = setter;
            this.defaultValue = defaultValue;
            this.value = clamp(current, min, max);
        }

        @Override
        protected int panelHeight() {
            return 208;
        }

        @Override
        protected void buildWidgets() {
            input = new EditBox(font, panelX + 20, panelY + 54, panelW - 40, 20, Component.literal(heading));
            input.setValue(fmt(pattern, value));
            input.setResponder(s -> {
                try {
                    double v = Double.parseDouble(s.trim());
                    if (v >= min && v <= max) {
                        value = v;
                        setter.accept(value);
                        save();
                    }
                } catch (NumberFormatException ignored) {
                }
            });
            addRenderableWidget(input);

            addRenderableWidget(new MenuButton(panelX + 20, panelY + 84, 26, 20,
                    Component.literal("-"), b -> stepBy(-step), false));
            addRenderableWidget(new MenuButton(panelX + panelW - 46, panelY + 84, 26, 20,
                    Component.literal("+"), b -> stepBy(step), false));
            addRenderableWidget(new MenuButton(panelX + 20, panelY + 124, 80, 20,
                    Component.literal("Reset"), b -> setValue(defaultValue), false));

            trackX = panelX + 56;
            trackY = panelY + 92;
            trackW = panelW - 112;

            addDoneCancel(() -> {
                applyInput();
                onClose();
            });
        }

        private void stepBy(double delta) {
            setValue(value + delta);
        }

        private void setValue(double newValue) {
            double v = clamp(newValue, min, max);
            if (step > 0) {
                v = Math.round(v / step) * step;
            }
            value = clamp(v, min, max);
            setter.accept(value);
            save();
            if (input != null) {
                input.setValue(fmt(pattern, value));
            }
        }

        private void applyInput() {
            if (input == null) {
                return;
            }
            try {
                setValue(Double.parseDouble(input.getValue().trim()));
            } catch (NumberFormatException ignored) {
                input.setValue(fmt(pattern, value));
            }
        }

        private void updateFromMouse(double mouseX) {
            double t = clamp((mouseX - trackX) / (double) Math.max(1, trackW), 0, 1);
            setValue(min + t * (max - min));
        }

        @Override
        public void extractRenderState(GuiGraphicsExtractor g, int mouseX, int mouseY, float partialTick) {
            super.extractRenderState(g, mouseX, mouseY, partialTick);
            g.fill(trackX, trackY, trackX + trackW, trackY + 3, 0xFF33363B);
            int fillW = (int) Math.round((value - min) / (max - min) * trackW);
            g.fill(trackX, trackY, trackX + fillW, trackY + 3, ACCENT);
            int knobX = trackX + fillW;
            g.fill(knobX - 3, trackY - 4, knobX + 3, trackY + 7, 0xFFE6EEF0);
            g.text(font, fmt(pattern, min), trackX, trackY + 8, TEXT_FAINT, false);
            String maxText = fmt(pattern, max);
            g.text(font, maxText, trackX + trackW - font.width(maxText), trackY + 8, TEXT_FAINT, false);
        }

        @Override
        public boolean mouseClicked(MouseButtonEvent event, boolean bl) {
            if (event.button() == 0 && event.y() >= trackY - 5 && event.y() <= trackY + 9
                    && event.x() >= trackX && event.x() <= trackX + trackW) {
                draggingSlider = true;
                updateFromMouse(event.x());
                return true;
            }
            return super.mouseClicked(event, bl);
        }

        @Override
        public boolean mouseDragged(MouseButtonEvent event, double dragX, double dragY) {
            if (draggingSlider) {
                updateFromMouse(event.x());
                return true;
            }
            return super.mouseDragged(event, dragX, dragY);
        }

        @Override
        public boolean mouseReleased(MouseButtonEvent event) {
            draggingSlider = false;
            return super.mouseReleased(event);
        }

        @Override
        public boolean keyPressed(KeyEvent event) {
            if (event.key() == 257) {
                applyInput();
                onClose();
                return true;
            }
            return super.keyPressed(event);
        }
    }

    private final class ColorEditor extends BaseEditor {
        private final int defaultValue;
        private final IntConsumer setter;
        private final int[] presets = {
                0xFF6EE7F0, 0xFF22C55E, 0xFFF59E0B, 0xFFEF4444, 0xFFA855F7, 0xFFFFFFFF
        };
        private int value;
        private EditBox input;
        private boolean updating;
        private int swatchX;
        private int swatchY;
        private static final int SWATCH = 24;
        private static final int SWATCH_GAP = 8;

        ColorEditor(String heading, String hint, int current, IntConsumer setter, int defaultValue) {
            super(heading, hint);
            this.setter = setter;
            this.defaultValue = defaultValue;
            this.value = current;
        }

        @Override
        protected int panelHeight() {
            return 248;
        }

        @Override
        protected void buildWidgets() {
            input = new EditBox(font, panelX + 20, panelY + 54, panelW - 40, 20, Component.literal("#RRGGBB"));
            input.setValue(hex(value));
            input.setResponder(s -> {
                if (updating) {
                    return;
                }
                Integer parsed = parseHex(s);
                if (parsed != null) {
                    value = parsed;
                    setter.accept(parsed);
                    save();
                }
            });
            addRenderableWidget(input);

            int total = presets.length * SWATCH + (presets.length - 1) * SWATCH_GAP;
            swatchX = panelX + (panelW - total) / 2;
            swatchY = panelY + 96;

            addRenderableWidget(new MenuButton(panelX + 20, panelY + 164, 80, 20,
                    Component.literal("Reset"), b -> applyColor(defaultValue), false));

            addDoneCancel(() -> {
                applyInput();
                onClose();
            });
        }

        private void applyColor(int color) {
            value = color;
            setter.accept(color);
            save();
            if (input != null) {
                updating = true;
                input.setValue(hex(color));
                updating = false;
            }
        }

        private void applyInput() {
            if (input == null) {
                return;
            }
            Integer parsed = parseHex(input.getValue());
            if (parsed != null) {
                applyColor(parsed);
            } else {
                updating = true;
                input.setValue(hex(value));
                updating = false;
            }
        }

        @Override
        public void extractRenderState(GuiGraphicsExtractor g, int mouseX, int mouseY, float partialTick) {
            super.extractRenderState(g, mouseX, mouseY, partialTick);
            g.text(font, "PRESETS", panelX + 20, panelY + 84, TEXT_FAINT, false);
            for (int i = 0; i < presets.length; i++) {
                int x = swatchX + i * (SWATCH + SWATCH_GAP);
                roundedFill(g, x, swatchY, SWATCH, SWATCH, 4, presets[i]);
                int border = value == presets[i] ? 0xFFFFFFFF : 0xFF33363B;
                roundedBorder(g, x, swatchY, SWATCH, SWATCH, 4, border);
            }

            int previewX = panelX + 20;
            int previewY = panelY + 130;
            int previewW = panelW - 40;
            String hexText = hex(value);
            int hexW = font.width(hexText);
            int barW = Math.max(40, previewW - hexW - 12);
            roundedFill(g, previewX, previewY, barW, 20, 4, value);
            roundedBorder(g, previewX, previewY, barW, 20, 4, 0xFF33363B);
            g.text(font, hexText, previewX + barW + 12, previewY + 6, TEXT_MUTED, false);
        }

        @Override
        public boolean mouseClicked(MouseButtonEvent event, boolean bl) {
            if (event.button() == 0) {
                for (int i = 0; i < presets.length; i++) {
                    int x = swatchX + i * (SWATCH + SWATCH_GAP);
                    if (event.x() >= x && event.x() < x + SWATCH
                            && event.y() >= swatchY && event.y() < swatchY + SWATCH) {
                        applyColor(presets[i]);
                        return true;
                    }
                }
            }
            return super.mouseClicked(event, bl);
        }
    }

    private final class ToggleEditor extends BaseEditor {
        private final BooleanSupplier getter;
        private final Consumer<Boolean> setter;

        ToggleEditor(String heading, String hint, BooleanSupplier getter, Consumer<Boolean> setter) {
            super(heading, hint);
            this.getter = getter;
            this.setter = setter;
        }

        @Override
        protected int panelHeight() {
            return 170;
        }

        @Override
        protected void buildWidgets() {
            addRenderableWidget(new MenuButton(panelX + 20, panelY + 64, panelW - 40, 24,
                    Component.literal(getter.getAsBoolean() ? "ENABLED" : "DISABLED"),
                    b -> {
                        boolean next = !getter.getAsBoolean();
                        setter.accept(next);
                        save();
                        ((MenuButton) b).setPrimary(next);
                        b.setMessage(Component.literal(next ? "ENABLED" : "DISABLED"));
                    }, true));
            addDoneCancel(this::onClose);
        }
    }

    private final class KeybindEditor extends BaseEditor {
        private boolean listening;
        private MenuButton listenButton;

        KeybindEditor() {
            super("Config Key", "Press the key that opens the Diamond HUD menu.");
        }

        @Override
        protected int panelHeight() {
            return 190;
        }

        @Override
        protected void buildWidgets() {
            listenButton = new MenuButton(panelX + 20, panelY + 64, panelW - 40, 24,
                    Component.literal(listening ? "Press a key..." : "Change Key: " + getKeyName(settings.keybind)),
                    b -> {
                        listening = true;
                        b.setMessage(Component.literal("Press a key..."));
                    }, false);
            addRenderableWidget(listenButton);
            addDoneCancel(this::onClose);
        }

        @Override
        public boolean keyPressed(KeyEvent event) {
            if (listening) {
                int key = event.key();
                if (key == 256) {
                    listening = false;
                    if (listenButton != null) {
                        listenButton.setMessage(Component.literal("Change Key: " + getKeyName(settings.keybind)));
                    }
                    return true;
                }
                if (isBindable(key)) {
                    settings.keybind = key;
                    save();
                    DiamondHud.applyKeybind(key);
                    listening = false;
                    onClose();
                    return true;
                }
                return true;
            }
            return super.keyPressed(event);
        }
    }

    private final class ModuleEditor extends BaseEditor {
        private final String key;
        private final HudConfig.Module module;
        private EditBox xBox;
        private EditBox yBox;

        ModuleEditor(String key, String name, HudConfig.Module module) {
            super(name + " Options", "Toggle this module or fine-tune its position.");
            this.key = key;
            this.module = module;
        }

        @Override
        protected int panelHeight() {
            return 250;
        }

        @Override
        protected void buildWidgets() {
            addRenderableWidget(new MenuButton(panelX + 20, panelY + 56, panelW - 40, 24,
                    Component.literal(module.enabled ? "ENABLED" : "DISABLED"),
                    b -> {
                        module.enabled = !module.enabled;
                        save();
                        ((MenuButton) b).setPrimary(module.enabled);
                        b.setMessage(Component.literal(module.enabled ? "ENABLED" : "DISABLED"));
                    }, module.enabled));

            xBox = new EditBox(font, panelX + 20, panelY + 100, panelW - 40, 20, Component.literal("X position"));
            xBox.setValue(fmt("%.3f", module.x));
            xBox.setResponder(s -> {
                try {
                    double v = Double.parseDouble(s.trim());
                    if (v >= 0 && v <= 1) {
                        module.x = v;
                        save();
                    }
                } catch (NumberFormatException ignored) {
                }
            });
            addRenderableWidget(xBox);

            yBox = new EditBox(font, panelX + 20, panelY + 138, panelW - 40, 20, Component.literal("Y position"));
            yBox.setValue(fmt("%.3f", module.y));
            yBox.setResponder(s -> {
                try {
                    double v = Double.parseDouble(s.trim());
                    if (v >= 0 && v <= 1) {
                        module.y = v;
                        save();
                    }
                } catch (NumberFormatException ignored) {
                }
            });
            addRenderableWidget(yBox);

            addRenderableWidget(new MenuButton(panelX + 20, panelY + 168, 80, 20,
                    Component.literal("Reset"), b -> resetModule(), false));

            addDoneCancel(() -> {
                applyBoxes();
                onClose();
            });
        }

        private void resetModule() {
            double x = 0.015;
            double y = switch (key) {
                case "cps" -> 0.075;
                case "keystrokes" -> 0.150;
                case "coords" -> 0.850;
                case "day" -> 0.225;
                case "ip" -> 0.300;
                case "armor" -> 0.015;
                default -> 0.015;
            };
            module.x = x;
            module.y = y;
            save();
            if (xBox != null) {
                xBox.setValue(fmt("%.3f", x));
            }
            if (yBox != null) {
                yBox.setValue(fmt("%.3f", y));
            }
        }

        private void applyBoxes() {
            if (xBox != null) {
                try {
                    module.x = clamp(Double.parseDouble(xBox.getValue().trim()), 0, 1);
                } catch (NumberFormatException ignored) {
                }
            }
            if (yBox != null) {
                try {
                    module.y = clamp(Double.parseDouble(yBox.getValue().trim()), 0, 1);
                } catch (NumberFormatException ignored) {
                }
            }
            save();
        }

        @Override
        public void extractRenderState(GuiGraphicsExtractor g, int mouseX, int mouseY, float partialTick) {
            super.extractRenderState(g, mouseX, mouseY, partialTick);
            g.text(font, "X POSITION", panelX + 20, panelY + 88, TEXT_FAINT, false);
            g.text(font, "Y POSITION", panelX + 20, panelY + 126, TEXT_FAINT, false);
        }
    }

    private static final class MenuButton extends Button {
        private boolean primary;

        MenuButton(int x, int y, int width, int height, Component message, OnPress onPress, boolean primary) {
            super(x, y, width, height, message, onPress, DEFAULT_NARRATION);
            this.primary = primary;
        }

        void setPrimary(boolean primary) {
            this.primary = primary;
        }

        @Override
        protected void extractContents(GuiGraphicsExtractor g, int mouseX, int mouseY, float partialTick) {
            boolean hover = isHoveredOrFocused() && active;
            int background;
            int foreground;
            int border;
            if (!active) {
                background = 0xFF15171A;
                foreground = 0xFF5C5F66;
                border = 0xFF2A2D31;
            } else if (primary) {
                background = hover ? 0xFF1E4A52 : 0xFF163A40;
                foreground = ACCENT;
                border = ACCENT;
            } else if (hover) {
                background = 0xFF2B2D31;
                foreground = ACCENT;
                border = ACCENT;
            } else {
                background = 0xFF232427;
                foreground = TEXT;
                border = 0xFF3A3D42;
            }
            g.fill(getX(), getY(), getX() + getWidth(), getY() + getHeight(), background);
            box(g, getX(), getY(), getWidth(), getHeight(), border);
            String label = getMessage().getString();
            net.minecraft.client.gui.Font f = Minecraft.getInstance().font;
            g.centeredText(f, label, getX() + getWidth() / 2,
                    getY() + (getHeight() - f.lineHeight) / 2 + 1, foreground);
        }
    }
}
