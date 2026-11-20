import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { buildGhost } from "./hologram.js";
import { previewTouches, rigPreview } from "./preview-input.js";

/** Load and center a preview in its authored front-facing pose. */
export async function loadGhostPreview(asset, model, color, place = (centered) => centered) {
  const { scene: source } = await new GLTFLoader().loadAsync(asset.url);
  const posed = new THREE.Group();
  posed.rotation.set(asset.tilt, -0.20, 0, "YXZ");
  posed.add(source);
  const bounds = new THREE.Box3().setFromObject(posed, true);
  const size = bounds.getSize(new THREE.Vector3());
  const center = bounds.getCenter(new THREE.Vector3());
  const { rig, parts, animate } = rigPreview(model, posed, center, size.x);
  const meshes = [];
  rig.traverse((object) => { if (object.isMesh) meshes.push(object); });
  const transform = place(new THREE.Matrix4().makeTranslation(center.clone().negate()), size);
  const ghost = buildGhost(meshes, transform, color, model);
  const placeTouches = previewTouches(parts.get("touchpad"), ghost, transform, center, size.x, color);
  ghost.animate = (input, dt, time) => {
    animate(input, dt, time);
    placeTouches(input?.touches);
  };
  return { ghost, size, source: rig };
}

export function disposeGhostPreview({ ghost, source }) {
  ghost.group.removeFromParent();
  const geometries = new Set(), materials = new Set(), textures = new Set(), images = new Set();
  for (const group of [ghost.group, source]) group.traverse((object) => {
    if (!object.isMesh) return;
    geometries.add(object.geometry);
    for (const material of [object.material].flat()) materials.add(material);
  });
  for (const geometry of geometries) geometry.dispose();
  for (const material of materials) {
    for (const value of Object.values(material)) if (value?.isTexture) textures.add(value);
    material.dispose();
  }
  for (const texture of textures) {
    if (texture.image?.close) images.add(texture.image);
    texture.dispose();
  }
  for (const image of images) image.close();
}
