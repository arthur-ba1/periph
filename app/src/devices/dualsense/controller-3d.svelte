<script module>
  // Optional asset (git-ignored): "Playstation 5 Dualsense" by AHarmlessPotato, CC BY 4.0,
  // https://sketchfab.com/3d-models/playstation-5-dualsense-878c1f882808477ab81c2fe86d5a3936
  // (glTF Binary download saved as assets/model.glb). See assets/NOTICE.md.
  const found = import.meta.glob("./assets/model.glb", { eager: true, query: "?url", import: "default" });
  export const MODEL_URL = Object.values(found)[0] ?? null;

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

  /** Camera per focus: what to look at, from which direction, how far. */
  export const SHOTS = {
    // `at` is a model-space point (it follows the standing pose); `dir` is the camera direction
    // in view space (+Y up, +Z towards the viewer), so every shot stays level.
    overview: { at: [0, 0, -0.01], dir: [0, 0.16, 1], dist: 0.48 },
    lightbar: { at: [0, 0.045, -0.085], dir: [0, 0.3, 1], dist: 0.3 },
    player: { at: [0, 0.058, -0.034], dir: [0, 0.3, 1], dist: 0.13 },
    mic: { at: [0, 0.047, 0.014], dir: [0, 0.3, 1], dist: 0.15 },
    // linked triggers: the same close-up as R2 (both paddles move and tint together)
    triggers: { at: [0.115, -0.01, -0.105], dir: [0.65, 0.75, 0.2], dist: 0.4 },
    l2: { at: [-0.115, -0.01, -0.105], dir: [-0.65, 0.75, 0.2], dist: 0.4 },
    r2: { at: [0.115, -0.01, -0.105], dir: [0.65, 0.75, 0.2], dist: 0.4 },
    standing: { at: [0, 0, -0.012], dir: [0, 0.12, 1], dist: 0.52 }, // sidebar thumbnail
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
  import { t } from "$lib/i18n.svelte.js";

  /**
   * cfg: driver config; focus: SHOTS key; virtual: show the emulated
   * twin; flash: { part, n } pulse; labels: parts that get a callout; onselect(part);
   * compact: sidebar thumbnail (fixed overview, no callouts).
   */
  let { cfg = null, focus = "overview", virtual = false, flash = null, labels = [], onselect = null, compact = false, fallback = null, class: className = "" } = $props();

  let host;
  let ready = $state(false);
  let failed = $state(!MODEL_URL);
  /** Screen position of each anchor (px in host), updated every frame. */
  let screen = $state({});
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

  onMount(() => {
    if (!MODEL_URL) return;
    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: "high-performance" });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.0;
    renderer.shadowMap.enabled = true;
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
    key.castShadow = true;
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
    world.rotation.x = compact ? Math.PI / 2 : Math.PI / 2 - 0.32;
    world.rotation.y = compact ? 0 : -0.2;
    scene.add(world);
    world.updateMatrixWorld(true);

    const floor = new THREE.Mesh(new THREE.PlaneGeometry(1.2, 1.2), new THREE.ShadowMaterial({ opacity: 0.32 }));
    floor.rotation.x = -Math.PI / 2;
    floor.position.y = -0.13;
    floor.receiveShadow = true;
    scene.add(floor);

    // glTF from Collada: the controller's up is -Z and its triggers +Y; turn it to +Y up,
    // triggers towards -Z (away from the default camera).
    const pivot = new THREE.Group();
    const upright = new THREE.Group();
    upright.rotation.x = Math.PI / 2;
    pivot.rotation.y = Math.PI;
    pivot.add(upright);
    world.add(pivot);
    const ghost = new THREE.Group(); // virtual twin
    world.add(ghost);

    const v3 = (a) => new THREE.Vector3(...a);
    const brand = cssColor("--brand", "#3291ff");

    let lbMat = null; // the model material whose emissive map is the lightbar

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

    // triggers: their own meshes, cut out of the model on load and tinted with the brand colour
    const trig = { l2: null, r2: null, mic: null };
    const white = new THREE.Color(0xffffff);
    const micColor = new THREE.Color(0xff7a1a);

    /** The L2/R2 paddles are separate pieces inside the dark-parts mesh: find them (connected
     *  pieces behind the shoulders, below the bumpers) and move their triangles to new meshes. */
    function splitTriggers(root) {
      world.updateMatrixWorld(true);
      const toModel = new THREE.Matrix4().copy(world.matrixWorld).invert();
      const v = new THREE.Vector3();
      const meshes = [];
      root.traverse((o) => o.isMesh && o.geometry.index && meshes.push(o));
      for (const o of meshes) {
        const g = o.geometry, pos = g.attributes.position, idx = g.index.array;
        const m = new THREE.Matrix4().multiplyMatrices(toModel, o.matrixWorld);
        // weld split vertices (UV seams) so a piece stays one component
        const keys = new Map(), weld = new Int32Array(pos.count), pts = new Float32Array(pos.count * 3);
        for (let i = 0; i < pos.count; i++) {
          v.fromBufferAttribute(pos, i).applyMatrix4(m);
          pts[i * 3] = v.x; pts[i * 3 + 1] = v.y; pts[i * 3 + 2] = v.z;
          const k = `${Math.round(v.x * 1e5)},${Math.round(v.y * 1e5)},${Math.round(v.z * 1e5)}`;
          if (!keys.has(k)) keys.set(k, keys.size);
          weld[i] = keys.get(k);
        }
        const parent = Int32Array.from({ length: keys.size }, (_, i) => i);
        const find = (x) => { while (parent[x] !== x) x = parent[x] = parent[parent[x]]; return x; };
        for (let t = 0; t < idx.length; t += 3) {
          const a = find(weld[idx[t]]);
          parent[find(weld[idx[t + 1]])] = a;
          parent[find(weld[idx[t + 2]])] = a;
        }
        const box = new Map();
        for (let i = 0; i < pos.count; i++) {
          const r = find(weld[i]);
          let b = box.get(r);
          if (!b) box.set(r, (b = { n: 0, min: [9, 9, 9], max: [-9, -9, -9] }));
          b.n++;
          for (let k = 0; k < 3; k++) { b.min[k] = Math.min(b.min[k], pts[i * 3 + k]); b.max[k] = Math.max(b.max[k], pts[i * 3 + k]); }
        }
        const side = new Map(), sideBox = {};
        for (const [r, b] of box) {
          const cx = (b.min[0] + b.max[0]) / 2, cz = (b.min[2] + b.max[2]) / 2;
          // mic mute button: the small pill under the PS button
          if (b.n > 200 && Math.abs(cx) < 0.004 && cz > 0.012 && cz < 0.03 && b.max[0] - b.min[0] < 0.03) side.set(r, "mic");
          if (b.n < 100 || b.max[1] > 0.01 || b.min[2] > -0.11) continue;
          const t = b.min[0] > 0.05 ? "r2" : b.max[0] < -0.05 ? "l2" : null;
          if (t) (side.set(r, t), (sideBox[t] = b));
        }
        if (!side.size) continue;
        const keep = [], cut = { l2: [], r2: [], mic: [] };
        for (let t = 0; t < idx.length; t += 3) {
          const s = side.get(find(weld[idx[t]]));
          (s ? cut[s] : keep).push(idx[t], idx[t + 1], idx[t + 2]);
        }
        g.setIndex(keep);
        for (const p of ["l2", "r2", "mic"]) {
          if (!cut[p].length) continue;
          const sg = new THREE.BufferGeometry();
          for (const [name, attr] of Object.entries(g.attributes)) sg.setAttribute(name, attr);
          sg.setIndex(cut[p]);
          const mat = o.material.clone();
          mat.emissive = new THREE.Color(0x000000);
          mat.emissiveMap = null;
          const mesh = new THREE.Mesh(sg, mat);
          mesh.castShadow = true;
          mesh.userData.k = 0;
          o.add(mesh); // child: same transform as the piece it came from
          trig[p] = mesh;
          if (sideBox[p]) {
            // hinge along X at the paddle's top-back edge, so it can be pressed
            const b = sideBox[p];
            const hinge = new THREE.Group();
            hinge.position.set((b.min[0] + b.max[0]) / 2, HINGE.y(b), HINGE.z(b));
            world.add(hinge);
            world.updateMatrixWorld(true);
            hinge.attach(mesh);
            mesh.userData.hinge = hinge;
            mesh.userData.sim = { pos: 0, phase: "rest", t: 0.3, bounce: 0 };
            mesh.userData.kick = 0;
          }
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
    const look = new THREE.Vector3();
    const goal = { at: new THREE.Vector3(), pos: new THREE.Vector3(), shot: compact ? SHOTS.standing : SHOTS.overview, k: 1 };
    function aim(shot, k, snap) {
      goal.shot = shot;
      goal.k = k;
      const fit = Math.max(1, 1.6 / camera.aspect);
      goal.at.copy(v3(shot.at)).applyMatrix4(world.matrixWorld);
      goal.pos.copy(v3(shot.dir).normalize().multiplyScalar((shot.dist * fit) / k)).add(goal.at);
      if (snap) {
        look.copy(goal.at);
        camera.position.copy(goal.pos);
      }
    }

    new GLTFLoader().load(
      MODEL_URL,
      (gltf) => {
        gltf.scene.traverse((o) => {
          if (!o.isMesh) return;
          o.castShadow = true;
          const m = o.material;
          if (m.emissiveMap) {
            // emissive texture = lightbar mask (blue); make it white so `emissive` sets the hue
            const img = m.emissiveMap.image;
            const c = document.createElement("canvas");
            c.width = img.width;
            c.height = img.height;
            const g = c.getContext("2d");
            g.drawImage(img, 0, 0);
            const d = g.getImageData(0, 0, c.width, c.height);
            for (let i = 0; i < d.data.length; i += 4) {
              const v = Math.max(d.data[i], d.data[i + 1], d.data[i + 2]) > 60 ? 255 : 0;
              d.data[i] = d.data[i + 1] = d.data[i + 2] = v;
            }
            g.putImageData(d, 0, 0);
            const tx = new THREE.CanvasTexture(c);
            tx.flipY = m.emissiveMap.flipY;
            tx.colorSpace = THREE.SRGBColorSpace;
            m.emissiveMap = tx;
            m.emissive = new THREE.Color(0x000000);
            lbMat = m;
          }
          // the export marks opaque materials as BLEND; that breaks depth sorting
          m.transparent = false;
          m.alphaMode = "OPAQUE";
          m.depthWrite = true;
        });
        upright.add(gltf.scene);
        splitTriggers(gltf.scene);
        placeLeds();
        // floor right under the standing controller
        floor.position.y = new THREE.Box3().setFromObject(pivot).min.y - 0.001;

        // virtual twin: same shape, flat tinted shell behind the real one
        const twin = gltf.scene.clone(true);
        const tint = new THREE.MeshBasicMaterial({ color: cssColor("--success", "#3ecf8e"), transparent: true, opacity: 0.09, depthWrite: false, blending: THREE.AdditiveBlending });
        twin.traverse((o) => o.isMesh && ((o.material = tint), (o.castShadow = false)));
        const tw = new THREE.Group();
        tw.rotation.x = Math.PI / 2;
        tw.add(twin);
        ghost.add(tw);
        ghost.rotation.y = Math.PI;
        ghost.position.set(0.06, 0.03, -0.07);
        ghost.visible = false;
        ready = true;
        renderer.domElement.style.opacity = "1";
      },
      undefined,
      () => (failed = true),
    );

    const resize = () => {
      const w = host.clientWidth || 1, h = host.clientHeight || 1;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      aim(goal.shot, goal.k, !ready);
    };
    const ro = new ResizeObserver(resize);
    ro.observe(host);
    resize();

    let visible = true;
    const io = new IntersectionObserver(([e]) => (visible = e.isIntersecting));
    io.observe(host);

    // ---- live state (pushed by the $effects below) ----
    let state = { cfg: null, focus: "overview", virtual: false, labels: [] };
    const lbColor = new THREE.Color(), tmp = new THREE.Color(), proj = new THREE.Vector3();

    let raf, time = 0, last = performance.now();
    const tick = (now = performance.now()) => {
      raf = requestAnimationFrame(tick);
      const dt = Math.min((now - last) / 1000, 0.05);
      last = now;
      time += dt;
      if (!visible || document.hidden) return;
      const c = state.cfg;
      const lit = (p) => state.focus === p || (state.focus === "triggers" && (p === "l2" || p === "r2"));

      // lightbar
      const lb = c?.lightbar;
      let lbK = 0;
      if (lb && lb.mode !== "none") {
        const period = (11 - lb.speed) * 0.35;
        const ph = (time % period) / period;
        lbK = lb.brightness / 100;
        if (lb.mode === "breathing") lbK *= 0.2 + 0.8 * (0.5 - 0.5 * Math.cos(ph * Math.PI * 2));
        if (lb.mode === "strobe") lbK *= ph < 0.5 ? 1 : 0.05;
        if (lb.mode === "rainbow") lbColor.setHSL((time / (period * 3)) % 1, 1, 0.5);
        else lbColor.setRGB(lb.color[0] / 255, lb.color[1] / 255, lb.color[2] / 255, THREE.SRGBColorSpace);
      }
      for (const p in flash) flash[p] = Math.max(0, flash[p] - dt / 0.7);
      if (lbMat) lbMat.emissive.copy(lbColor).multiplyScalar(lbK * 2.2 * (1 + flash.lightbar * 0.3));

      // player LEDs
      const pl = c?.playerLeds;
      leds.forEach((m, i) => {
        const on = !!pl?.enabled && ((pl.mask >> i) & 1) === 1;
        // lit: white at the chosen level; unlit: dark like the real strip (a faint trace of the
        // lens while editing, so you can see where it is)
        m.material.color.set(0xffffff);
        m.material.opacity = on ? LED_ALPHA[pl.brightness] : compact ? 0 : Math.max(lit("player") ? 0.07 : 0, flash.player * 0.15);
        m.userData.halo.material.opacity = on ? LED_ALPHA[pl.brightness] * 0.85 : 0;
      });

      // mic button: orange while the LED is on, accent while it is being edited
      const micOn = !!c?.micLed?.enabled && c.micLed.on;
      if (trig.mic) {
        const tm = trig.mic;
        const target = micOn ? 1 : !compact && lit("mic") ? 0.6 : 0;
        tm.userData.k += (target - tm.userData.k) * Math.min(1, dt * 8);
        const tint = micOn ? micColor : brand;
        const k = Math.min(1, tm.userData.k + flash.mic * 0.25);
        tm.material.color.copy(white).lerp(tint, k * 0.7);
        tm.material.emissive.copy(tint).multiplyScalar(k * (micOn ? 0.9 : 0.5));
      }

      // triggers. Linked (focus "triggers"): R2 runs the press simulation and L2 mirrors it
      // frame by frame, so both paddles move and flash as one.
      const linked = state.focus === "triggers";
      for (const [p, section] of [["r2", "rightTrigger"], ["l2", "leftTrigger"]]) {
        const tm = trig[p];
        if (!tm) continue;
        const e = c?.[section]?.effect;
        // accent only on the trigger being edited (both when linked); never in the thumbnail
        const target = !compact && lit(p) ? 1 : 0;
        tm.userData.k += (target - tm.userData.k) * Math.min(1, dt * 6);
        const lead = trig.r2;
        if (linked && p === "l2" && lead?.userData.hinge && tm.userData.hinge) {
          Object.assign(tm.userData.sim, lead.userData.sim);
          tm.userData.hinge.rotation.x = lead.userData.hinge.rotation.x;
          tm.userData.kick = lead.userData.kick;
        } else {
          animateTrigger(tm, lit(p) && !compact ? e : null, dt);
          tm.userData.kick = Math.max(0, (tm.userData.kick ?? 0) - dt / 0.35);
        }
        const k = Math.min(1, tm.userData.k + flash[p] * 0.25);
        tm.material.color.copy(white).lerp(brand, k * 0.7);
        tm.material.emissive.copy(brand).multiplyScalar(k * 0.55 + tm.userData.kick * 0.9);
      }

      ghost.visible = state.virtual && ready;

      // camera: ease towards the shot
      const k = 1 - Math.pow(0.0015, dt);
      look.lerp(goal.at, k);
      camera.position.lerp(goal.pos, k);
      camera.lookAt(look);
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
      set(s) {
        state = s;
      },
      aim(f, z) {
        aim(SHOTS[f] ?? SHOTS.overview, z, false);
      },
      pulse(part) {
        if (part in flash && !compact) flash[part] = 1;
      },
    };
    if (import.meta.env.DEV && !compact) window.__ds3d = { THREE, camera, scene, pivot, goal, aim: (s) => aim(s, 1, true) };

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      io.disconnect();
      pmrem.dispose();
      ledTex.dispose();
      haloTex.dispose();
      scene.traverse((o) => {
        o.geometry?.dispose?.();
        if (o.material) [o.material].flat().forEach((m) => m.dispose());
      });
      renderer.dispose();
      renderer.domElement.remove();
      api = null;
    };
  });

  $effect(() => {
    // read everything the frame loop needs so the effect re-runs on any change
    const s = { cfg: cfg ? $state.snapshot(cfg) : null, focus, virtual, labels: [...labels] };
    api?.set(s);
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
    if (part === "lightbar") return cfg.lightbar.mode === "none" ? "" : t(`dualsense.lights.modes.${cfg.lightbar.mode}`);
    if (part === "l2" || part === "r2" || part === "triggers") {
      const e = cfg[part === "l2" ? "leftTrigger" : "rightTrigger"].effect;
      return e ? tm(e.kind) : "";
    }
    return "";
  }
  const zones = (part) => zonesOf(cfg?.[part === "l2" ? "leftTrigger" : "rightTrigger"]?.effect ?? null);
  // only the part being edited is tagged (the overview stays clean)
  const show = (p) => focus === p;
</script>

<div bind:this={host} class="relative size-full overflow-hidden {className}">
  {#if failed}{@render fallback?.()}{/if}

  {#if ready && !compact}
    {#each labels as part (part)}
      {#if screen[part] && show(part)}
        {@const [x, y] = screen[part]}
        {@const z = part === "l2" || part === "r2" || part === "triggers" ? zones(part) : null}
        <button
          type="button"
          class="callout absolute flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[10.5px] font-medium whitespace-nowrap backdrop-blur transition-[background-color,border-color,color] duration-300
            {focus === part ? 'border-brand bg-brand/25 text-foreground' : 'bg-background/65 text-muted-foreground hover:text-foreground border-white/10'}"
          style="left:{x}px; top:{y}px; transform: translate(-50%, {part === 'player' || part === 'mic' ? '18px' : '-180%'})"
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
