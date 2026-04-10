import { proxyRequest } from "h3";

export default defineEventHandler(async (event) => {
    const path = getRequestURL(event).pathname;
    if (!path.startsWith("/core/")) {
        return;
    }

    const config = useRuntimeConfig(event);
    const base = config.coreApiUrl.replace(/\/$/, "");
    const sub = path.slice("/core".length) || "/";
    const target = `${base}${sub}${getRequestURL(event).search}`;

    return proxyRequest(event, target);
});
