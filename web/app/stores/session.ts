import { defineStore } from "pinia";
import type { ManagedSessionResponse } from "~/types/core";
import { getCoreErrorMessage } from "~/utils/coreErrors";

export const useSessionStore = defineStore("coreSession", () => {
    const devices = ref<ManagedSessionResponse[]>([]);
    const loading = ref(false);
    const actionError = ref("");

    async function load() {
        const api = useCoreApi();
        loading.value = true;
        actionError.value = "";
        try {
            const { sessions } = await api.listSessions();
            devices.value = sessions;
        } catch (e: unknown) {
            actionError.value = getCoreErrorMessage(e, "Could not load sessions");
            devices.value = [];
        } finally {
            loading.value = false;
        }
    }

    function clear() {
        devices.value = [];
        actionError.value = "";
    }

    async function revokeOne(sessionId: string) {
        const api = useCoreApi();
        actionError.value = "";
        try {
            await api.closeSession(sessionId);
            await load();
        } catch (e: unknown) {
            actionError.value = getCoreErrorMessage(e, "Could not revoke session");
        }
    }

    async function revokeOthers() {
        const api = useCoreApi();
        actionError.value = "";
        try {
            const { closed_count } = await api.closeOtherSessions();
            await load();
            return closed_count;
        } catch (e: unknown) {
            actionError.value = getCoreErrorMessage(
                e,
                "Could not sign out other devices",
            );
            return null;
        }
    }

    return { devices, loading, actionError, load, clear, revokeOne, revokeOthers };
});
