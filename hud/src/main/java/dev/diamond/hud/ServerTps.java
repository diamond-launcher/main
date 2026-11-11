package dev.diamond.hud;

import java.util.ArrayDeque;
import java.util.Locale;

/**
 * Measures the server tick rate from the game time carried by
 * {@code ClientboundSetTimePacket}.
 *
 * <p>The server sends that packet once every 20 ticks and its game time advances
 * by exactly one per server tick, so ticks divided by wall-clock seconds over a
 * short window is the real server TPS. Nothing is estimated when no packet has
 * arrived recently: the value is then reported as unknown and the HUD shows
 * {@code --} instead of a made-up number.</p>
 */
public final class ServerTps {
    /** The estimate covers at most this much wall time, so drops show up quickly. */
    private static final long WINDOW_NS = 3_000_000_000L;
    /** No time update for this long means the value is stale (paused or disconnected). */
    private static final long STALE_NS = 3_000_000_000L;

    private record Sample(long nanos, long gameTime) {
    }

    private static final ArrayDeque<Sample> SAMPLES = new ArrayDeque<>();
    /** Connection the samples belong to; a new server starts with a clean window. */
    private static Object connection;

    private ServerTps() {
    }

    /** Called from the time packet handler on the client thread. */
    public static synchronized void onTimeUpdate(Object source, long gameTime) {
        if (source != connection) {
            SAMPLES.clear();
            connection = source;
        }
        long now = System.nanoTime();
        Sample last = SAMPLES.peekLast();
        if (last != null && (now - last.nanos > STALE_NS || gameTime < last.gameTime)) {
            SAMPLES.clear();
        }
        SAMPLES.addLast(new Sample(now, gameTime));
        while (SAMPLES.size() > 2 && now - SAMPLES.peekFirst().nanos > WINDOW_NS) {
            SAMPLES.removeFirst();
        }
    }

    /** Server ticks per second, or {@code -1} when there is no recent measurement. */
    public static synchronized double current() {
        if (SAMPLES.size() < 2) {
            return -1.0;
        }
        Sample first = SAMPLES.peekFirst();
        Sample last = SAMPLES.peekLast();
        long elapsed = last.nanos - first.nanos;
        if (elapsed < 500_000_000L || System.nanoTime() - last.nanos > STALE_NS) {
            return -1.0;
        }
        long ticks = last.gameTime - first.gameTime;
        if (ticks <= 0) {
            return -1.0;
        }
        // Vanilla servers never exceed 20 TPS; clamp away network jitter spikes.
        return Math.min(20.0, ticks * 1_000_000_000.0 / elapsed);
    }

    public static String format(double tps) {
        return String.format(Locale.ROOT, "%.1f", tps);
    }
}
