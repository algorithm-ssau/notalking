<template>
    <NuxtLink class="logo-mark" :to="to" :aria-label="ariaLabel">
        <span v-if="label" class="logo-label">{{ label }}</span>
    </NuxtLink>
</template>

<script setup lang="ts">
const { to = "/", label = "Notalking" } = defineProps<{
    to?: string;
    label?: string;
}>();

const ariaLabel = computed(() => label || "Notalking");
</script>
