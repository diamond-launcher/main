package dev.diamond.hud.mixin;

import dev.diamond.hud.ServerTps;
import net.minecraft.client.multiplayer.ClientPacketListener;
import net.minecraft.network.protocol.game.ClientboundSetTimePacket;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/**
 * Feeds server time updates to {@link ServerTps}.
 *
 * <p>{@code TAIL} runs only on the client thread: an off-thread call schedules
 * the packet and leaves the method early, so a HEAD injection would record the
 * same packet twice.</p>
 */
@Mixin(ClientPacketListener.class)
public class ClientPacketListenerMixin {
    @Inject(method = "handleSetTime", at = @At("TAIL"))
    private void diamondhud$recordServerTime(ClientboundSetTimePacket packet, CallbackInfo ci) {
        ServerTps.onTimeUpdate(this, packet.gameTime());
    }
}
