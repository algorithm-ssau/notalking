import tailwindcss from "@tailwindcss/vite";

export default defineNuxtConfig({
    compatibilityDate: "2025-07-15",
    devtools: { enabled: true },
    app: {
        head: {
            link: [
                {
                    rel: "preconnect",
                    href: "https://fonts.googleapis.com",
                },
                {
                    rel: "preconnect",
                    href: "https://fonts.gstatic.com",
                    crossorigin: "",
                },
                {
                    rel: "stylesheet",
                    href: "https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap",
                },
            ],
        },
    },
    runtimeConfig: {
        coreApiUrl: process.env.NUXT_CORE_API_URL || "http://127.0.0.1:40000",
    },
    modules: ["@pinia/nuxt"],
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
});
