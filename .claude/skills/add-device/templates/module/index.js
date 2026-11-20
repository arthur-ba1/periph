// Device module: <Brand Model>. Rust side: crates/periph-app/src/drivers/mydevice.rs (id "mydevice").
// UI text lives in ./locales/<lang>.json and is read with t("mydevice.<key>").
import Settings from "./pages/Settings.svelte";
import { demo } from "./store.svelte.js";
import { t } from "$lib/i18n.svelte.js";

export default {
  id: "mydevice",
  get name() {
    return t("mydevice.name");
  },
  kind: "mouse", // mouse | keyboard | headset | controller | other
  icon: "mouse", // any name from src/lib/icons/icons.js
  order: 10,
  tabs: [{ key: "performance", icon: "performance", page: Settings }].map((tab) => ({
    ...tab,
    get label() {
      return t(`mydevice.tabs.${tab.key}`);
    },
  })),
  waiting: (d) => ({
    title: t(d.connected ? "mydevice.waiting.reading" : "mydevice.waiting.searching"),
    text: t("mydevice.waiting.text"),
  }),
  demo,
};
