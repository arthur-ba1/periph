// Whether the app's window can be seen. Animation loops stop while it can't and resume through
// onVisible(). WebView2 does not always report a window hidden to the tray as `document.hidden`,
// so the backend also sends "window-visible" (see main.rs).
let windowShown = true;
const listeners = new Set();

export const isVisible = () => windowShown && !document.hidden;

/** Calls `fn` whenever the window becomes visible again; returns the unsubscribe function. */
export function onVisible(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

const notify = () => isVisible() && listeners.forEach((fn) => fn());

if (typeof document !== "undefined") document.addEventListener("visibilitychange", notify);

/** Set by the backend event listener (devices.svelte.js). */
export function setWindowShown(shown) {
  windowShown = shown;
  notify();
}
