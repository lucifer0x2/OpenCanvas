import { defineConfig } from "vite";

export default defineConfig({
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
});
