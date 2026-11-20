import * as THREE from "three";
import { controlAxis, controlPressed, controlTrigger } from "./controller-input.js";
import { splitControlGeometry } from "./control-geometry.js";

// Centres measured in the front-facing preview, relative to the model's width.
// IDs are DualSense input roles, so Xbox A/B/X/Y follow the equivalent physical positions.
const CENTERS = {
  dualshock4: {
    MAT_3: [["touchpad", -0.027, 0.203, 0.091]],
    MAT_4: [["l3", -0.184, 0.002, 0.114], ["r3", 0.132, 0.002, 0.178]],
    MAT_5: [
      ["share", -0.233, 0.248, 0.070], ["options", 0.172, 0.248, 0.154], ["ps", -0.031, -0.001, 0.114],
      ["up", -0.344, 0.194, 0.066], ["right", -0.300, 0.148, 0.084],
      ["down", -0.347, 0.101, 0.080], ["left", -0.391, 0.148, 0.062],
      ["triangle", 0.276, 0.221, 0.195], ["circle", 0.345, 0.149, 0.218],
      ["cross", 0.272, 0.077, 0.212], ["square", 0.203, 0.149, 0.189],
    ],
    MAT_6: [
      ["l1", -0.315, 0.310, -0.022], ["l1", -0.310, 0.302, -0.069],
      ["r1", 0.290, 0.310, 0.100], ["r1", 0.293, 0.302, 0.053],
      ["l2", -0.301, 0.257, -0.138], ["r2", 0.298, 0.257, -0.017],
    ],
  },
  xbox360: {
    Body: [
      ["l3", -0.312, 0.215, 0.077], ["r3", 0.084, 0.020, 0.198], ["dpad", -0.187, 0.010, 0.119],
      ["cross", 0.265, 0.107, 0.148], ["circle", 0.342, 0.186, 0.144],
      ["square", 0.186, 0.183, 0.134], ["triangle", 0.263, 0.262, 0.130],
      ["share", -0.133, 0.189, 0.084], ["options", 0.078, 0.189, 0.126], ["ps", -0.029, 0.190, 0.106],
      ["l1", -0.278, 0.359, -0.041], ["r1", 0.256, 0.359, 0.067],
      ["l2", -0.251, 0.310, -0.158], ["r2", 0.279, 0.310, -0.050],
    ],
  },
};

