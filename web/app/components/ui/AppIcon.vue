<template>
    <component :is="iconComponent" :size="size" aria-hidden="true" focusable="false" />
</template>

<script setup lang="ts">
import {
    IconAiAgent,
    IconBlockquote,
    IconChevronDown,
    IconX,
    IconMenu,
    IconMicrophone,
    IconNotes,
    IconLayoutSidebarLeftCollapse,
    IconLayoutSidebarRightCollapse,
    IconPencil,
    IconPlus,
    IconSearch,
    IconSend,
    IconSettings,
    IconShield,
    IconTrash,
    IconUser,
    IconAlertTriangle,
} from "@tabler/icons-vue";

const iconMap: Record<string, any> = {
    agent: IconAiAgent,
    block: IconBlockquote,
    chevronDown: IconChevronDown,
    close: IconX,
    menu: IconMenu,
    mic: IconMicrophone,
    note: IconNotes,
    panelLeft: IconLayoutSidebarLeftCollapse,
    panelRight: IconLayoutSidebarRightCollapse,
    pen: IconPencil,
    plus: IconPlus,
    search: IconSearch,
    send: IconSend,
    settings: IconSettings,
    shield: IconShield,
    trash: IconTrash,
    user: IconUser,
    warning: IconAlertTriangle,
};

const { name, size = 24 } = defineProps<{
    name: keyof typeof iconMap;
    size?: number;
}>();

const iconComponent = computed(() => iconMap[name] ?? IconAiAgent);

</script>
