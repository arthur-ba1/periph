// Device module: Rapoo VT2 Max. Rust side: crates/periph-app/src/drivers/vt2.rs (id "vt2").
import Dpi from "./pages/Dpi.svelte";
import Performance from "./pages/Performance.svelte";
import Sensor from "./pages/Sensor.svelte";
import Buttons from "./pages/Buttons.svelte";
import Card from "./Card.svelte";
import Badges from "./Badges.svelte";
import Hero from "./Hero.svelte";
import { demo } from "./store.svelte.js";

export default {
  id: "vt2",
  name: "Rapoo VT2 Max",
  kind: "Mouse",
  icon: "mouse",
  order: 1,
  tabs: [
    { label: "DPI", icon: "dpi", page: Dpi },
    { label: "Desempenho", icon: "performance", page: Performance },
    { label: "Sensor", icon: "sensor", page: Sensor },
    { label: "Botões", icon: "buttons", page: Buttons },
  ],
  Card,
  Badges,
  Layer: Hero,
  /** Text for the "waiting" screen while the device is absent. */
  waiting(d) {
    if ((d.reason ?? "").includes("dormindo"))
      return { title: "O mouse está dormindo", text: "O dongle está conectado, mas o mouse não respondeu. Mexa ou clique no mouse para acordá-lo; o app tenta de novo sozinho.", hideReason: true };
    return {
      title: d.connected ? "Lendo configuração…" : "Procurando o VT2 Max",
      text: "Conecte o dongle ou o cabo. Se o A HUB estiver aberto, feche-o, porque os dois não podem usar o mouse ao mesmo tempo.",
    };
  },
  demo,
};
