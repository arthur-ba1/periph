<script module>
  // Optional asset (git-ignored): "Playstation 5 Dualsense" by AHarmlessPotato, CC BY 4.0,
  // https://sketchfab.com/3d-models/playstation-5-dualsense-878c1f882808477ab81c2fe86d5a3936
  // (glTF Binary download saved as assets/model.glb). See assets/NOTICE.md.
  const found = import.meta.glob("./assets/model.glb", { eager: true, query: "?url", import: "default" });
  export const MODEL_URL = Object.values(found)[0] ?? null;

  // The model file is fetched once and parsed per view (the sidebar thumbnail and the stage).
  let glb = null;
  const loadGlb = () => (glb ??= fetch(MODEL_URL).then((r) => (r.ok ? r.arrayBuffer() : Promise.reject(r.status))));

  // Model space after the pivot below: +Y up, -Z towards the triggers, X across (metres).
  // Anchors were measured on the model (the lightbar from its emissive texture).
  export const ANCHORS = {
    lightbar: [0, 0.052, -0.118],
    player: [0, 0.058, -0.036],
    mic: [0, 0.047, 0.02],
    l2: [-0.12, 0.014, -0.124],
    r2: [0.12, 0.014, -0.124],
    triggers: [0.12, 0.014, -0.124], // linked: tag on the R2 the camera shows
  };

  const rear = { at: [0, -0.012, -0.035], dir: [0.56, 0.79, -0.84], dist: 0.46 };
  const REAR_TRIGGERS = [["rightShort", 1], ["leftShort", 0]];

  /** Camera per focus: what to look at, from which direction, how far. */
  export const SHOTS = {
    // `at` is a model-space point (it follows the standing pose); `dir` is the camera direction
    // in view space (+Y up, +Z towards the viewer), so every shot stays level.
    overview: { at: [0, 0, -0.01], dir: [0, 0.16, 1], dist: 0.48 },
    lightbar: { at: [0, 0.045, -0.085], dir: [0, 0.3, 1], dist: 0.3 },
    player: { at: [0, 0.058, -0.034], dir: [0, 0.3, 1], dist: 0.13 },
    mic: { at: [0, 0.047, 0.014], dir: [0, 0.3, 1], dist: 0.15 },
    triggers: rear,
    l2: rear,
    r2: rear,
    standing: { at: [0, 0.01, -0.02], dir: [0, 0.12, 1], dist: 0.62 }, // sidebar thumbnail
  };

</script>

