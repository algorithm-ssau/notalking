import tailwindcss from "@tailwindcss/vite";

export default defineNuxtConfig({
    compatibilityDate: "2025-07-15",
    devtools: { enabled: true },
    app: {
        head: {
            title: "Notalking",
            meta: [
                {
                    name: "description",
                    content: "Structured notes with optional AI context.",
                },
            ],
        },
    },
    runtimeConfig: {
        coreApiUrl: process.env.NUXT_CORE_API_URL || "http://127.0.0.1:40000",
        intelligenceApiUrl:
            process.env.NUXT_INTELLIGENCE_API_URL ||
            process.env.NUXT_PUBLIC_INTELLIGENCE_PROXY_TARGET ||
            "http://127.0.0.1:41000",
        public: {
            intelligenceApiUrl:
                process.env.NUXT_PUBLIC_INTELLIGENCE_API_URL || "/intel",
        },
    },
    modules: ["@pinia/nuxt"],
    css: ["~/assets/styles/main.css"],
    vite: {
        plugins: [tailwindcss()],
        server: {
            proxy: {
                "/core": {
                    target:
                        process.env.NUXT_CORE_API_URL ||
                        "http://127.0.0.1:40000",
                    changeOrigin: true,
                    rewrite: (path) => path.replace(/^\/core/, "") || "/",
                },
                "/intel": {
                    target:
                        process.env.NUXT_PUBLIC_INTELLIGENCE_PROXY_TARGET ||
                        "http://127.0.0.1:41000",
                    changeOrigin: true,
                    rewrite: (path) => path.replace(/^\/intel/, "") || "/",
                },
            },
        },
    },
});
