// Device module: Rapoo VT2 Max. Rust side: crates/periph-app/src/drivers/vt2.rs (id "vt2").
import Dpi from "./pages/Dpi.svelte";
import Performance from "./pages/Performance.svelte";
import Sensor from "./pages/Sensor.svelte";
import Buttons from "./pages/Buttons.svelte";
import Card from "./Card.svelte";
import Badges from "./Badges.svelte";
import Hero from "./Hero.svelte";
import { demo } from "./store.svelte.js";
import { t } from "$lib/i18n.svelte.js";

export default {
  id: "vt2",
  name: "Rapoo VT2 Max",
  kind: "mouse",
  icon: "mouse",
  order: 1,
  tabs: [
    { key: "dpi", icon: "dpi", page: Dpi },
    { key: "performance", icon: "performance", page: Performance },
    { key: "sensor", icon: "sensor", page: Sensor },
    { key: "buttons", icon: "buttons", page: Buttons },
  ].map((tab) => ({ ...tab, get label() { return t(`vt2.tabs.${tab.key}`); } })),
  Card,
  Badges,
  Layer: Hero,
  /** Text for the "waiting" screen while the device is absent. */
  waiting(d) {
    if (d.code === "asleep") return { title: t("vt2.waiting.asleepTitle"), text: t("vt2.waiting.asleepText"), hideReason: true };
    return { title: t(d.connected ? "vt2.waiting.reading" : "vt2.waiting.searching"), text: t("vt2.waiting.text") };
  },
  demo,
};
