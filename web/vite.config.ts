import { defineConfig } from "vite";

export default defineConfig({
  server: {
    fs: { allow: [".."] },
    proxy: {
      "/api": process.env.OPENCANVAS_API_URL ?? "http://127.0.0.1:3847",
    },
  },
});
