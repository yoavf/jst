import { defineConfig } from "vite";

export default defineConfig({
  base: "/assets/",
  build: {
    emptyOutDir: false,
    lib: {
      entry: {
        "demo-cli": "site/demo-cli.js",
        "demo-command-v6": "site/demo-command.js",
        "demo-runtime": "site/demo-runtime.js",
        "demo-sandbox": "site/demo-sandbox.js",
      },
      formats: ["es"],
    },
    rollupOptions: {
      output: {
        entryFileNames: "[name].js",
      },
    },
    modulePreload: {
      polyfill: false,
    },
    outDir: "docs/assets",
    sourcemap: true,
  },
  server: {
    host: true,
    port: 3000,
    allowedHosts: true,
    headers: {
      "Cross-Origin-Embedder-Policy": "require-corp",
      "Cross-Origin-Opener-Policy": "same-origin",
    },
    proxy: {
      "/api/jst-demo": {
        target: "http://server:8080",
        changeOrigin: true,
        headers: {
          origin: "https://jst.sh",
        },
        rewrite: () => "/demo",
      },
      "/api/jst-status": {
        target: "http://server:8080",
        changeOrigin: true,
        rewrite: () => "/status",
      },
    },
  },
});
