// Device packages (docs/device-packages.md): devices described in JSON, installed in
// %APPDATA%\Periph\devices\ and run by the backend engine (crates/periph-pack). Each one becomes
// a generated module (src/devices/_package/).
import { invoke } from "@tauri-apps/api/core";
import { DEMO } from "./device-state.svelte.js";
import { addModule } from "./devices.svelte.js";
import { addCatalog } from "./i18n.svelte.js";
import { packageModule } from "../devices/_package/module.js";

// Demo mode with "packages" in the URL (?demo&packages): the example packages of the repository.
const examples = import.meta.glob("../../../packages/*/{manifest,device}.json", { import: "default" });
const exampleLocales = import.meta.glob("../../../packages/*/locales/*.json", { import: "default" });

async function fromRepository() {
  const byId = {};
  for (const [path, load] of Object.entries(examples)) {
    const parts = path.split("/");
    (byId[parts.at(-2)] ??= { locales: {} })[parts.at(-1).replace(".json", "")] = await load();
  }
  for (const [path, load] of Object.entries(exampleLocales)) {
    const parts = path.split("/");
    if (byId[parts.at(-3)]) byId[parts.at(-3)].locales[parts.at(-1).replace(".json", "")] = await load();
  }
  return Object.values(byId).filter((p) => p.manifest && p.device);
}

/** Packages that could not be loaded: [{ dir, error }] (shown by the app, see App.svelte). */
export const failedPackages = [];

export async function loadPackages() {
  let list = [];
  try {
    if (window.__TAURI_INTERNALS__) {
      const r = await invoke("device_packages");
      list = r.packages;
      failedPackages.push(...r.failed);
    } else if (DEMO && location.search.includes("packages")) list = await fromRepository();
  } catch (e) {
    console.error("device packages:", e);
  }
  for (const pkg of list) {
    for (const [lang, json] of Object.entries(pkg.locales ?? {})) addCatalog(lang, pkg.manifest.id, json);
    addModule(packageModule(pkg));
  }
}
