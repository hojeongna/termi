import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import path from "path";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [svelte()],

  resolve: {
    alias: {
      $lib: path.resolve("./src/lib"),
    },
  },

  test: {
    resolve: {
      conditions: ['browser']
    }
  },

  clearScreen: false,
  server: {
    port: parseInt(process.env.VITE_DEV_PORT || '1420', 10),
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: parseInt(process.env.VITE_HMR_PORT || '1421', 10),
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
