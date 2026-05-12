import { proxyRequest, type H3Event } from "h3";

export default defineEventHandler(async (event) => {
    const path = getRequestURL(event).pathname;
    const target = proxyTarget(event, path);
    if (!target) {
        return;
    }

    return proxyRequest(event, target);
});

function proxyTarget(event: H3Event, path: string): string | null {
    const config = useRuntimeConfig(event);
    const currentUrl = getRequestURL(event);

    if (path === "/core" || path.startsWith("/core/")) {
        return buildTarget(config.coreApiUrl, path, "/core", currentUrl.search);
    }

    if (path === "/intel" || path.startsWith("/intel/")) {
        return buildTarget(
            config.intelligenceApiUrl,
            path,
            "/intel",
            currentUrl.search,
        );
    }

    return null;
}

function buildTarget(
    baseUrl: string,
    path: string,
    prefix: string,
    search: string,
): string {
    const base = baseUrl.replace(/\/$/, "");
    const sub = path.slice(prefix.length) || "/";
    return `${base}${sub}${search}`;
}