<script>
  // 3D controller for the DualSense pages: the camera flies to the part being edited, the
  // lightbar / player LEDs / mic LED show the live config, the triggers and mic button are tinted on the model itself,
  // and callouts tag each part. Pure view: all state comes in through props.
  import { onMount } from "svelte";
  import * as THREE from "three";
  import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
  import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";
  import { zonesOf } from "./store.svelte.js";
  import { buildGhost } from "./hologram.js";
  import { EMULATION_MODELS } from "./emulation-models.js";
  import { disposeGhostPreview, loadGhostPreview } from "./load-ghost-preview.js";
  import { splitControlGeometry } from "./control-geometry.js";
  import { browserControls, controlAxis, controlForPiece, controlPressed, controlTrigger } from "./controller-input.js";
  import { DEMO } from "$lib/device-state.svelte.js";
  import { t } from "$lib/i18n.svelte.js";

  /**
   * cfg: driver config; focus: SHOTS key; virtual: show the emulated controller (a translucent silhouette;
   *   the physical one materializes back from it when this turns off)
   * flash: { part, n } pulse; labels: parts that get a callout; onselect(part);
   * compact: sidebar thumbnail (fixed overview, no callouts).
   */
  // lights: what the driver last wrote to the real pad (status.lights); shown as is when given
  let { cfg = null, lights = null, controls = null, focus = "overview", virtual = false, preview = null, flash = null, labels = [], onselect = null, compact = false, model = "white", fallback = null, class: className = "" } = $props();

  let host;
  let ready = $state(false);
  let failed = $state(!MODEL_URL);
  let failedPreviews = $state({});
  const previewFailed = $derived(!!preview && failedPreviews[preview]);
  /** Screen position of each anchor (px in host), updated every frame. */
  let screen = $state({});
  let triggerTravel = $state([0, 0]);
  let api = null;

  /** Theme colour from a CSS variable (the theme uses oklch, which THREE.Color can't parse). */
  function cssColor(name, fallback) {
    const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    if (!v) return new THREE.Color(fallback);
    const g = Object.assign(document.createElement("canvas"), { width: 1, height: 1 }).getContext("2d");
    g.fillStyle = fallback;
    g.fillStyle = v;
    g.fillRect(0, 0, 1, 1);
    const [r, gr, b] = g.getImageData(0, 0, 1, 1).data;
    return new THREE.Color().setRGB(r / 255, gr / 255, b / 255, THREE.SRGBColorSpace);
  }

  /** Trigger hinge (model space): pivot line at the paddle's top-back edge; `dir` and `travel`
   *  give the press rotation (radians at full travel). */
  const HINGE = { y: (b) => b.max[1], z: (b) => b.min[2], dir: -1, travel: 0.42 };

  /** Player LEDs sit in the dark strip just below the touchpad edge (model space): Z of the
   *  strip's centre line, and X of each LED, the three middle ones closer together as on the
   *  real controller. */
  const LED_Z = -0.036;
  const LED_X = [-0.049, -0.0135, 0, 0.0135, 0.049]; // outer two near the ends of the strip

  const BUZZ = new Set(["vibration", "multiVibration", "galloping", "machine"]);
  const LED_ALPHA = [1, 0.6, 0.3]; // high, medium, low
  const PRESS_DEPTH = { touchpad: 0.003, ps: 0.004, mute: 0.003, l3: 0.004, r3: 0.004 };
  const OPAQUE_ALPHA = 0.999;

  // The export packs opaque plastic and translucent button caps into the same materials.
  // Render the plastic into the depth buffer first so it stays visible beneath the caps.
  function addTransparencyPass(mesh) {
    const opaque = mesh.material;
    const translucent = opaque.clone();
    translucent.color = opaque.color;
    translucent.emissive = opaque.emissive;
    translucent.onBeforeCompile = (shader) => {
      shader.fragmentShader = shader.fragmentShader
        .replace("#include <alphatest_fragment>", `if (diffuseColor.a >= ${OPAQUE_ALPHA}) discard;`)
        .replace("#include <lights_fragment_end>", `#include <lights_fragment_end>
          reflectedLight.directSpecular = vec3(0.0);
          reflectedLight.indirectSpecular = vec3(0.0);`);
    };
    opaque.alphaTest = OPAQUE_ALPHA;
    opaque.transparent = false;
    opaque.depthWrite = true;
    mesh.add(new THREE.Mesh(mesh.geometry, translucent));
  }

  function preserveLightbarHue(material, brightness) {
    const compile = material.onBeforeCompile;
    const cacheKey = material.customProgramCacheKey();
    material.customProgramCacheKey = () => `${cacheKey}:lightbar-hue`;
    material.onBeforeCompile = function (shader) {
      compile.call(this, shader);
      shader.uniforms.lightbarBrightness = brightness;
      // Tone-map reflections on the plastic first. Apply the configured LED color separately
      // so the filmic curve cannot shift its hue, while the emissive map still shapes the glow.
      shader.fragmentShader = "uniform float lightbarBrightness;\n" + shader.fragmentShader
        .replace("#include <emissivemap_fragment>", `#include <emissivemap_fragment>
          float lightbarMix = smoothstep(0.0, 0.6, emissiveColor.r) * lightbarBrightness;
          totalEmissiveRadiance = vec3(0.0);`)
        .replace("#include <tonemapping_fragment>", `#include <tonemapping_fragment>
          gl_FragColor.rgb = mix(gl_FragColor.rgb, emissive, clamp(lightbarMix, 0.0, 1.0));`);
    };
  }

  /** Colourways, as a tone curve over the model's flat-colour textures: [in, out] luminance
   *  points (sRGB, 0..1) plus a tint. The white model's palette is a navy-black (~0.16), grey
   *  print (~0.6), light grey buttons and trims (~0.72-0.78) and the lavender-white shell
   *  (~0.88). Midnight Black: the shell turns charcoal and the black parts go deeper. */
  const STYLES = {
    midnight: {
      curve: [[0, 0], [0.16, 0.055], [0.45, 0.42], [0.6, 0.5], [0.68, 0.36], [0.74, 0.17], [0.88, 0.135], [1, 0.15]],
      tint: [0.97, 0.98, 1.04],
    },
  };

  function tone(curve, x) {
    for (let i = 1; i < curve.length; i++) {
      const [x0, y0] = curve[i - 1], [x1, y1] = curve[i];
      if (x <= x1) return y0 + ((y1 - y0) * (x - x0)) / (x1 - x0);
    }
    return curve.at(-1)[1];
  }
  /** Recoloured copy of a base-colour texture (same UVs, flipY and colour space). */
  function restyle(tex, style) {
    const img = tex.image;
    const c = document.createElement("canvas");
    c.width = img.width;
    c.height = img.height;
    const g = c.getContext("2d", { willReadFrequently: true });
    g.drawImage(img, 0, 0);
    const d = g.getImageData(0, 0, c.width, c.height);
    const lut = new Float32Array(256);
    for (let i = 0; i < 256; i++) lut[i] = tone(style.curve, i / 255);
    const [tr, tg, tb] = style.tint;
    const px = d.data;
    for (let i = 0; i < px.length; i += 4) {
      const r = px[i], gr = px[i + 1], b = px[i + 2];
      const l = Math.round(0.2126 * r + 0.7152 * gr + 0.0722 * b);
      const k = l ? (lut[l] * 255) / l : 0;
      // keep a little of the original hue (the trims' warm/cool casts), mostly neutral
      const n = lut[l] * 255;
      px[i] = Math.min(255, (n * 0.8 + r * k * 0.2) * tr);
      px[i + 1] = Math.min(255, (n * 0.8 + gr * k * 0.2) * tg);
      px[i + 2] = Math.min(255, (n * 0.8 + b * k * 0.2) * tb);
    }
    g.putImageData(d, 0, 0);
    const out = new THREE.CanvasTexture(c);
    for (const p of ["flipY", "colorSpace", "wrapS", "wrapT", "magFilter", "minFilter", "channel"]) out[p] = tex[p];
    out.anisotropy = 4;
    return out;
  }

  onMount(() => {
    if (!MODEL_URL) return;
    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: "high-performance" });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.0;
    renderer.shadowMap.enabled = !compact;
    renderer.shadowMap.type = THREE.PCFShadowMap;
    Object.assign(renderer.domElement.style, { position: "absolute", inset: "0", width: "100%", height: "100%", display: "block", opacity: "0", transition: "opacity 700ms ease" });
    host.prepend(renderer.domElement);

    const scene = new THREE.Scene();
    const pmrem = new THREE.PMREMGenerator(renderer);
    scene.environment = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
    scene.environmentIntensity = 0.75;

    const camera = new THREE.PerspectiveCamera(30, 1, 0.01, 10);

    const key = new THREE.DirectionalLight(0xffffff, 1.5);
    key.position.set(0.3, 0.8, 0.45);
    key.castShadow = !compact;
    key.shadow.mapSize.set(1024, 1024);
    key.shadow.radius = 5;
    Object.assign(key.shadow.camera, { left: -0.3, right: 0.3, top: 0.3, bottom: -0.3, near: 0.1, far: 2 });
    scene.add(key);
    const rim = new THREE.DirectionalLight(0x9fb7ff, 0.7);
    rim.position.set(-0.5, 0.3, -0.6);
    scene.add(rim);

    // everything that belongs to the controller lives in `world`, which stands it up: face to
    // the camera, triggers up. The stage leans it back a little and turns it slightly.
    const world = new THREE.Group();
    world.rotation.order = "YXZ";
    world.rotation.x = compact ? Math.PI / 2 - 0.5 : Math.PI / 2 - 0.32;
    world.rotation.y = compact ? -0.24 : -0.2;
    scene.add(world);
    world.updateMatrixWorld(true);

    let floor = null;
    if (!compact) {
      floor = new THREE.Mesh(new THREE.PlaneGeometry(1.2, 1.2), new THREE.ShadowMaterial({ opacity: 0.32 }));
      floor.rotation.x = -Math.PI / 2;
      floor.receiveShadow = true;
      scene.add(floor);
    }

    // glTF from Collada: the controller's up is -Z and its triggers +Y; turn it to +Y up,
    // triggers towards -Z (away from the default camera).
    const pivot = new THREE.Group();
    const upright = new THREE.Group();
    upright.rotation.x = Math.PI / 2;
    pivot.rotation.y = Math.PI;
    pivot.add(upright);
    world.add(pivot);

    const v3 = (a) => new THREE.Vector3(...a);
    const brand = cssColor("--brand", "#3291ff");

    const lbColor = new THREE.Color();
    const lbBrightness = { value: 0 };

    // physical <-> hologram sweep: holoK 0 = physical, 1 = ghost (eased in the frame loop)
    renderer.localClippingEnabled = !compact;
    const clip = new THREE.Plane();
    const clipNormal = new THREE.Vector3(0, 0, -1);
    const swapClip = new THREE.Plane(), outgoingClip = new THREE.Plane();
    const touchClip = new THREE.Plane(clipNormal.clone(), 1);
    let holo = null, shownGhost = null, holoK = 0;
    let swap = null;
    const previews = new Map(), loadingPreviews = new Set();
    const ghostTint = cssColor("--success", "#3ecf8e");
    const SWEEP_S = 0.7, SWEEP_MARGIN = 0.02;

    // colourways: original base-colour map uuid -> { white: original, <style>: recoloured (lazy) }.
    // Materials carry the uuid in userData, which survives the clones made for the triggers.
    const maps = new Map();
    let style = "white";
    function applyStyle(s) {
      style = Object.hasOwn(STYLES, s) ? s : "white";
      world.traverse((o) => {
        if (!o.isMesh) return;
        const m = o.material, v = maps.get(m.userData.baseMap);
        if (!v) return;
        v[style] ??= restyle(v.white, STYLES[style]);
        if (m.map !== v[style]) ((m.map = v[style]), (m.needsUpdate = true));
      });
    }

    // player LEDs: the model only has them in its texture, so five flat bars are laid on the
    // surface below the touchpad (placed by raycast once the model is in)
    // Each LED is light seen through the strip's frosted plastic: a pill with a hot core that
    // fades out at its edges (soft texture, additive), never a flat shape pasted on top.
    const ledTex = (() => {
      const c = document.createElement("canvas");
      c.width = 256;
      c.height = 64;
      const g = c.getContext("2d");
      g.filter = "blur(7px)";
      g.fillStyle = "rgb(150,160,180)"; // diffused rim, slightly cool
      g.beginPath();
      g.roundRect(22, 18, 212, 28, 14);
      g.fill();
      g.filter = "blur(4px)";
      g.fillStyle = "white"; // hot core
      g.beginPath();
      g.roundRect(46, 25, 164, 14, 7);
      g.fill();
      const tx = new THREE.CanvasTexture(c);
      tx.colorSpace = THREE.SRGBColorSpace;
      return tx;
    })();
    // halo: the light spilling up from each LED towards the touchpad (nothing below it). Lies
    // flat on the surface (not a camera-facing sprite), so it stays tight to the controller.
    const haloTex = (() => {
      const c = document.createElement("canvas");
      c.width = 256;
      c.height = 128;
      const g = c.getContext("2d");
      // centred on the bottom edge, so only the upper half of the glow exists
      const grd = g.createRadialGradient(128, 128, 0, 128, 128, 128);
      grd.addColorStop(0, "rgba(225,232,255,0.9)");
      grd.addColorStop(0.3, "rgba(200,212,255,0.35)");
      grd.addColorStop(1, "rgba(200,212,255,0)");
      g.setTransform(1, 0, 0, 0.5, 0, 64); // squash: wide, short glow along the strip
      g.fillStyle = grd;
      g.fillRect(0, 0, 256, 256);
      const tx = new THREE.CanvasTexture(c);
      tx.colorSpace = THREE.SRGBColorSpace;
      return tx;
    })();
    const leds = [0, 1, 2, 3, 4].map(() => {
      const m = new THREE.Mesh(
        new THREE.PlaneGeometry(0.0125, 0.0031),
        new THREE.MeshBasicMaterial({ map: ledTex, color: 0xffffff, transparent: true, opacity: 0, toneMapped: false, depthWrite: false, blending: THREE.AdditiveBlending, polygonOffset: true, polygonOffsetFactor: -2 }),
      );
      m.visible = false;
      const halo = new THREE.Mesh(
        new THREE.PlaneGeometry(0.028, 0.008),
        new THREE.MeshBasicMaterial({ map: haloTex, transparent: true, opacity: 0, toneMapped: false, depthWrite: false, blending: THREE.AdditiveBlending, polygonOffset: true, polygonOffsetFactor: -1 }),
      );
      // local +Y is "up" in the standing pose (towards the touchpad): the halo sits above the LED
      halo.position.set(0, 0.004, -0.0001);
      m.add(halo);
      m.userData.halo = halo;
      world.add(m);
      return m;
    });
    function placeLeds() {
      world.updateMatrixWorld(true);
      const rc = new THREE.Raycaster();
      const down = new THREE.Vector3(0, -1, 0).transformDirection(world.matrixWorld);
      leds.forEach((m, i) => {
        const from = new THREE.Vector3(LED_X[i], 0.2, LED_Z).applyMatrix4(world.matrixWorld);
        rc.set(from, down);
        const hit = rc.intersectObject(pivot, true)[0];
        if (!hit) return;
        const toModel = new THREE.Matrix4().copy(world.matrixWorld).invert();
        const p = hit.point.clone().applyMatrix4(toModel);
        const n = hit.face.normal.clone().transformDirection(hit.object.matrixWorld).transformDirection(toModel);
        m.position.copy(p).addScaledVector(n, 0.0002);
        m.lookAt(m.position.clone().add(n).applyMatrix4(world.matrixWorld)); // plane faces out of the surface
        m.visible = true;
      });
    }

    // Movable pieces use the original triangles and materials, including the button inserts.
    const parts = new Map();
    const trig = { l2: null, r2: null, mic: null };
    const white = new THREE.Color(0xffffff);
    const micColor = new THREE.Color(0xff7a1a);

    /** The GLB stores separate pieces in material batches. Move each control into its own group. */
    function splitControls(root) {
      world.updateMatrixWorld(true);
      const toModel = new THREE.Matrix4().copy(world.matrixWorld).invert();
      const meshes = [];
      root.traverse((o) => o.isMesh && meshes.push(o));
      for (const o of meshes) {
        const transform = new THREE.Matrix4().multiplyMatrices(toModel, o.matrixWorld);
        const pieces = splitControlGeometry(o, transform, (material, bounds) => {
          const id = controlForPiece(material, bounds);
          return !compact || id === "mute" ? id : null;
        });
        for (const { id, geometry, bounds } of pieces) {
          const mat = o.material.clone();
          if (mat.emissiveMap) mat.emissive = o.material.emissive;
          const mesh = new THREE.Mesh(geometry, mat);
          mesh.castShadow = true;
          mesh.userData.k = 0;
          o.add(mesh);
          const paddle = id === "l2" || id === "r2";
          if (!parts.has(id)) {
            const b = bounds;
            const mount = new THREE.Group();
            mount.name = id;
            if (paddle) mount.position.set((b.min[0] + b.max[0]) / 2, HINGE.y(b), HINGE.z(b));
            if (id === "l3" || id === "r3") {
              mount.position.set((b.min[0] + b.max[0]) / 2, 0.037, (b.min[2] + b.max[2]) / 2);
            }
            world.add(mount);
            parts.set(id, { mount, origin: mount.position.clone(), materials: [], press: { value: 0 }, heldUntil: 0 });
          }
          const part = parts.get(id);
          part.mount.attach(mesh);
          mesh.userData.press = part.press;
          if (!paddle) {
            part.materials.push({ mat, color: mat.color.clone() });
          }
          if (id === "mute") trig.mic = mesh;
          if (paddle) {
            trig[id] = mesh;
            mesh.userData.hinge = part.mount;
            mesh.userData.sim = { pos: 0, phase: "rest", t: 0.3, bounce: 0 };
            mesh.userData.kick = 0;
          }
        }
      }
    }

    const touchMarkers = [];
    function addTouchMarkers() {
      const pad = parts.get("touchpad");
      if (!pad) return;
      const geometry = new THREE.RingGeometry(0.0035, 0.0055, 24);
      const material = new THREE.MeshBasicMaterial({ color: brand, side: THREE.DoubleSide, clippingPlanes: [touchClip] });
      for (let i = 0; i < 2; i++) {
        const marker = new THREE.Mesh(geometry, material);
        marker.rotation.x = -Math.PI / 2;
        marker.visible = false;
        pad.mount.add(marker);
        touchMarkers.push(marker);
      }
    }

    function animateControls(input, dt, time) {
      const k = Math.min(1, dt * 65);
      for (const [id, part] of parts) {
        if (id === "l2" || id === "r2") continue;
        const { mount, origin, press } = part;
        // Hold short taps long enough to be visible, then ease out a little more slowly.
        if (controlPressed(input?.buttons, id)) part.heldUntil = time + 0.04;
        const pressed = time < part.heldUntil;
        press.value = THREE.MathUtils.damp(press.value, Number(pressed), pressed ? 75 : 28, dt);
        mount.position.y = origin.y - (PRESS_DEPTH[id] ?? 0.0055) * press.value;
        if (id === "l1" || id === "r1") mount.position.z = origin.z + 0.004 * press.value;
        if (id === "l3" || id === "r3") {
          const i = id === "l3" ? 0 : 2;
          mount.rotation.x += (0.34 * controlAxis(input?.sticks?.[i + 1]) - mount.rotation.x) * k;
          mount.rotation.z += (-0.34 * controlAxis(input?.sticks?.[i]) - mount.rotation.z) * k;
        }
        if (id === "mute") continue;
        for (const { mat, color } of part.materials) {
          mat.color.lerpColors(color, brand, 0.10 * press.value);
          if (!mat.emissiveMap) mat.emissive.copy(brand).multiplyScalar(0.14 * press.value);
        }
      }
      for (const [i, marker] of touchMarkers.entries()) {
        const touch = input?.touches?.[i];
        marker.visible = !!touch;
        if (touch) {
          const x = THREE.MathUtils.clamp(touch[0] / 1919, 0, 1);
          const y = THREE.MathUtils.clamp(touch[1] / 1079, 0, 1);
          marker.position.set(0.14 * (x - 0.5) + 0.01 * (x - 0.5) * y, 0.063, -0.107 + 0.06 * y);
        }
      }
    }

    // ---- trigger press simulation (while a trigger is being edited) ----
    // A finger presses the paddle over and over: resistance zones slow it down (a weapon
    // effect gives way past its end), vibrating zones shake it, GameCube stops it half-way.
    const RESIST = new Set(["feedback", "weapon", "slope", "multiFeedback", "bow", "custom"]);
    function animateTrigger(tm, e, dt) {
      const sim = tm.userData.sim, hinge = tm.userData.hinge;
      if (!sim || !hinge) return;
      if (!e) {
        sim.pos = Math.max(0, sim.pos - dt * 2.5);
        sim.phase = "rest";
        sim.t = 0.3;
        hinge.rotation.x = HINGE.dir * sim.pos * HINGE.travel;
        return;
      }
      const zones = zonesOf(e) ?? Array(10).fill(e.kind === "custom" ? 6 : 0);
      const zone = Math.min(9, Math.floor(sim.pos * 10));
      const stop = e.kind === "gameCube" ? 0.45 : 1;
      // weapon / bow: resistance that breaks at `end`, then the paddle fires to the bottom
      const breaks = e.kind === "weapon" || e.kind === "bow";
      const breakAt = breaks ? Math.min(1, (e.end + 1) / 10) : 2;
      if (sim.phase === "press") {
        const r = RESIST.has(e.kind) ? zones[zone] : 0;
        // creep harder through the last stretch before the break: builds the tension
        const slow = breaks && sim.pos > breakAt - 0.12 && sim.pos < breakAt ? 2.2 : 1;
        sim.pos = Math.min(stop, sim.pos + (dt * 1.3) / ((1 + r * 0.35) * slow));
        if (breaks && sim.pos >= breakAt - 0.005 && sim.pos < stop) {
          sim.phase = "fire";
          tm.userData.kick = 1; // brief accent burst on the break
        }
        if (sim.pos >= stop) ((sim.phase = "hold"), (sim.t = 0.35));
      } else if (sim.phase === "fire") {
        sim.pos = Math.min(stop, sim.pos + dt * 9); // snaps through in a few frames
        if (sim.pos >= stop) ((sim.phase = "hold"), (sim.t = 0.55), (sim.bounce = 1));
      } else if (sim.phase === "release") {
        sim.pos = Math.max(0, sim.pos - dt * 2.4);
        if (sim.pos <= 0) ((sim.phase = "rest"), (sim.t = 0.45));
      } else if ((sim.t -= dt) <= 0) {
        sim.phase = sim.phase === "hold" ? "release" : "press";
      }
      let shake = 0;
      if (BUZZ.has(e.kind) && zones[zone] > 0 && sim.phase !== "rest") {
        const f = 18 + (e.frequency ?? 6) * 3;
        const a = (zones[zone] / 8) * 0.06;
        shake = e.kind === "galloping"
          ? a * Math.max(0, Math.sin(time * 9)) ** 6 * Math.sin(time * f) * 2 // hoof beats
          : e.kind === "machine"
            ? a * (Math.sin(time * 4) > 0 ? 1 : 0.4) * Math.sin(time * f)
            : a * Math.sin(time * f);
      }
      // tension tremble right before the break, then a hard bounce at the bottom
      if (breaks && sim.phase === "press" && sim.pos > breakAt - 0.12) shake += 0.012 * Math.sin(time * 70);
      if (sim.bounce > 0.01) {
        shake -= sim.bounce * 0.1 * Math.cos((0.55 - sim.t) * 38);
        sim.bounce *= Math.exp(-dt * 7);
      }
      hinge.rotation.x = HINGE.dir * (sim.pos * HINGE.travel + shake);
    }

    // change pulse
    // change pulse: a short burst of accent on the part itself (no overlay)
    const flash = { lightbar: 0, player: 0, mic: 0, l2: 0, r2: 0 };

    // ---- camera: only the interface moves it (no mouse orbit) ----
    // Shots are framed for a 16:10 view; narrower views back off so the width still fits.
    // The camera orbits: target, direction and distance ease separately, so moving between
    // shots swings around the controller instead of cutting through it.
    const cam = { at: new THREE.Vector3(), dir: new THREE.Vector3(0, 0, 1), dist: 0.5 };
    const goal = { at: new THREE.Vector3(), dir: new THREE.Vector3(0, 0, 1), dist: 0.5, shot: compact ? SHOTS.standing : SHOTS.overview, k: 1 };
    let thumbnailBounds = null;
    function aim(shot, k, snap) {
      goal.shot = shot;
      goal.k = k;
      const fit = Math.max(1, 1.6 / camera.aspect);
      goal.at.copy(v3(shot.at)).applyMatrix4(world.matrixWorld);
      goal.dir.copy(v3(shot.dir)).normalize();
      goal.dist = (shot.dist * fit) / k;
      if (thumbnailBounds) {
        thumbnailBounds.getCenter(goal.at);
        const half = thumbnailBounds.getSize(new THREE.Vector3()).multiplyScalar(0.5);
        const right = new THREE.Vector3().crossVectors(camera.up, goal.dir).normalize();
        const up = new THREE.Vector3().crossVectors(goal.dir, right);
        const extent = (axis) => Math.abs(axis.x) * half.x + Math.abs(axis.y) * half.y + Math.abs(axis.z) * half.z;
        const visibleHeight = Math.tan(THREE.MathUtils.degToRad(camera.fov / 2)) * 0.86;
        // Fit both axes at the nearest face of the bounds, leaving room around the model.
        goal.dist = extent(goal.dir) + Math.max(extent(up), extent(right) / camera.aspect) / visibleHeight;
      }
      if (snap) {
        cam.at.copy(goal.at);
        cam.dir.copy(goal.dir);
        cam.dist = goal.dist;
      }
    }

    let gone = false;
    function loadPreview(id) {
      if (!id || previews.has(id) || loadingPreviews.has(id)) return;
      const asset = EMULATION_MODELS[id];
      if (!asset?.url) return;
      loadingPreviews.add(id);
      failedPreviews[id] = false;
      loadGhostPreview(asset, id, ghostTint, (centered, size) => {
        const scale = 0.38 / size.x;
        return world.matrixWorld.clone().invert().scale(new THREE.Vector3(scale, scale, scale)).multiply(centered);
      }).then((loaded) => {
        if (gone) return disposeGhostPreview(loaded);
        loaded.ghost.group.name = id;
        previews.set(id, loaded);
        world.add(loaded.ghost.group);
      }).catch(() => {
        if (!gone) failedPreviews[id] = true;
      }).finally(() => loadingPreviews.delete(id));
    }

    loadGlb().then((buf) => new GLTFLoader().parseAsync(buf, "")).then(
      (gltf) => {
        if (gone) return;
        gltf.scene.traverse((o) => {
          if (!o.isMesh) return;
          o.castShadow = true;
          const m = o.material;
          if (m.map) {
            if (!maps.has(m.map.uuid)) maps.set(m.map.uuid, { white: m.map });
            m.userData.baseMap = m.map.uuid;
          }
          if (m.emissiveMap) {
            // Keep the lightbar's original intensity gradient and let `emissive` set its hue.
            const img = m.emissiveMap.image;
            const c = document.createElement("canvas");
            c.width = img.width;
            c.height = img.height;
            const g = c.getContext("2d");
            g.drawImage(img, 0, 0);
            const d = g.getImageData(0, 0, c.width, c.height);
            for (let i = 0; i < d.data.length; i += 4) {
              const v = Math.max(d.data[i], d.data[i + 1], d.data[i + 2]);
              d.data[i] = d.data[i + 1] = d.data[i + 2] = v;
            }
            g.putImageData(d, 0, 0);
            const tx = new THREE.CanvasTexture(c);
            tx.flipY = m.emissiveMap.flipY;
            tx.colorSpace = THREE.SRGBColorSpace;
            m.emissiveMap = tx;
            m.emissive = lbColor;
          }
        });
        upright.add(gltf.scene);
        splitControls(gltf.scene);
        placeLeds();
        if (compact) {
          thumbnailBounds = new THREE.Box3().setFromObject(pivot, true);
          aim(SHOTS.standing, 1, true);
        } else {
          floor.position.y = new THREE.Box3().setFromObject(pivot).min.y - 0.001;
        }

        const surfaces = [];
        world.traverse((o) => o.isMesh && o.material.userData.baseMap && surfaces.push(o));
        // The sidebar thumbnail always shows the physical controller.
        if (!compact) {
          holo = buildGhost(surfaces, new THREE.Matrix4().copy(world.matrixWorld).invert(), ghostTint);
          if (state.virtual && !state.preview) {
            shownGhost = holo;
            holoK = 1;
          }
        }
        for (const mesh of surfaces) {
          if (mesh.material.transparent) addTransparencyPass(mesh);
        }
        world.traverse((object) => {
          if (object.isMesh && object.material.emissiveMap) preserveLightbarHue(object.material, lbBrightness);
        });
        if (!compact) {
          // everything physical (model and LEDs) is clipped at the sweep edge
          world.traverse((o) => o.isMesh && [o.material].flat().forEach((m) => (m.clippingPlanes = [clip])));
          world.add(holo.group);
          addTouchMarkers();
        }
        applyStyle(state.model);
        ready = true;
        renderer.domElement.style.opacity = "1";
      },
      () => (failed = true),
    );

    const resize = () => {
      const w = host.clientWidth || 1, h = host.clientHeight || 1;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      aim(goal.shot, goal.k, compact || !ready);
    };
    const ro = new ResizeObserver(resize);
    ro.observe(host);
    resize();

    let visible = true;
    const io = new IntersectionObserver(([e]) => (visible = e.isIntersecting));
    io.observe(host);

    // ---- live state (pushed by the $effects below) ----
    let state = { cfg: null, lights: null, focus: "overview", virtual: false, preview: null, labels: [], model: "white" };
    let liveControls = null;
    const tmp = new THREE.Color(), proj = new THREE.Vector3();

    let raf, time = 0, last = performance.now();
    const tick = (now = performance.now()) => {
      raf = requestAnimationFrame(tick);
      const dt = Math.min((now - last) / 1000, 0.05);
      last = now;
      time += dt;
      if (!visible || document.hidden) return;
      const c = state.cfg;
      const input = compact ? null : liveControls ?? (DEMO ? browserControls() : null);
      animateControls(input, dt, time);
      const lit = (p) => state.focus === p || (state.focus === "triggers" && (p === "l2" || p === "r2"));

      // Lights: with a connected pad, exactly what was written to it (by Periph or the game), so
      // the model matches the controller; otherwise (demo, offline) a preview of the config.
      const shown = state.lights;
      // lightbar
      const lb = c?.lightbar;
      let lbK = 0;
      if (shown) {
        const [r, g, b] = shown.lightbar ?? [0, 0, 0];
        const peak = Math.max(r, g, b);
        lbK = peak / 255;
        if (peak) lbColor.setRGB(r / peak, g / peak, b / peak, THREE.SRGBColorSpace);
      } else if (lb && lb.mode !== "none" && lb.mode !== "off") {
        // same timing and curves as the driver (Lightbar::color_at in effects.rs)
        const period = 3.25 - 0.25 * lb.speed;
        const ph = (time % period) / period;
        lbK = lb.brightness / 100;
        if (lb.mode === "breathing") lbK *= 0.5 - 0.5 * Math.cos(ph * Math.PI * 2);
        if (lb.mode === "strobe") lbK *= ph < 0.5 ? 1 : 0;
        if (lb.mode === "rainbow") lbColor.setHSL(ph, 1, 0.5);
        else lbColor.setRGB(lb.color[0] / 255, lb.color[1] / 255, lb.color[2] / 255, THREE.SRGBColorSpace);
      }
      for (const p in flash) flash[p] = Math.max(0, flash[p] - dt / 0.7);
      lbBrightness.value = lbK * (1 + flash.lightbar * 0.3);

      // player LEDs
      const pl = shown ? (shown.playerLeds ? { enabled: true, mask: shown.playerLeds[0], brightness: shown.playerLeds[1] } : null) : c?.playerLeds;
      leds.forEach((m, i) => {
        const on = !!pl?.enabled && ((pl.mask >> i) & 1) === 1;
        // lit: white at the chosen level; unlit: dark like the real strip (a faint trace of the
        // lens while editing, so you can see where it is)
        m.material.color.set(0xffffff);
        m.material.opacity = on ? LED_ALPHA[pl.brightness] : compact ? 0 : Math.max(lit("player") ? 0.07 : 0, flash.player * 0.15);
        m.userData.halo.material.opacity = on ? LED_ALPHA[pl.brightness] * 0.85 : 0;
      });

      // mic button: orange while the LED is on, accent while it is being edited
      const mic = c?.micLed;
      const micOn = shown
        ? !!shown.micLed
        : mic?.mode === "on" || (mic?.mode === "blink" && Math.floor((time * 1000) / mic.intervalMs) % 2 === 0);
      if (trig.mic) {
        const tm = trig.mic;
        const target = micOn ? 1 : !compact && lit("mic") ? 0.6 : 0;
        tm.userData.k += (target - tm.userData.k) * Math.min(1, dt * 8);
        const tint = micOn ? micColor : brand;
        const k = Math.min(1, tm.userData.k + flash.mic * 0.25 + parts.get("mute").press.value * 0.24);
        tm.material.color.copy(white).lerp(tint, k * 0.7);
        tm.material.emissive.copy(tint).multiplyScalar(k * (micOn ? 0.9 : 0.5));
      }

      // Live input moves each paddle independently. With no input source, the effect preview
      // can simulate a pull, mirroring R2 on L2 when the effects are linked.
      const linked = state.focus === "triggers";
      if (linked || state.focus === "l2" || state.focus === "r2") {
        const left = Math.round((input?.triggers?.[0] ?? 0) * 100 / 255);
        const right = Math.round((input?.triggers?.[1] ?? 0) * 100 / 255);
        if (left !== triggerTravel[0] || right !== triggerTravel[1]) triggerTravel = [left, right];
      }
      for (const [p, section] of [["r2", "rightTrigger"], ["l2", "leftTrigger"]]) {
        const tm = trig[p];
        if (!tm) continue;
        const e = c?.[section]?.effect;
        const travel = controlTrigger(input, p);
        const press = parts.get(p).press;
        press.value = THREE.MathUtils.damp(press.value, travel, travel > press.value ? 75 : 28, dt);
        const target = !compact && lit(p) ? 1 : 0;
        tm.userData.k = THREE.MathUtils.damp(tm.userData.k, target, 9, dt);
        const lead = trig.r2;
        if (input) {
          tm.userData.hinge.rotation.x = HINGE.dir * HINGE.travel * press.value;
          Object.assign(tm.userData.sim, { pos: press.value, phase: "rest", t: 0.3, bounce: 0 });
          tm.userData.kick = 0;
        } else if (linked && p === "l2" && lead?.userData.hinge && tm.userData.hinge) {
          Object.assign(tm.userData.sim, lead.userData.sim);
          tm.userData.hinge.rotation.x = lead.userData.hinge.rotation.x;
          tm.userData.kick = lead.userData.kick;
        } else {
          animateTrigger(tm, lit(p) && !compact ? e : null, dt);
          tm.userData.kick = Math.max(0, (tm.userData.kick ?? 0) - dt / 0.35);
        }
        const accent = 0.08 * tm.userData.k + 0.11 * press.value + 0.2 * flash[p];
        tm.material.color.copy(white).lerp(brand, accent);
        tm.material.emissive.copy(brand).multiplyScalar(accent + tm.userData.kick * 0.9);
      }

      // sweep: the edge runs triggers -> grips as the controller materializes (and back)
      if (holo) {
        const wanted = state.preview
          ? previews.get(state.preview)?.ghost ?? (holoK ? shownGhost : null)
          : state.virtual ? holo : null;
        if (holoK === 0) shownGhost = wanted;
        if (!swap && holoK === 1 && wanted && wanted !== shownGhost) swap = { ghost: wanted, progress: 0 };
        if (swap) {
          // Complementary planes swap the two ghosts directly while the physical mesh stays clipped.
          swap.progress = Math.min(1, swap.progress + dt / SWEEP_S);
          const far = Math.max(shownGhost.zMax, swap.ghost.zMax) + SWEEP_MARGIN;
          const near = Math.min(shownGhost.zMin, swap.ghost.zMin) - SWEEP_MARGIN;
          const edge = THREE.MathUtils.lerp(far, near, THREE.MathUtils.smoothstep(swap.progress, 0, 1));
          clip.set(clipNormal, holo.zMin - SWEEP_MARGIN).applyMatrix4(world.matrixWorld);
          swapClip.set(clipNormal, edge).applyMatrix4(world.matrixWorld);
          if (shownGhost === holo) touchClip.copy(swapClip);
          else if (swap.ghost === holo) touchClip.copy(swapClip).negate();
          else touchClip.copy(clip);
          world.updateMatrixWorld(true);
          shownGhost.animate?.(input, dt, time);
          swap.ghost.animate?.(input, dt, time);
          shownGhost.update(outgoingClip.copy(swapClip).negate());
          swap.ghost.update(swapClip);
          shownGhost.group.visible = swap.progress < 1;
          swap.ghost.group.visible = true;
          if (swap.progress === 1) {
            shownGhost = swap.ghost;
            swap = null;
          }
        } else {
          holoK = wanted ? Math.min(1, holoK + dt / SWEEP_S) : Math.max(0, holoK - dt / SWEEP_S);
          const e = THREE.MathUtils.smoothstep(holoK, 0, 1);
          const far = Math.max(holo.zMax, shownGhost?.zMax ?? holo.zMax) + SWEEP_MARGIN;
          const near = Math.min(holo.zMin, shownGhost?.zMin ?? holo.zMin) - SWEEP_MARGIN;
          clip.set(clipNormal, THREE.MathUtils.lerp(far, near, e)).applyMatrix4(world.matrixWorld);
          if (shownGhost && shownGhost !== holo) touchClip.copy(clip);
          else touchClip.set(clipNormal, far).applyMatrix4(world.matrixWorld);
          if (shownGhost) shownGhost.group.visible = holoK > 0.001;
          if (shownGhost?.group.visible) {
            world.updateMatrixWorld(true);
            shownGhost.animate?.(input, dt, time);
            shownGhost.update(clip);
          }
        }
        floor.material.opacity = 0.32 * (1 - THREE.MathUtils.smoothstep(holoK, 0, 1));
      }

      // camera: ease towards the shot
      const k = 1 - Math.pow(0.0015, dt);
      cam.at.lerp(goal.at, k);
      cam.dir.lerp(goal.dir, k).normalize();
      cam.dist += (goal.dist - cam.dist) * k;
      camera.position.copy(cam.dir).multiplyScalar(cam.dist).add(cam.at);
      camera.lookAt(cam.at);
      renderer.render(scene, camera);

      // callout positions
      if (state.labels.length) {
        const w = host.clientWidth, h = host.clientHeight, next = {};
        for (const p of state.labels) {
          proj.set(...ANCHORS[p]).applyMatrix4(world.matrixWorld).project(camera);
          if (proj.z > 1) continue;
          // keep the tag inside the view (it is centred on its anchor)
          next[p] = [Math.min(w - 80, Math.max(80, ((proj.x + 1) / 2) * w)), ((1 - proj.y) / 2) * h];
        }
        const same = Object.keys(next).length === Object.keys(screen).length && Object.entries(next).every(([k, [x, y]]) => screen[k] && Math.abs(screen[k][0] - x) < 0.5 && Math.abs(screen[k][1] - y) < 0.5);
        if (!same) screen = next;
      }
    };
    tick();

    api = {
      setControls(input) {
        liveControls = input;
      },
      set(s) {
        if (s.preview !== state.preview) loadPreview(s.preview);
        state = s;
        if (ready && s.model !== style) applyStyle(s.model);
      },
      aim(f, z) {
        aim(SHOTS[f] ?? SHOTS.overview, z, false);
      },
      pulse(part) {
        if (part in flash && !compact) flash[part] = 1;
      },
    };
    if (import.meta.env.DEV && !compact) window.__ds3d = { THREE, camera, scene, pivot, goal, cam, holo: () => holoK, aim: (s) => aim(s, 1, true) };

    return () => {
      gone = true;
      cancelAnimationFrame(raf);
      ro.disconnect();
      io.disconnect();
      pmrem.dispose();
      for (const loaded of previews.values()) disposeGhostPreview(loaded);
      ledTex.dispose();
      haloTex.dispose();
      for (const v of maps.values()) for (const [k, tx] of Object.entries(v)) if (k !== "white") tx.dispose();
      const geometries = new Set();
      const materials = new Set();
      scene.traverse((o) => {
        if (o.geometry) geometries.add(o.geometry);
        if (o.material) for (const material of [o.material].flat()) materials.add(material);
      });
      for (const geometry of geometries) geometry.dispose();
      for (const material of materials) material.dispose();
      renderer.dispose();
      renderer.domElement.remove();
      api = null;
    };
  });

  $effect(() => {
    // read everything the frame loop needs so the effect re-runs on any change
    const s = { cfg: cfg ? $state.snapshot(cfg) : null, lights: lights ? $state.snapshot(lights) : null, focus, virtual, preview, labels: [...labels], model };
    api?.set(s);
  });
  $effect(() => {
    const input = controls ? $state.snapshot(controls) : null;
    api?.setControls(input);
  });
  $effect(() => {
    const f = compact ? "standing" : focus;
    api?.aim(f, 1);
  });
  let lastFlash = 0;
  $effect(() => {
    if (flash && flash.n !== lastFlash) {
      lastFlash = flash.n;
      api?.pulse(flash.part);
    }
  });

  // callout text
  const tm = (k) => t(`dualsense.triggers.modes.${k}`);
  function caption(part) {
    if (!cfg) return "";
    if (part === "lightbar") return cfg.lightbar.mode === "none" || cfg.lightbar.mode === "off" ? "" : t(`dualsense.lights.modes.${cfg.lightbar.mode}`);
    if (part === "l2" || part === "r2" || part === "triggers") {
      const e = cfg[part === "l2" ? "leftTrigger" : "rightTrigger"].effect;
      return e ? tm(e.kind) : "";
    }
    return "";
  }
  const zones = (part) => zonesOf(cfg?.[part === "l2" ? "leftTrigger" : "rightTrigger"]?.effect ?? null);
  // only the part being edited is tagged (the overview stays clean)
  const show = (p) => focus === p;
  const triggerView = $derived(focus === "triggers" || focus === "l2" || focus === "r2");
