// Re-run a layout measurement when the layout can have changed, instead of on every frame:
// on resize of the observed elements, on scroll, after clicks inside the scroller (sections that
// open or close) and for a short burst of frames after transitions (`burst()`).
// Reading layout every frame forces style and layout work even while nothing moves.

export function followLayout(measure, { scroller = null } = {}) {
  let raf = 0, until = 0, gone = false;
  const frame = (now) => {
    raf = 0;
    if (gone) return;
    measure();
    if (now < until) raf = requestAnimationFrame(frame);
  };
  /** Measure on every frame for `ms` (0: just the next frame). */
  const burst = (ms = 900) => {
    until = Math.max(until, performance.now() + ms);
    if (!raf && !gone) raf = requestAnimationFrame(frame);
  };
  const next = () => burst(0);
  const afterClick = () => burst(600);

  const ro = new ResizeObserver(next);
  scroller?.addEventListener("scroll", next, { passive: true });
  scroller?.addEventListener("click", afterClick, true);
  window.addEventListener("resize", next);
  burst();

  return {
    burst,
    /** Elements whose size changes move what is measured (replaces the previous set). */
    observe(elements) {
      ro.disconnect();
      for (const el of elements) if (el) ro.observe(el);
    },
    destroy() {
      gone = true;
      cancelAnimationFrame(raf);
      ro.disconnect();
      scroller?.removeEventListener("scroll", next);
      scroller?.removeEventListener("click", afterClick, true);
      window.removeEventListener("resize", next);
    },
  };
}
