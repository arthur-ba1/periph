import * as THREE from "three";

/** Split the indexed GLB batches into controls without breaking pieces at UV seams. */
export function splitControlGeometry(mesh, transform, classify) {
  const geometry = mesh.geometry;
  if (!geometry.index) return [];
  const { position } = geometry.attributes;
  const indices = geometry.index.array;
  const keys = new Map(), weld = new Int32Array(position.count);
  const points = new Float32Array(position.count * 3), point = new THREE.Vector3();
  for (let i = 0; i < position.count; i++) {
    point.fromBufferAttribute(position, i).applyMatrix4(transform).toArray(points, i * 3);
    const key = `${Math.round(point.x * 1e5)},${Math.round(point.y * 1e5)},${Math.round(point.z * 1e5)}`;
    if (!keys.has(key)) keys.set(key, keys.size);
    weld[i] = keys.get(key);
  }
  const parent = Int32Array.from({ length: keys.size }, (_, i) => i);
  function find(x) {
    while (parent[x] !== x) x = parent[x] = parent[parent[x]];
    return x;
  }
  for (let t = 0; t < indices.length; t += 3) {
    const root = find(weld[indices[t]]);
    parent[find(weld[indices[t + 1]])] = root;
    parent[find(weld[indices[t + 2]])] = root;
  }

  const bounds = new Map();
  for (let i = 0; i < position.count; i++) {
    const root = find(weld[i]);
    let box = bounds.get(root);
    if (!box) bounds.set(root, box = { n: 0, min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity] });
    box.n++;
    for (let axis = 0; axis < 3; axis++) {
      const value = points[i * 3 + axis];
      box.min[axis] = Math.min(box.min[axis], value);
      box.max[axis] = Math.max(box.max[axis], value);
    }
  }
  const owners = new Map(), parts = new Map();
  for (const [root, box] of bounds) {
    const id = classify(mesh.material.name, box);
    if (!id) continue;
    owners.set(root, id);
    if (!parts.has(id)) parts.set(id, { id, bounds: box, indices: [] });
  }
  if (!owners.size) return [];
  const keep = [];
  for (let t = 0; t < indices.length; t += 3) {
    const part = parts.get(owners.get(find(weld[indices[t]])));
    (part?.indices ?? keep).push(indices[t], indices[t + 1], indices[t + 2]);
  }
  geometry.setIndex(keep);
  return Array.from(parts.values(), ({ id, bounds, indices }) => {
    const split = new THREE.BufferGeometry();
    for (const [name, attribute] of Object.entries(geometry.attributes)) split.setAttribute(name, attribute);
    split.setIndex(indices);
    return { id, bounds, geometry: split };
  });
}
