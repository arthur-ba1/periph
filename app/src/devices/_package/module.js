// Device module generated from a device package (docs/device-packages.md). The folder starts
// with "_" so the registry does not take it for a device; lib/packages.js builds one module
// per installed package with packageModule().
import { t } from "$lib/i18n.svelte.js";
import Page from "./Page.svelte";
import Card from "./Card.svelte";

export const KIND_ICON = { mouse: "mouse", keyboard: "keyboard", headset: "headset", controller: "gamepad", other: "logo" };

/** Translated package text, or `fallback` when the package has no such key. */
export function text(pkg, key, fallback = null, params) {
  const full = `${pkg.manifest.id}.${key}`;
  const s = t(full, params);
  return s === full ? fallback : s;
}

export const settingOf = (pkg, id) => pkg.device.settings.find((s) => s.id === id);
export const codecOf = (pkg, s) => pkg.device.codecs?.[s.type];

/** Values written as "0x08" strings in JSON become numbers. */
export const num = (v) => (typeof v === "string" ? Number(v) : v);

/** A setting the engine will refuse to write (reports transport: no `send` template). */
export const isReadOnly = (pkg, s) => !!s.readOnly || (pkg.device.transport.type === "reports" && !s.send);

/** Which control shows a setting (its `control` hint, or one inferred from its shape). */
export function controlOf(pkg, s) {
  if (codecOf(pkg, s)) return s.count ? "actions" : "action";
  if (s.count) return s.control === "stages" ? "stages" : "list";
  if (isReadOnly(pkg, s)) return "text";
  if (s.control) return s.control;
  if (s.type === "bool" || s.type === "bit") return "switch";
  if (s.indexOf) return "index";
  if (s.values) return s.values.length <= 8 ? "segmented" : "select";
  if (s.range) {
    const steps = (num(s.range.max) - num(s.range.min)) / (num(s.range.step) || 1);
    return steps <= 200 ? "slider" : "number";
  }
  return "text";
}

/** Same change the engine will make, applied to the local config (demo, optimistic UI). */
export function applyLocal(config, cmd) {
  if (cmd.type !== "set" || !config) return;
  if (cmd.index != null) config[cmd.id][cmd.index] = cmd.value;
  else config[cmd.id] = cmd.value;
}

export function packageModule(pkg) {
  const { manifest, device } = pkg;
  const icon = KIND_ICON[manifest.kind] ?? "logo";
  const demo = device.demo;
  return {
    id: manifest.id,
    kind: manifest.kind,
    icon,
    order: 50,
    package: pkg,
    replaces: manifest.replaces,
    get name() {
      return text(pkg, "name", manifest.name);
    },
    tabs: (device.pages ?? []).map((page) => ({
      key: page.id,
      icon: page.icon ?? icon,
      page: Page,
      props: { pkg, page },
      get label() {
        return text(pkg, `pages.${page.id}`, page.id);
      },
    })),
    Card,
    cardProps: { pkg },
    /** Waiting screen: the "asleep" code (transport.response.pendingCode) gets its own text. */
    waiting(d) {
      if (d.code === "asleep") return { title: t("packages.ui.asleepTitle"), text: t("packages.ui.asleepText"), hideReason: true };
      return undefined;
    },
    demo: demo && {
      state: { product: demo.product ?? manifest.name, serial: demo.serial ?? "", config: demo.config, status: demo.status ?? null },
      apply: (d, cmd) => applyLocal(d.config, cmd),
    },
  };
}
