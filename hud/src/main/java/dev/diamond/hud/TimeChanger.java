package dev.diamond.hud;

import net.minecraft.client.ClientClockManager;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.core.Holder;
import net.minecraft.world.clock.WorldClock;
import net.minecraft.world.clock.WorldClocks;

import java.lang.reflect.Field;
import java.util.Map;
import java.util.Optional;

/**
 * Client-side time of day override for the Time Changer module.
 *
 * <p>The vanilla client clock is only changed for the frame that is about to be
 * rendered: the natural value is restored at the start of every client tick, the
 * tick advances it normally, and the override is applied again at the end of the
 * tick. That keeps the day counter and the natural clock intact, while the sky,
 * sun, moon and stars use the configured time.</p>
 *
 * <p>Nothing is sent to the server, so other players and the real world time are
 * never affected. Turning the module off simply stops applying the override, so
 * the next frame shows the normal world time again.</p>
 */
final class TimeChanger {
    private static final long TICKS_PER_DAY = 24000L;

    private static Field clocksField;
    private static Field totalTicksField;
    private static boolean unavailable;

    private static ClientLevel level;
    private static Holder<WorldClock> clock;
    private static boolean applied;
    private static long naturalTicks;

    private TimeChanger() {
    }

    /** Puts the natural clock value back before the level ticks, so time keeps flowing normally. */
    static void beforeTick() {
        Minecraft mc = Minecraft.getInstance();
        ClientLevel current = mc == null ? null : mc.level;
        if (current != level) {
            // A new level owns a new clock manager; there is nothing to restore.
            level = current;
            clock = null;
            applied = false;
            return;
        }
        if (!applied || current == null) {
            return;
        }
        setTotalTicks(current, naturalTicks);
        applied = false;
    }

    /** Applies the configured time of day after the level tick, before the world is rendered. */
    static void afterTick(HudConfig.Settings settings) {
        Minecraft mc = Minecraft.getInstance();
        ClientLevel current = mc == null ? null : mc.level;
        if (current == null) {
            level = null;
            clock = null;
            applied = false;
            return;
        }
        if (current != level) {
            level = current;
            clock = null;
            applied = false;
        }
        if (!settings.enabled || !settings.timeChanger.enabled) {
            return;
        }
        Holder<WorldClock> holder = clock(current);
        if (holder == null) {
            return;
        }
        long natural = current.clockManager().getTotalTicks(holder);
        long dayBase = natural - Math.floorMod(natural, TICKS_PER_DAY);
        long target = dayBase + Math.floorMod(settings.timeChangerTime, (int) TICKS_PER_DAY);
        if (setTotalTicks(current, target)) {
            naturalTicks = natural;
            applied = true;
        }
    }

    private static Holder<WorldClock> clock(ClientLevel level) {
        if (clock != null) {
            return clock;
        }
        Optional<Holder.Reference<WorldClock>> holder = level.registryAccess().get(WorldClocks.OVERWORLD);
        clock = holder.orElse(null);
        return clock;
    }

    /**
     * Writes the overworld clock's tick counter through reflection.
     *
     * <p>The mod deliberately has no mixin: only two private fields are touched,
     * and the feature is silently skipped if a future Minecraft version changes
     * them instead of crashing the game.</p>
     */
    private static boolean setTotalTicks(ClientLevel level, long value) {
        if (unavailable) {
            return false;
        }
        try {
            if (clocksField == null) {
                clocksField = ClientClockManager.class.getDeclaredField("clocks");
                clocksField.setAccessible(true);
                Class<?> instanceClass = Class.forName("net.minecraft.client.ClientClockManager$ClockInstance");
                totalTicksField = instanceClass.getDeclaredField("totalTicks");
                totalTicksField.setAccessible(true);
            }
            ClientClockManager manager = level.clockManager();
            @SuppressWarnings("unchecked")
            Map<Holder<WorldClock>, Object> clocks =
                    (Map<Holder<WorldClock>, Object>) clocksField.get(manager);
            Object instance = clocks.get(clock);
            if (instance == null) {
                // Ask the public API to create the entry, then look it up again.
                manager.getTotalTicks(clock);
                instance = clocks.get(clock);
            }
            if (instance == null) {
                return false;
            }
            totalTicksField.setLong(instance, value);
            return true;
        } catch (Throwable failure) {
            unavailable = true;
            System.err.println("[DiamondHUD] Time Changer disabled: " + failure);
            return false;
        }
    }
}
