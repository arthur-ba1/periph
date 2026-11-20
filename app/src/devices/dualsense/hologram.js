import * as THREE from "three";

const PRINTS = {
  dualsense: {
    "1001": `
      vec3 p = controllerPosition;
      if (abs(p.x) < 0.075 || abs(p.x) > 0.174 || p.y < 0.048 || p.z < -0.102 || p.z > -0.01) discard;
      float value = dot(sampledDiffuseColor.rgb, vec3(0.2126, 0.7152, 0.0722));
      float mark = smoothstep(0.10, 0.22, value) * (1.0 - smoothstep(0.52, 0.69, value));
    `,
  },
  dualshock4: {
    MAT_1: `
      // The arrows are printed on the ring around the D-pad, in the front shell atlas.
      if (abs(distance(vMapUv, vec2(0.2905, 0.1768)) - 0.0735) > 0.013) discard;
      float value = sampledDiffuseColor.r;
      float mark = 0.65 * smoothstep(0.009, 0.016, value) * (1.0 - smoothstep(0.034, 0.075, value));
    `,
    MAT_5: `
      vec3 rgb = sampledDiffuseColor.rgb;
      float high = max(max(rgb.r, rgb.g), rgb.b);
      float low = min(min(rgb.r, rgb.g), rgb.b);
      float mark = smoothstep(0.08, 0.20, high - low);
      if (distance(vMapUv, vec2(0.17, 0.17)) < 0.12) mark = max(mark, smoothstep(0.16, 0.5, low));
    `,
  },
  xbox360: {
    Body: `
      vec3 rgb = sampledDiffuseColor.rgb;
      float mark = 0.0;
      if (vMapUv.x > 0.307 && vMapUv.x < 0.387 && vMapUv.y > 0.015 && vMapUv.y < 0.095) {
        mark = smoothstep(0.053, 0.11, min(min(rgb.r, rgb.g), rgb.b));
      } else if (vMapUv.x > 0.018 && vMapUv.x < 0.080 && vMapUv.y > 0.912 && vMapUv.y < 0.975) {
        mark = smoothstep(0.05, 0.15, rgb.g - rgb.b);
      }
    `,
  },
};

// Show the GLB's original button print over the ghost's untextured caps.
function createButtonPrint(source, transform, clip, model, mask) {
  const material = new THREE.MeshBasicMaterial({
    map: source.map,
    alphaTest: 0.999,
    clippingPlanes: [clip],
    transparent: true,
    depthTest: false,
    depthWrite: false,
  });
  material.customProgramCacheKey = () => `ghost-print:${model}:${source.name}`;
  material.onBeforeCompile = (shader) => {
    shader.uniforms.controllerTransform = { value: transform };
    shader.vertexShader = `
      uniform mat4 controllerTransform;
      varying vec3 controllerPosition;
    ` + shader.vertexShader.replace("#include <begin_vertex>", `
      #include <begin_vertex>
      controllerPosition = (controllerTransform * vec4(position, 1.0)).xyz;
    `);
    shader.fragmentShader = "varying vec3 controllerPosition;\n" + shader.fragmentShader.replace(
      "#include <opaque_fragment>",
      `
        ${mask}
        if (mark <= 0.0) discard;
        outgoingLight = vec3(0.68, 0.93, 1.0);
        diffuseColor.a = 0.7 * mark;
        #include <opaque_fragment>
      `,
    );
  };
  return material;
}

/** Translucent copy of the controller, driven by the same moving pieces. */
export function buildGhost(meshes, toModel, color, model = "dualsense") {
  const group = new THREE.Group();
  const clip = new THREE.Plane();
  const bounds = new THREE.Box3();
  const copies = [];
  const clipping = { side: THREE.DoubleSide, clippingPlanes: [clip] };

  // Keep the nearest surface so the controller's back and internal parts cannot muddy the face.
  const depth = new THREE.MeshBasicMaterial({ ...clipping, colorWrite: false });
  const finishes = new Map();
  function finishFor(press) {
    if (finishes.has(press)) return finishes.get(press);
    const finish = new THREE.MeshNormalMaterial({
      ...clipping,
      transparent: true,
      forceSinglePass: true,
      depthWrite: false,
      depthFunc: THREE.EqualDepth,
    });
    finish.onBeforeCompile = (shader) => {
      shader.uniforms.ghostTint = { value: color };
      shader.uniforms.ghostPress = press ?? { value: 0 };
      shader.fragmentShader = "uniform vec3 ghostTint;\nuniform float ghostPress;\n" + shader.fragmentShader.replace(
        "gl_FragColor = vec4( normalize( normal ) * 0.5 + 0.5, diffuseColor.a );",
        `
          float rim = pow(1.0 - abs(normal.z), 1.65);
          float key = smoothstep(-0.35, 0.93, dot(normal, vec3(-0.45, 0.74, 0.50)));
          vec3 tint = mix(ghostTint, vec3(0.68, 0.93, 1.0), 0.65 * rim + 0.16 * ghostPress);
          gl_FragColor = vec4(tint * (0.27 + 0.55 * key + 0.55 * rim),
                              0.045 + 0.105 * key + 0.56 * rim + 0.10 * ghostPress);
          #include <colorspace_fragment>
        `,
      );
    };
    finishes.set(press, finish);
    return finish;
  }

  for (const mesh of meshes) {
    const { geometry } = mesh;
    const transform = new THREE.Matrix4().multiplyMatrices(toModel, mesh.matrixWorld);
    if (!geometry.boundingBox) geometry.computeBoundingBox();
    bounds.union(geometry.boundingBox.clone().applyMatrix4(transform));

    const surface = new THREE.Mesh(geometry, depth);
    surface.matrixAutoUpdate = false;
    surface.matrix.copy(transform);
    surface.add(new THREE.Mesh(geometry, finishFor(mesh.userData.press)));
    const { name, map } = mesh.material;
    const mask = PRINTS[model]?.[name];
    if (map && mask) {
      const print = new THREE.Mesh(geometry, createButtonPrint(mesh.material, transform, clip, model, mask));
      print.renderOrder = 1;
      surface.add(print);
    }
    group.add(surface);
    copies.push([mesh, surface]);
  }

  group.visible = false;
  return {
    group,
    clippingPlane: clip,
    zMin: bounds.min.z,
    zMax: bounds.max.z,
    update(physicalClip) {
      clip.copy(physicalClip).negate();
      for (const [mesh, surface] of copies) {
        surface.matrix.multiplyMatrices(toModel, mesh.matrixWorld);
        surface.matrixWorldNeedsUpdate = true;
      }
    },
  };
}
