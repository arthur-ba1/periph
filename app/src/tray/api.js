// Tray flyout <-> backend (crates/periph-app/src/flyout.rs). Outside Tauri (`?tray` in the
// browser) every call is a no-op and the panel comes from demo.js.
import { invoke } from "@tauri-apps/api/core";

export const TAURI = typeof window !== "undefined" && !!window.__TAURI_INTERNALS__;

const call = (cmd, args) => (TAURI ? invoke(cmd, args).catch(() => {}) : Promise.resolve());

export const hide = () => call("flyout_hide");
/** Open the main window, on `device` if given. */
export const openDevice = (device) => call("flyout_open", { device: device ?? null });
/** A quick setting (item id from the driver's `Tray`) of `device`. */
export const command = (device, item) => call("flyout_command", { device, item });
export const setAutostart = (enabled) => call("flyout_autostart", { enabled });
export const quit = () => call("flyout_quit");
/** Size of what was rendered for request `seq`: the backend places and shows the window. */
export const layout = (seq, width, height) => call("flyout_layout", { seq, width, height });

export const KIND_ICON = { mouse: "mouse", keyboard: "keyboard", controller: "gamepad", headset: "headset", other: "logo" };
