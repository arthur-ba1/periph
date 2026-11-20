// ButtonAction (serde external tagging) helpers. `icon` = VT2 Icons name.
import { usageName } from "$lib/hid.js";
import { t } from "$lib/i18n.svelte.js";

export const ACTIONS = [
  { key: "left", value: { Mouse: 1 }, icon: "clickLeft" },
  { key: "right", value: { Mouse: 2 }, icon: "clickRight" },
  { key: "middle", value: { Mouse: 4 }, icon: "clickMiddle" },
  { key: "back", value: { Mouse: 8 }, icon: "back" },
  { key: "forward", value: { Mouse: 16 }, icon: "forward" },
  { key: "dpiUp", value: "DpiUp", icon: "dpiUp" },
  { key: "dpiDown", value: "DpiDown", icon: "dpiDown" },
  { key: "off", value: "Disabled", icon: "disabled" },
].map((a) => ({ ...a, get label() { return t(`vt2.buttons.actions.${a.key}`); } }));

export function actionKey(a) {
  if (a === "DpiUp") return "dpiUp";
  if (a === "DpiDown") return "dpiDown";
  if (a === "Disabled") return "off";
  if (a && typeof a === "object") {
    if ("Mouse" in a) return { 1: "left", 2: "right", 4: "middle", 8: "back", 16: "forward" }[a.Mouse] ?? "other";
    if ("Key" in a) return "key";
  }
  return "other";
}

export function actionLabel(a) {
  const k = actionKey(a);
  if (k === "key") return t("vt2.buttons.actions.key", { key: usageName(a.Key) });
  if (k === "other") return t("vt2.buttons.actions.other");
  return ACTIONS.find((x) => x.key === k).label;
}

export const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