</script>

<div bind:this={host} class="relative size-full overflow-hidden {className}" class:preview-failed={previewFailed}>
  {#if failed || previewFailed}{@render fallback?.()}{/if}

  {#if ready && !compact}
    {#if triggerView}
      <div class="absolute inset-x-3 bottom-7 grid grid-cols-2 gap-2">
        {#each REAR_TRIGGERS as [side, i] (side)}
          <div class="bg-background/75 rounded-md border px-2.5 py-2 backdrop-blur">
            <div class="mb-1.5 flex justify-between gap-2 text-xs">
              <span class="font-medium">{t(`dualsense.triggers.${side}`)}</span>
              <span class="text-muted-foreground tabular-nums">{triggerTravel[i]}%</span>
            </div>
            <div class="bg-muted h-1 overflow-hidden rounded-full">
              <div class="bg-brand h-full rounded-full" style:width="{triggerTravel[i]}%"></div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
    {#each labels as part (part)}
      {#if screen[part] && show(part)}
        {@const [x, y] = screen[part]}
        {@const z = part === "l2" || part === "r2" || part === "triggers" ? zones(part) : null}
        <button
          type="button"
          class="callout absolute flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[10.5px] font-medium whitespace-nowrap backdrop-blur transition-[background-color,border-color,color] duration-300
            {focus === part ? 'border-brand bg-brand/25 text-foreground' : 'bg-background/65 text-muted-foreground hover:text-foreground border-white/10'}"
          style="left:{x}px; top:{y}px; transform: translate(-50%, {part === 'player' || part === 'mic' ? '18px' : triggerView ? '-250%' : '-180%'})"
          onclick={() => onselect?.(part)}
        >
          <span class="size-1.5 rounded-full {focus === part ? 'bg-brand' : 'bg-muted-foreground/50'}"></span>
          {t(`dualsense.stage.parts.${part}`)}
          {#if caption(part)}<span class="text-muted-foreground">· {caption(part)}</span>{/if}
          {#if z}
            <span class="ml-0.5 flex h-2.5 items-end gap-px">
              {#each z as v}<span class="bg-brand w-[2px] rounded-[1px]" style:height="{Math.max(12, (v / 8) * 100)}%" style:opacity={v ? 1 : 0.25}></span>{/each}
            </span>
          {/if}
        </button>
      {/if}
    {/each}
  {/if}
</div>

<style>
  .preview-failed > :global(canvas) { visibility: hidden; }
</style>
