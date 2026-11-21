import { mount } from "svelte";
import "./app.css";

document.documentElement.classList.add("dark");
const q = location.search;
// The tray flyout window (flyout.rs) loads the same page; it only needs src/tray/.
const tray = q.includes("tray") || window.__TAURI_INTERNALS__?.metadata?.currentWindow?.label === "tray";
// Device packages become modules before the app mounts (the module list is not reactive).
if (!tray) await (await import("./lib/packages.js")).loadPackages();
const Root = tray
  ? (await import("./tray/Flyout.svelte")).default
  : q.includes("icons")
    ? (await import("./lib/icons/IconSheet.svelte")).default
    : q.includes("3d")
      ? (await import("./devices/vt2/components/Mouse3DSheet.svelte")).default
      : (await import("./App.svelte")).default;
export default mount(Root, { target: document.getElementById("app") });
