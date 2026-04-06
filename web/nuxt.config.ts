import tailwindcss from "@tailwindcss/vite";

export default defineNuxtConfig({
    compatibilityDate: "2025-07-15",
    devtools: { enabled: true },
    runtimeConfig: {
        coreApiUrl: process.env.NUXT_CORE_API_URL || "http://127.0.0.1:40000",
    },
    modules: ["@nuxt/fonts"],
    css: ["~/assets/styles/main.css"],
    vite: {
        plugins: [tailwindcss()],
        server: {
            proxy: {
                "/core": {
                    target: process.env.NUXT_CORE_API_URL || "http://127.0.0.1:40000",
                    changeOrigin: true,
                    rewrite: (path) => path.replace(/^\/core/, "") || "/",
                },
            },
        },
    },
    fonts: {
        families: [
            {
                name: "Inter",
                provider: "google",
                weights: [400, 500, 600, 700],
            },
        ],
    },
});
