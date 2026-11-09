import { defineConfig } from "vite";
import { sceneFingerprint } from "./sceneFingerprint.ts";

export default defineConfig(({ command }) => ({
  // Copied or linked dependencies must share the app renderer's hook dispatcher.
  resolve: { dedupe: ["react", "react-dom"] },
  // HMR can replace algorithms after startup, so persistence is production-build only.
  define: {
    __SCENE_BUILD_FINGERPRINT__: JSON.stringify(
      command === "build" ? sceneFingerprint() : "",
    ),
  },
  server: {
    fs: { allow: [".."] },
    proxy: {
      "/api": {
        target: process.env.OPENCANVAS_API_URL ?? "http://127.0.0.1:3847",
        changeOrigin: true,
        configure(proxy) {
          proxy.on("proxyReq", (outgoing, incoming) => {
            // Preserve cross-origin rejection; translate only this dev server's own origin.
            if (incoming.headers.origin === `http://${incoming.headers.host}`) {
              outgoing.setHeader(
                "Origin",
                process.env.OPENCANVAS_API_URL ?? "http://127.0.0.1:3847",
              );
            }
          });
        },
      },
    },
  },
}));
