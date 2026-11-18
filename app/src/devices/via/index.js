// Device module: any VIA-compatible keyboard with a definition in ./keyboards/.
// Rust side: crates/periph-app/src/drivers/via.rs (id "via").
import Keymap from "./pages/Keymap.svelte";
import Lighting from "./pages/Lighting.svelte";
import Card from "./Card.svelte";
import Badges from "./Badges.svelte";
import { kbd, demo } from "./store.svelte.js";

export default {
  id: "via",
  name: "Teclado VIA",
  kind: "Teclado",
  icon: "keyboard",
  order: 2,
  tabs: [
    { label: "Keymap", icon: "keyboard", page: Keymap },
    { label: "Iluminação", icon: "light", page: Lighting },
  ],
  Card,
  Badges,
  displayName: () => kbd.definition?.displayName ?? kbd.definition?.name ?? (kbd.product || "Teclado VIA"),
  ready: () => kbd.connected && !!kbd.definition && kbd.keymap.length > 0,
  waiting() {
    if (kbd.connected && !kbd.definition)
      return {
        title: "Teclado VIA sem definição",
        text: `Encontrei ${kbd.product || "um teclado"} (VID ${kbd.vendorId.toString(16)}, PID ${kbd.productId.toString(16)}), mas não há um arquivo de definição VIA para ele em app/src/devices/via/keyboards/.`,
      };
    return {
      title: kbd.connected ? "Lendo o teclado…" : "Procurando teclado VIA",
      text: "Conecte o teclado pelo cabo USB. Se o site do VIA (usevia.app) estiver aberto, feche a aba, porque os dois não podem usar o teclado ao mesmo tempo.",
    };
  },
  demo,
};
