// Device module: <Brand Model>. Rust side: crates/periph-app/src/drivers/mydevice.rs (id "mydevice").
import Settings from "./pages/Settings.svelte";
import { demo } from "./store.svelte.js";

export default {
  id: "mydevice",
  name: "Brand Model",
  kind: "Mouse",
  icon: "mouse", // any name from src/lib/icons/icons.js
  order: 10,
  tabs: [{ label: "Desempenho", icon: "performance", page: Settings }],
  waiting: (d) => ({
    title: d.connected ? "Lendo configuração…" : "Procurando o Brand Model",
    text: "Conecte o dispositivo. Se o software oficial estiver aberto, feche-o, porque os dois não podem usar o dispositivo ao mesmo tempo.",
  }),
  demo,
};
