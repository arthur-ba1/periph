<script module>
  // Optional asset: generated locally by ml/gen_shape.py + ml/post.py (Hunyuan3D-2 geometry).
  const found = import.meta.glob("../assets/model.glb", { eager: true, query: "?url", import: "default" });
  export const MODEL_URL = Object.values(found)[0] ?? null;
</script>

<script>
  import { onMount } from "svelte";
  import { t } from "$lib/i18n.svelte.js";
  import { isVisible, onVisible } from "$lib/visibility.js";
  import * as THREE from "three";
  import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
  import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
  import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";

  /**
   * color: hex of the selected VT2 color variant (device.json rectColor)
   * autoRotate: slow idle turntable; interactive: drag to orbit
   */
  let { color = "#FF2558", autoRotate = true, interactive = true, speed = 0.6, zoom = 1, class: className = "" } = $props();

  let host;
  let ready = $state(false);
  let failed = $state(false);
  let api = null; // { setColor(hex), setAuto(bool), setInteractive(bool) }

  const paint = (hex) => {
    const c = new THREE.Color(hex);
    const max = Math.max(c.r, c.g, c.b);
    return max > 0.72 ? c.multiplyScalar(0.72 / max) : c;
  };

  onMount(() => {
    if (!MODEL_URL) {
      failed = true;
      return;
    }
    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: "high-performance" });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.05;
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFShadowMap; // PCFSoft was removed in current three
    // canvas must not size its host (setSize(..., false) leaves CSS alone): fill it absolutely,
    // otherwise the ResizeObserver below feeds back and the canvas grows without bound
    Object.assign(renderer.domElement.style, { position: "absolute", inset: "0", width: "100%", height: "100%", display: "block" });
    host.appendChild(renderer.domElement);

    const scene = new THREE.Scene();
    const pmrem = new THREE.PMREMGenerator(renderer);
    scene.environment = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;

    const camera = new THREE.PerspectiveCamera(30, 1, 0.01, 10);
    camera.position.set(0.19, 0.17, 0.27).multiplyScalar(1 / zoom); // zoom > 1 frames tighter (small slots)

    // key light with soft shadow on an invisible catcher plane
    const key = new THREE.DirectionalLight(0xffffff, 1.6);
    key.position.set(0.25, 0.5, 0.2);
    key.castShadow = true;
    key.shadow.mapSize.set(1024, 1024);
    key.shadow.radius = 6;
    Object.assign(key.shadow.camera, { left: -0.15, right: 0.15, top: 0.15, bottom: -0.15, near: 0.1, far: 1.2 });
    scene.add(key);
    const rim = new THREE.DirectionalLight(0x9fb7ff, 0.6);
    rim.position.set(-0.3, 0.15, -0.3);
    scene.add(rim);

    const floor = new THREE.Mesh(new THREE.PlaneGeometry(0.6, 0.6), new THREE.ShadowMaterial({ opacity: 0.35 }));
    floor.rotation.x = -Math.PI / 2;
    floor.receiveShadow = true;
    scene.add(floor);

    const material = new THREE.MeshPhysicalMaterial({
      color: paint(color),
      roughness: 0.42,
      metalness: 0,
      clearcoat: 0.35,
      clearcoatRoughness: 0.45,
      sheen: 0.2,
    });
    const target = material.color.clone();
    // a pure white albedo blows out under the key light + env map; cap it so the shape still reads

    const pivot = new THREE.Group();
    scene.add(pivot);

    const controls = new OrbitControls(camera, renderer.domElement);
    controls.target.set(0, 0.018, 0);
    controls.enablePan = false;
    controls.enableZoom = false;
    controls.enableDamping = true;
    controls.dampingFactor = 0.08;
    controls.minPolarAngle = 0.25;
    controls.maxPolarAngle = Math.PI / 2 - 0.08;
    controls.autoRotate = autoRotate;
    controls.autoRotateSpeed = speed;
    controls.enabled = interactive;
    controls.update();

    // intro: model swings in and settles
    let introT = 0;
    new GLTFLoader().load(
      MODEL_URL,
      (gltf) => {
        gltf.scene.traverse((o) => {
          if (o.isMesh) {
            o.material = material;
            o.castShadow = true;
            o.geometry.computeVertexNormals();
          }
        });
        pivot.add(gltf.scene);
        ready = true;
        wake();
      },
      undefined,
      () => (failed = true),
    );

    const resize = () => {
      const w = host.clientWidth || 1, h = host.clientHeight || 1;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    };
    const ro = new ResizeObserver(() => {
      resize();
      wake();
    });
    ro.observe(host);
    resize();

    let visible = true;
    const io = new IntersectionObserver(([e]) => {
      visible = e.isIntersecting;
      if (visible) wake();
    });
    io.observe(host);
    const offVisible = onVisible(() => wake());

    // Frames only while something moves: the turntable, the intro, a drag (and its damping) or
    // a colour fade. Otherwise the loop stops until wake().
    const QUIET_MS = 2000;
    const clock = new THREE.Clock();
    let raf = 0, activeUntil = 0, gone = false;
    function wake() {
      activeUntil = performance.now() + QUIET_MS;
      if (!raf && !gone) {
        clock.getDelta();
        raf = requestAnimationFrame(tick);
      }
    }
    controls.addEventListener("change", wake);
    controls.addEventListener("start", wake);
    const tick = (now = performance.now()) => {
      raf = 0;
      if (gone || !visible || !isVisible()) return;
      if (!controls.autoRotate && now >= activeUntil) return;
      raf = requestAnimationFrame(tick);
      const dt = Math.min(clock.getDelta(), 0.05);
      if (ready && introT < 1) {
        introT = Math.min(1, introT + dt / 1.3);
        const e = 1 - Math.pow(1 - introT, 4);
        pivot.rotation.y = (1 - e) * -1.1;
        pivot.position.y = (1 - e) * 0.03;
        pivot.scale.setScalar(0.85 + 0.15 * e);
      }
      material.color.lerp(target, 1 - Math.pow(0.001, dt)); // smooth color change
      controls.update();
      renderer.render(scene, camera);
    };
    wake();

    api = {
      setColor: (hex) => (target.copy(paint(hex)), wake()),
      setAuto: (on) => ((controls.autoRotate = on), wake()),
      setInteractive: (on) => (controls.enabled = on),
    };

    return () => {
      gone = true;
      cancelAnimationFrame(raf);
      offVisible();
      ro.disconnect();
      io.disconnect();
      controls.dispose();
      pmrem.dispose();
      scene.traverse((o) => o.geometry?.dispose?.());
      material.dispose();
      renderer.dispose();
      renderer.domElement.remove();
      api = null;
    };
  });

  $effect(() => api?.setColor(color));
  $effect(() => api?.setAuto(autoRotate));
  $effect(() => api?.setInteractive(interactive));
</script>

<div bind:this={host} class="relative size-full {interactive ? 'cursor-grab active:cursor-grabbing' : ''} {className}" class:opacity-0={!ready} style="transition: opacity 500ms ease">
  {#if failed}
    <div class="text-muted-foreground absolute inset-0 grid place-items-center text-xs">{t("vt2.model3dUnavailable")}</div>
  {/if}
</div>
