<script>
  import { onMount } from "svelte";
  import * as THREE from "three";
  import { DEMO } from "$lib/device-state.svelte.js";
  import { browserControls } from "./controller-input.js";
  import { disposeGhostPreview, loadGhostPreview } from "./load-ghost-preview.js";

  let { model, asset, controls = null, fallback } = $props();
  let host;
  let failed = $state(false);

  onMount(() => {
    if (!asset.url) {
      failed = true;
      return;
    }
    let renderer;
    try {
      renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true });
    } catch {
      failed = true;
      return;
    }
    renderer.localClippingEnabled = true;
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.domElement.className = "absolute inset-0 block size-full";
    host.prepend(renderer.domElement);

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(30);
    let preview = null;
    let gone = false;
    let frame, last = performance.now(), time = 0;
    const clip = new THREE.Plane(new THREE.Vector3(0, 0, -1));
    function resize() {
      if (!preview || !host.clientWidth || !host.clientHeight) return;
      const { size } = preview;
      renderer.setSize(host.clientWidth, host.clientHeight, false);
      camera.aspect = host.clientWidth / host.clientHeight;
      const height = Math.max(size.y, size.x / camera.aspect);
      const distance = height / (2 * Math.tan(Math.PI / 12) * 0.88) + size.z / 2;
      camera.position.z = distance;
      camera.near = distance / 100;
      camera.far = distance * 4;
      camera.updateProjectionMatrix();
    }
    const observer = new ResizeObserver(resize);
    observer.observe(host);
    function render(now) {
      frame = requestAnimationFrame(render);
      const dt = Math.min((now - last) / 1000, 0.05);
      last = now;
      if (!preview || document.hidden) return;
      time += dt;
      preview.ghost.animate(controls ?? (DEMO ? browserControls() : null), dt, time);
      preview.ghost.update(clip);
      renderer.render(scene, camera);
    }

    loadGhostPreview(asset, model, new THREE.Color("#3ecf8e")).then((loaded) => {
      if (gone) return disposeGhostPreview(loaded);
      preview = loaded;
      const { ghost } = preview;
      clip.constant = ghost.zMin - 1;
      ghost.group.visible = true;
      scene.add(ghost.group);
      resize();
      frame = requestAnimationFrame(render);
    }).catch(() => { if (!gone) failed = true; });

    return () => {
      gone = true;
      cancelAnimationFrame(frame);
      observer.disconnect();
      if (preview) disposeGhostPreview(preview);
      renderer.dispose();
      renderer.forceContextLoss();
      renderer.domElement.remove();
    };
  });
</script>

<div bind:this={host} class="relative size-full overflow-hidden">
  {#if failed}{@render fallback?.()}{/if}
</div>
