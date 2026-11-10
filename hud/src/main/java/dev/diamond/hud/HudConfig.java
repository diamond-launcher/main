package dev.diamond.hud;

import java.io.IOException;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Properties;

/**
 * Reads {@code diamond-hud.properties} from the game directory.
 *
 * <p>The launcher writes the file before every launch, and the renderer polls
 * it once per second so editing the file by hand also works.</p>
 */
final class HudConfig {
    static final String ACCENT = "#6ee7f0";
    static final String TEXT = "#e8f0f2";

    static final class Module {
        boolean enabled;
        double x;
        double y;

        Module(boolean enabled, double x, double y) {
            this.enabled = enabled;
            this.x = x;
            this.y = y;
        }
    }

    static final class Settings {
        boolean enabled;
        float scale;
        float background;
        int accent;
        int text;
        int keybind; // GLFW key code
        int cornerRadius; // 0 = no rounding, higher = more rounded
        Module fps;
        Module cps;
        Module keystrokes;
        Module coords;
        Module day;
        Module ip;
        Module armor;
        Module fullbright;
        /// Client-side time of day override (0..24000 ticks).
        Module timeChanger;
        /// Shows the player's current server ping.
        Module ping;
        /// Toggle sneak behavior (hold vs toggle).
        Module toggleSneak;
        /// Toggle sprint behavior (hold vs toggle).
        Module toggleSprint;

        static Settings defaults() {
            Settings s = new Settings();
            s.enabled = true;
            s.scale = 1.0f;
            s.background = 0.55f;
            s.accent = 0xFF6EE7F0;
            s.text = 0xFFE8F0F2;
            s.keybind = 344; // GLFW_KEY_RIGHT_SHIFT
            s.cornerRadius = 3;
            // Spread out defaults so they don't overlap
            s.fps = new Module(true, 0.015, 0.015);
            s.cps = new Module(true, 0.015, 0.075);
            s.keystrokes = new Module(true, 0.015, 0.150);
            s.coords = new Module(true, 0.015, 0.850);
            s.day = new Module(true, 0.015, 0.225);
            s.ip = new Module(true, 0.015, 0.300);
            s.armor = new Module(true, 0.850, 0.015);
            s.fullbright = new Module(false, 0.850, 0.075);
            s.timeChanger = new Module(false, 0.850, 0.135);
            s.ping = new Module(false, 0.850, 0.200);
            s.toggleSneak = new Module(false, 0.850, 0.265);
            s.toggleSprint = new Module(false, 0.850, 0.330);
            return s;
        }
    }

    private static final long CHECK_INTERVAL_NS = 1_000_000_000L;

    private final Path path;
    private volatile Settings settings = Settings.defaults();
    private long lastModified = Long.MIN_VALUE;
    private long lastCheck;

    HudConfig(Path path) {
        this.path = path;
    }

    Settings get() {
        return settings;
    }

    /** Reloads the file when it changed. Safe to call every frame. */
    void refresh() {
        long now = System.nanoTime();
        if (now - lastCheck < CHECK_INTERVAL_NS) {
            return;
        }
        lastCheck = now;
        try {
            long modified = Files.getLastModifiedTime(path).toMillis();
            if (modified == lastModified) {
                return;
            }
            lastModified = modified;
            Properties properties = new Properties();
            try (Reader reader = Files.newBufferedReader(path, StandardCharsets.UTF_8)) {
                properties.load(reader);
            }
            settings = parse(properties);
        } catch (IOException ignored) {
            // No config yet (or it is being rewritten): keep the current values.
        }
    }

