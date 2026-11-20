// Device module: any VIA-compatible keyboard with a definition in ./keyboards/.
// Rust side: crates/periph-app/src/drivers/via.rs (id "via").
import Keymap from "./pages/Keymap.svelte";
import Lighting from "./pages/Lighting.svelte";
import Card from "./Card.svelte";
import Badges from "./Badges.svelte";
import { kbd, demo } from "./store.svelte.js";
import { t } from "$lib/i18n.svelte.js";

export default {
  id: "via",
  get name() {
    return t("via.name");
  },
  kind: "keyboard",
  icon: "keyboard",
  order: 2,
  tabs: [
    { key: "keymap", icon: "keyboard", page: Keymap },
    { key: "lighting", icon: "light", page: Lighting },
  ].map((tab) => ({ ...tab, get label() { return t(`via.tabs.${tab.key}`); } })),
  Card,
  Badges,
  displayName: () => kbd.definition?.displayName ?? kbd.definition?.name ?? (kbd.product || t("via.name")),
  ready: () => kbd.connected && !!kbd.definition && kbd.keymap.length > 0,
  waiting() {
    if (kbd.connected && !kbd.definition)
      return {
        title: t("via.waiting.noDefinitionTitle"),
        text: t("via.waiting.noDefinitionText", { product: kbd.product || t("via.waiting.aKeyboard"), vid: kbd.vendorId.toString(16), pid: kbd.productId.toString(16) }),
      };
    return { title: t(kbd.connected ? "via.waiting.reading" : "via.waiting.searching"), text: t("via.waiting.text") };
  },
  demo,
};