/** Add a pose rig in the same front-facing coordinates used to center the preview. */
export function rigPreview(model, source, center, width) {
  const rig = new THREE.Group();
  rig.add(source);
  rig.updateMatrixWorld(true);
  const normalized = new THREE.Matrix4().makeScale(1 / width, 1 / width, 1 / width)
    .multiply(new THREE.Matrix4().makeTranslation(center.clone().negate()));
  const parts = new Map(), meshes = [];
  source.traverse((mesh) => { if (mesh.isMesh && CENTERS[model]?.[mesh.material.name]) meshes.push(mesh); });

  for (const mesh of meshes) {
    const transform = normalized.clone().multiply(mesh.matrixWorld);
    const pieces = splitControlGeometry(mesh, transform, (material, { n, min, max }) => {
      if (n < 50) return null;
      return CENTERS[model][material].find(([, ...coords]) =>
        coords.every((value, axis) => Math.abs(value - (min[axis] + max[axis]) / 2) < 0.014),
      )?.[0] ?? null;
    });
    for (const { id, geometry, bounds } of pieces) {
      if (!parts.has(id)) {
        const mount = new THREE.Group();
        mount.name = id;
        mount.position.fromArray(bounds.min.map((value, axis) => (value + bounds.max[axis]) * width / 2)).add(center);
        if (id === "l2" || id === "r2") mount.position.y = bounds.max[1] * width + center.y;
        if (id === "l3" || id === "r3") mount.position.z = bounds.min[2] * width + center.z;
        rig.add(mount);
        parts.set(id, { mount, bounds, rest: mount.position.clone(), press: { value: 0 }, heldUntil: 0 });
      }
      const part = parts.get(id);
      const piece = new THREE.Mesh(geometry, mesh.material);
      piece.userData.press = part.press;
      mesh.add(piece);
      part.mount.attach(piece);
    }
  }

  function animate(input, dt, time) {
    const dpad = input?.buttons?.dpad;
    const dpadActive = dpad >= 0 && dpad < 8;
    for (const [id, part] of parts) {
      const { mount, rest, press } = part;
      if (id === "l2" || id === "r2") {
        const amount = controlTrigger(input, id);
        press.value = THREE.MathUtils.damp(press.value, amount, amount > press.value ? 75 : 28, dt);
        mount.rotation.x = -0.42 * press.value;
        continue;
      }
      if (id === "dpad") {
        press.value = THREE.MathUtils.damp(press.value, Number(dpadActive), 36, dt);
        const direction = dpadActive ? dpad * Math.PI / 4 : 0;
        mount.rotation.x = THREE.MathUtils.damp(mount.rotation.x, -0.23 * Math.cos(direction) * Number(dpadActive), 48, dt);
        mount.rotation.y = THREE.MathUtils.damp(mount.rotation.y, 0.23 * Math.sin(direction) * Number(dpadActive), 48, dt);
        continue;
      }
      if (controlPressed(input?.buttons, id)) part.heldUntil = time + 0.04;
      const pressed = time < part.heldUntil;
      press.value = THREE.MathUtils.damp(press.value, Number(pressed), pressed ? 75 : 28, dt);
      const depth = width * (id === "touchpad" ? 0.008 : 0.014) * press.value;
      if (id === "l1" || id === "r1") {
        mount.position.y = rest.y - depth;
      } else {
        mount.position.z = rest.z - depth;
      }
      if (id === "l3" || id === "r3") {
        const i = id === "l3" ? 0 : 2;
        mount.rotation.x = THREE.MathUtils.damp(mount.rotation.x, 0.34 * controlAxis(input?.sticks?.[i + 1]), 65, dt);
        mount.rotation.y = THREE.MathUtils.damp(mount.rotation.y, 0.34 * controlAxis(input?.sticks?.[i]), 65, dt);
      }
    }
    rig.updateMatrixWorld(true);
  }

  return { rig, parts, animate };
}

export function previewTouches(pad, ghost, transform, center, width, color) {
  if (!pad) return () => {};
  const geometry = new THREE.RingGeometry(width * 0.009, width * 0.014, 24);
  const material = new THREE.MeshBasicMaterial({ color, side: THREE.DoubleSide, clippingPlanes: [ghost.clippingPlane] });
  const markers = Array.from({ length: 2 }, () => {
    const marker = new THREE.Mesh(geometry, material);
    marker.matrixAutoUpdate = false;
    marker.visible = false;
    ghost.group.add(marker);
    return marker;
  });
  const ray = new THREE.Raycaster();
  const start = new THREE.Vector3(), down = new THREE.Vector3(0, 0, -1);
  const translation = new THREE.Matrix4();
  const { min, max } = pad.bounds;
  return (touches) => {
    for (const [index, marker] of markers.entries()) {
      const touch = touches?.[index];
      marker.visible = false;
      if (!touch) continue;
      const x = THREE.MathUtils.clamp(touch[0] / 1919, 0, 1);
      const y = THREE.MathUtils.clamp(touch[1] / 1079, 0, 1);
      start.set(THREE.MathUtils.lerp(min[0] + 0.025, max[0] - 0.025, x),
        THREE.MathUtils.lerp(max[1] - 0.023, min[1] + 0.023, y), max[2] + 0.02);
      ray.set(start.multiplyScalar(width).add(center), down);
      const [hit] = ray.intersectObject(pad.mount, true);
      if (!hit) continue;
      hit.point.z += width * 0.003;
      marker.matrix.multiplyMatrices(transform, translation.makeTranslation(hit.point));
      marker.matrixWorldNeedsUpdate = true;
      marker.visible = true;
    }
  };
}
