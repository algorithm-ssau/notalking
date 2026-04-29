<template>
    <svg
        :width="size"
        :height="size"
        viewBox="0 0 24 24"
        fill="none"
        aria-hidden="true"
        focusable="false"
    >
        <path
            v-for="path in activePaths"
            :key="path"
            :d="path"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
        />
    </svg>
</template>

<script setup lang="ts">
const { name, size = 24 } = defineProps<{
    name: keyof typeof paths;
    size?: number;
}>();

const paths = {
    agent: [
        "M12 3l1.7 4.8L18.5 9.5l-4.8 1.7L12 16l-1.7-4.8L5.5 9.5l4.8-1.7L12 3z",
        "M5 15l.8 2.2L8 18l-2.2.8L5 21l-.8-2.2L2 18l2.2-.8L5 15z",
        "M19 14l.7 1.8 1.8.7-1.8.7L19 19l-.7-1.8-1.8-.7 1.8-.7L19 14z",
    ],
    block: ["M5 5h14v4H5z", "M5 15h14v4H5z", "M5 9v6", "M19 9v6"],
    chat: ["M5 6.5A4.5 4.5 0 019.5 2h5A4.5 4.5 0 0119 6.5v4A4.5 4.5 0 0114.5 15H11l-5 4v-4.5A4.5 4.5 0 015 10.5z"],
    chevronDown: ["M6 9l6 6 6-6"],
    close: ["M6 6l12 12", "M18 6L6 18"],
    database: ["M5 6c0-1.7 3.1-3 7-3s7 1.3 7 3-3.1 3-7 3-7-1.3-7-3z", "M5 6v6c0 1.7 3.1 3 7 3s7-1.3 7-3V6", "M5 12v6c0 1.7 3.1 3 7 3s7-1.3 7-3v-6"],
    file: ["M7 3h7l4 4v14H7z", "M14 3v5h5", "M9 13h6", "M9 17h6"],
    lock: ["M7 11V8a5 5 0 0110 0v3", "M6 11h12v10H6z", "M12 15v2"],
    logout: ["M10 7V5a2 2 0 012-2h7v18h-7a2 2 0 01-2-2v-2", "M15 12H3", "M6 9l-3 3 3 3"],
    menu: ["M4 7h16", "M4 12h16", "M4 17h16"],
    mic: ["M12 3a3 3 0 00-3 3v5a3 3 0 006 0V6a3 3 0 00-3-3z", "M5 10a7 7 0 0014 0", "M12 17v4", "M8 21h8"],
    note: ["M6 4h9l3 3v13H6z", "M15 4v4h4", "M9 11h6", "M9 15h5"],
    panelLeft: ["M4 5h16v14H4z", "M9 5v14"],
    panelRight: ["M4 5h16v14H4z", "M15 5v14"],
    pen: ["M4 20l4.5-1 10-10a2.1 2.1 0 00-3-3l-10 10L4 20z", "M13.5 6.5l4 4"],
    plus: ["M12 5v14", "M5 12h14"],
    puzzle: ["M8 4h5v4h3a2 2 0 110 4h-3v3h-3a2 2 0 11-4 0H4v-5h4V7a3 3 0 010-3z"],
    search: ["M10.5 18a7.5 7.5 0 100-15 7.5 7.5 0 000 15z", "M16 16l5 5"],
    send: ["M4 12l16-8-5 16-3-7-8-1z", "M12 13l8-9"],
    settings: ["M12 8a4 4 0 100 8 4 4 0 000-8z", "M4 12h2", "M18 12h2", "M12 4v2", "M12 18v2", "M5.6 5.6L7 7", "M17 17l1.4 1.4", "M18.4 5.6L17 7", "M7 17l-1.4 1.4"],
    shield: ["M12 3l7 3v5c0 5-3.2 8.2-7 10-3.8-1.8-7-5-7-10V6z", "M9 12l2 2 4-5"],
    stack: ["M12 3l8 4-8 4-8-4 8-4z", "M4 12l8 4 8-4", "M4 17l8 4 8-4"],
    trash: ["M5 7h14", "M10 11v6", "M14 11v6", "M6 7l1 14h10l1-14", "M9 7V4h6v3"],
    user: ["M12 12a4 4 0 100-8 4 4 0 000 8z", "M4 21a8 8 0 0116 0"],
    warning: ["M12 3l10 18H2L12 3z", "M12 9v5", "M12 18h.01"],
} as const;

const activePaths = computed(() => paths[name] ?? paths.agent);
</script>
