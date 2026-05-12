const fetchOpts = { credentials: "include" as const };

/**
 * Intelligence service reachability (GET /health on the configured origin or dev proxy).
 */
export function useIntelligenceStatus() {
    const config = useRuntimeConfig();
    const intelligenceBaseUrl = computed(() => {
        const b = (config.public.intelligenceApiUrl as string) || "/intel";
        return b.endsWith("/") ? b.slice(0, -1) : b;
    });

    const offline = ref(true);
    const checking = ref(false);

    async function refresh() {
        if (!import.meta.client) {
            return;
        }
        checking.value = true;
        try {
            const r = await fetch(`${intelligenceBaseUrl.value}/health`, fetchOpts);
            offline.value = !r.ok;
        } catch {
            offline.value = true;
        } finally {
            checking.value = false;
        }
    }

    onMounted(() => {
        void refresh();
    });

    return { offline, checking, refresh };
}
