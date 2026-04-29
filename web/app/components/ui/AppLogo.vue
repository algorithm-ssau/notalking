<template>
    <NuxtLink class="logo-mark" :to="to" :style="{ fontSize }">
        Notalking
    </NuxtLink>
</template>

<script setup lang="ts">
const { to = '/', fontSize = '17px' } = defineProps<{
    to?: string;
    fontSize?: string;
}>();
</script>