    /** Writes the current settings to disk atomically. */
    void write(Settings s) {
        Properties properties = new Properties();
        properties.setProperty("enabled", Boolean.toString(s.enabled));
        properties.setProperty("scale", Float.toString(s.scale));
        properties.setProperty("background", Float.toString(s.background));
        properties.setProperty("accent", String.format("#%06X", s.accent & 0xFFFFFF));
        properties.setProperty("text", String.format("#%06X", s.text & 0xFFFFFF));
        properties.setProperty("keybind", Integer.toString(s.keybind));
        properties.setProperty("cornerRadius", Integer.toString(s.cornerRadius));
        writeModule(properties, "fps", s.fps);
        writeModule(properties, "cps", s.cps);
        writeModule(properties, "keystrokes", s.keystrokes);
        writeModule(properties, "coords", s.coords);
        writeModule(properties, "day", s.day);
        writeModule(properties, "ip", s.ip);
        writeModule(properties, "armor", s.armor);
        writeModule(properties, "fullbright", s.fullbright);
        writeModule(properties, "timeChanger", s.timeChanger);
        writeModule(properties, "ping", s.ping);
        writeModule(properties, "toggleSneak", s.toggleSneak);
        writeModule(properties, "toggleSprint", s.toggleSprint);

        Path tmp = path.resolveSibling(path.getFileName() + ".tmp");
        try (var writer = Files.newBufferedWriter(tmp, StandardCharsets.UTF_8)) {
            properties.store(writer, "Modified in-game by Diamond HUD.");
        } catch (IOException ignored) {
            return;
        }
        try {
            Files.move(tmp, path, java.nio.file.StandardCopyOption.REPLACE_EXISTING);
        } catch (IOException ignored) {
            try { Files.deleteIfExists(tmp); } catch (IOException ignored2) {}
            return;
        }
        this.settings = s;
        try {
            lastModified = Files.getLastModifiedTime(path).toMillis();
        } catch (IOException ignored) {
            lastModified = System.currentTimeMillis();
        }
    }

    private static void writeModule(Properties p, String prefix, Module m) {
        p.setProperty(prefix + ".enabled", Boolean.toString(m.enabled));
        p.setProperty(prefix + ".x", Double.toString(m.x));
        p.setProperty(prefix + ".y", Double.toString(m.y));
    }

    private static Settings parse(Properties p) {
        Settings s = Settings.defaults();
        s.enabled = bool(p, "enabled", s.enabled);
        s.scale = number(p, "scale", s.scale, 0.5f, 2.0f);
        s.background = number(p, "background", s.background, 0.0f, 1.0f);
        s.accent = color(p, "accent", s.accent);
        s.text = color(p, "text", s.text);
        s.keybind = number(p, "keybind", s.keybind, 0, 1000);
        s.cornerRadius = number(p, "cornerRadius", s.cornerRadius, 0, 3);
        s.fps = module(p, "fps", s.fps);
        s.cps = module(p, "cps", s.cps);
        s.keystrokes = module(p, "keystrokes", s.keystrokes);
        s.coords = module(p, "coords", s.coords);
        s.day = module(p, "day", s.day);
        s.ip = module(p, "ip", s.ip);
        s.armor = module(p, "armor", s.armor);
        s.fullbright = module(p, "fullbright", s.fullbright);
        s.timeChanger = module(p, "timeChanger", s.timeChanger);
        s.ping = module(p, "ping", s.ping);
        s.toggleSneak = module(p, "toggleSneak", s.toggleSneak);
        s.toggleSprint = module(p, "toggleSprint", s.toggleSprint);
        return s;
    }

    private static Module module(Properties p, String prefix, Module fallback) {
        return new Module(
                bool(p, prefix + ".enabled", fallback.enabled),
                number(p, prefix + ".x", (float) fallback.x, 0.0f, 1.0f),
                number(p, prefix + ".y", (float) fallback.y, 0.0f, 1.0f)
        );
    }

    private static boolean bool(Properties p, String key, boolean fallback) {
        String value = p.getProperty(key);
        if (value == null) {
            return fallback;
        }
        value = value.trim();
        return value.equalsIgnoreCase("true") || value.equals("1") || value.equalsIgnoreCase("yes");
    }

    private static int number(Properties p, String key, int fallback, int min, int max) {
        String value = p.getProperty(key);
        if (value == null) {
            return fallback;
        }
        try {
            return Math.max(min, Math.min(max, Integer.parseInt(value.trim())));
        } catch (NumberFormatException ignored) {
            return fallback;
        }
    }

    private static float number(Properties p, String key, float fallback, float min, float max) {
        String value = p.getProperty(key);
        if (value == null) {
            return fallback;
        }
        try {
            return Math.max(min, Math.min(max, Float.parseFloat(value.trim())));
        } catch (NumberFormatException ignored) {
            return fallback;
        }
    }

    private static int color(Properties p, String key, int fallback) {
        String value = p.getProperty(key);
        if (value == null) {
            return fallback;
        }
        value = value.trim();
        if (value.startsWith("#")) {
            value = value.substring(1);
        }
        if (value.length() == 6) {
            value = "FF" + value;
        }
        if (value.length() != 8) {
            return fallback;
        }
        try {
            return (int) Long.parseLong(value, 16);
        } catch (NumberFormatException ignored) {
            return fallback;
        }
    }
}
