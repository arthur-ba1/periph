import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

export default defineConfig({
  plugins: [
    tailwindcss(),
    svelte({
      // SVG hotspots are mouse-only by design; mirrored by the button list for keyboard users.
      onwarn: (w, handler) => (w.code.startsWith("a11y_") ? undefined : handler(w)),
    }),
  ],
  resolve: { alias: { $lib: path.resolve("./src/lib") } },
  clearScreen: false,
  // fs.allow: the demo can load the example device packages in ../packages (?demo&packages)
  server: { port: 5173, strictPort: true, fs: { allow: [".."] } },
  build: { target: "es2022", outDir: "dist", emptyOutDir: true },
});
