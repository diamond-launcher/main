package dev.diamond;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.lang.instrument.ClassFileTransformer;
import java.lang.instrument.Instrumentation;
import java.security.ProtectionDomain;

/**
 * Makes the Minecraft client announce itself as the launcher brand instead of "vanilla".
 * The brand is what the client sends to servers (minecraft:brand channel) and what F3 shows.
 * Only the string constant in net.minecraft.client.ClientBrandRetriever is replaced.
 */
public final class BrandAgent {
    private static final String TARGET = "net/minecraft/client/ClientBrandRetriever";
    private static final String OLD = "vanilla";

    private BrandAgent() {}

    public static void premain(String brand, Instrumentation inst) {
        final String value = (brand == null || brand.isEmpty()) ? "Diamond Launcher" : brand;
        final File log = logFile();
        if (log != null) {
            log.delete();
        }
        log(log, "premain brand=" + value);
        inst.addTransformer(new ClassFileTransformer() {
            @Override
            public byte[] transform(ClassLoader loader, String name, Class<?> cls,
                                    ProtectionDomain pd, byte[] bytes) {
                if (!TARGET.equals(name)) {
                    return null;
                }
                try {
                    byte[] out = replaceConstant(bytes, OLD, value);
                    log(log, "ClientBrandRetriever loaded, bytes=" + bytes.length + " patched=" + (out != null));
                    return out;
                } catch (Throwable t) {
                    log(log, "ClientBrandRetriever patch failed: " + t);
                    return null;
                }
            }
        });
    }

    /** diamond-brand.log next to the agent jar (the game root), so a run can be checked afterwards. */
    private static File logFile() {
        try {
            File jar = new File(BrandAgent.class.getProtectionDomain().getCodeSource().getLocation().toURI());
            return new File(jar.getParentFile(), "diamond-brand.log");
        } catch (Exception e) {
            return null;
        }
    }

    private static void log(File file, String message) {
        System.err.println("[Diamond] " + message);
        if (file == null) {
            return;
        }
        try (FileWriter w = new FileWriter(file, true)) {
            w.write(message + System.lineSeparator());
        } catch (IOException ignored) {
            // Diagnostics only; a failed log write must not affect the game.
        }
    }

    static byte[] replaceConstant(byte[] in, String from, String to) throws Exception {
        DataInputStream din = new DataInputStream(new ByteArrayInputStream(in));
        ByteArrayOutputStream bos = new ByteArrayOutputStream(in.length + 32);
        DataOutputStream out = new DataOutputStream(bos);

        out.writeInt(din.readInt());
        out.writeShort(din.readUnsignedShort());
        out.writeShort(din.readUnsignedShort());
        int count = din.readUnsignedShort();
        out.writeShort(count);

        boolean changed = false;
        for (int i = 1; i < count; i++) {
            int tag = din.readUnsignedByte();
            out.writeByte(tag);
            switch (tag) {
                case 1: {
                    String s = din.readUTF();
                    if (s.equals(from)) {
                        s = to;
                        changed = true;
                    }
                    out.writeUTF(s);
                    break;
                }
                case 3: case 4: case 9: case 10: case 11: case 12: case 17: case 18:
                    out.writeInt(din.readInt());
                    break;
                case 5: case 6:
                    out.writeLong(din.readLong());
                    i++;
                    break;
                case 7: case 8: case 16: case 19: case 20:
                    out.writeShort(din.readUnsignedShort());
                    break;
                case 15:
                    out.writeByte(din.readUnsignedByte());
                    out.writeShort(din.readUnsignedShort());
                    break;
                default:
                    throw new IllegalStateException("unknown constant tag " + tag);
            }
        }

        byte[] rest = new byte[din.available()];
        din.readFully(rest);
        out.write(rest);
        return changed ? bos.toByteArray() : null;
    }
}
