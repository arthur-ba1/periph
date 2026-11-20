// Device module: Sony DualSense (USB) with virtual controller emulation.
// Rust side: crates/periph-app/src/drivers/dualsense.rs (id "dualsense").
import Emulation from "./pages/Emulation.svelte";
import Lights from "./pages/Lights.svelte";
import Triggers from "./pages/Triggers.svelte";
import Card from "./Card.svelte";
import Badges from "./Badges.svelte";
import StageLayer from "./stage-layer.svelte";
import { demo } from "./store.svelte.js";
import { t } from "$lib/i18n.svelte.js";

export default {
  id: "dualsense",
  name: "DualSense",
  kind: "controller",
  icon: "gamepad",
  order: 3,
  tabs: [
    { key: "emulation", icon: "emulate", page: Emulation },
    { key: "lights", icon: "lightbar", page: Lights },
    { key: "triggers", icon: "trigger", page: Triggers },
  ].map((tab) => ({
    ...tab,
    get label() {
      return t(`dualsense.tabs.${tab.key}`);
    },
  })),
  Card,
  Badges,
  Layer: StageLayer,
  /** Text for the "waiting" screen while the controller is absent. */
  waiting(d) {
    if (d.code === "emulatorRunning") return { title: t("dualsense.waiting.emulatorRunningTitle"), text: t("dualsense.errors.emulatorRunning"), hideReason: true };
    if (d.code === "hidden") return { title: t("dualsense.waiting.hiddenTitle"), text: t("dualsense.errors.hidden"), hideReason: true };
    return { title: t(d.connected ? "dualsense.waiting.reading" : "dualsense.waiting.searching"), text: t("dualsense.waiting.text"), hideReason: d.code === "notFound" };
  },
  demo,
};
