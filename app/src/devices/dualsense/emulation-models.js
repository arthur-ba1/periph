const assets = import.meta.glob("./assets/{dualshock4,xbox360}.glb", { eager: true, query: "?url", import: "default" });

export const EMULATION_MODELS = {
  dualsense: { supported: true, icon: "dualsense" },
  dualshock4: { supported: false, icon: "dualshock4", url: assets["./assets/dualshock4.glb"], tilt: -0.12 },
  xbox360: { supported: false, icon: "xbox360", url: assets["./assets/xbox360.glb"], tilt: 0.49 },
};
