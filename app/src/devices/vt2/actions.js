// ButtonAction (serde external tagging) helpers. `icon` = VT2 Icons name.
import { usageName } from "$lib/hid.js";

export const ACTIONS = [
  { key: "left", label: "Clique esquerdo", value: { Mouse: 1 }, icon: "clickLeft" },
  { key: "right", label: "Clique direito", value: { Mouse: 2 }, icon: "clickRight" },
  { key: "middle", label: "Clique do meio", value: { Mouse: 4 }, icon: "clickMiddle" },
  { key: "back", label: "Voltar", value: { Mouse: 8 }, icon: "back" },
  { key: "forward", label: "Avançar", value: { Mouse: 16 }, icon: "forward" },
  { key: "dpiUp", label: "DPI +", value: "DpiUp", icon: "dpiUp" },
  { key: "dpiDown", label: "DPI −", value: "DpiDown", icon: "dpiDown" },
  { key: "off", label: "Desativado", value: "Disabled", icon: "disabled" },
];

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
  if (k === "key") return `Tecla ${usageName(a.Key)}`;
  if (k === "other") return "Rolagem (padrão)";
  return ACTIONS.find((x) => x.key === k).label;
}

export const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
