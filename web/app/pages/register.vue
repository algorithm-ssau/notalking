<template>
    <main class="auth-page">
        <div class="auth-glow" aria-hidden="true" />
        <div class="auth-wrap">
            <UiAppLogo class="auth-logo" font-size="20px" />
            <section
                class="auth-card surface-card"
                :class="{ 'is-shaking': shaking }"
                aria-labelledby="register-title"
            >
                <div class="auth-heading">
                    <h1 id="register-title">Create account</h1>
                    <p>Set up your note workspace</p>
                </div>

                <form class="auth-form" @submit.prevent="doRegister">
                    <label class="form-label">
                        Full name
                        <input
                            v-model="fullName"
                            class="input"
                            type="text"
                            autocomplete="name"
                            placeholder="Ada Lovelace"
                        />
                    </label>
                    <label class="form-label">
                        Email
                        <input
                            v-model="email"
                            class="input"
                            type="email"
                            autocomplete="username"
                            placeholder="you@example.com"
                            required
                        />
                    </label>
                    <label class="form-label">
                        Password
                        <input
                            v-model="password"
                            class="input"
                            type="password"
                            autocomplete="new-password"
                            placeholder="Create a password"
                            required
                        />
                    </label>

                    <div v-if="password" class="strength" aria-label="Password strength">
                        <span
                            v-for="segment in 4"
                            :key="segment"
                            :class="['strength__segment', strengthClass(segment)]"
                        />
                    </div>

                    <label class="form-label">
                        Confirm password
                        <input
                            v-model="confirmPassword"
                            class="input"
                            type="password"
                            autocomplete="new-password"
                            placeholder="Repeat password"
                            required
                        />
                    </label>

                    <p v-if="authError" class="error-chip" role="alert">
                        {{ authError }}
                    </p>

                    <button class="btn btn-primary auth-submit" type="submit" :disabled="submitting">
                        {{ submitting ? "Creating..." : "Create account" }}
                    </button>
                </form>

                <p class="auth-switch">
                    Already have an account?
                    <NuxtLink to="/login">Sign in</NuxtLink>
                </p>
            </section>
        </div>
    </main>
</template>

<script setup lang="ts">
import { getCoreErrorMessage } from "~/utils/coreErrors";

const api = useCoreApi();
const sessionStore = useSessionStore();
const fullName = ref("");
const email = ref("");
const password = ref("");
const confirmPassword = ref("");
const authError = ref("");
const submitting = ref(false);
const shaking = ref(false);

const passwordStrength = computed(() => {
    const value = password.value;
    let score = 0;
    if (value.length >= 8) score++;
    if (/[A-Z]/.test(value) && /[a-z]/.test(value)) score++;
    if (/\d/.test(value)) score++;
    if (/[^A-Za-z0-9]/.test(value) || value.length >= 14) score++;
    return Math.min(score, 4);
});

async function doRegister() {
    if (submitting.value) {
        return;
    }
    authError.value = "";
    if (password.value !== confirmPassword.value) {
        authError.value = "Passwords do not match.";
        triggerShake();
        return;
    }
    submitting.value = true;
    try {
        await api.register(email.value.trim(), password.value);
        sessionStore.clear();
        await navigateTo({ path: "/app" });
    } catch (e: unknown) {
        authError.value = getCoreErrorMessage(e, "Registration failed");
        triggerShake();
    } finally {
        submitting.value = false;
    }
}

function strengthClass(segment: number): string {
    if (segment > passwordStrength.value) {
        return "";
    }
    if (passwordStrength.value <= 1) {
        return "danger";
    }
    if (passwordStrength.value <= 3) {
        return "warning";
    }
    return "success";
}

function triggerShake() {
    shaking.value = false;
    requestAnimationFrame(() => {
        shaking.value = true;
        window.setTimeout(() => {
            shaking.value = false;
        }, 360);
    });
}
</script>

<style scoped>
.auth-page {
    position: relative;
    display: grid;
    min-height: 100vh;
    place-items: center;
    overflow: hidden;
    background: var(--bg-base);
    padding: 24px;
}

.auth-glow {
    position: absolute;
    inset: auto auto 44% 50%;
    width: 480px;
    height: 480px;
    border-radius: 50%;
    background: rgb(201 169 110 / 0.05);
    filter: blur(42px);
    transform: translateX(-50%);
}

.auth-wrap {
    position: relative;
    z-index: 1;
    display: grid;
    width: min(400px, 100%);
    justify-items: center;
    gap: 24px;
}

.auth-card {
    width: 100%;
    padding: 32px;
}

.auth-card.is-shaking {
    animation: shake 360ms ease both;
}

.auth-heading {
    text-align: center;
}

.auth-heading h1 {
    margin: 0;
    font-family: var(--font-heading);
    font-size: 24px;
    font-weight: 700;
    letter-spacing: -0.02em;
}

.auth-heading p,
.auth-switch {
    margin: 8px 0 0;
    color: var(--text-muted);
    font-size: 14px;
    line-height: 24px;
}

.auth-form {
    display: grid;
    gap: 16px;
    margin-top: 24px;
}

.strength {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
    margin-top: -8px;
}

.strength__segment {
    height: 4px;
    border-radius: var(--r-pill);
    background: var(--bg-3);
    transition: background-color 150ms ease;
}

.strength__segment.danger {
    background: var(--danger);
}

.strength__segment.warning {
    background: var(--warning);
}

.strength__segment.success {
    background: var(--success);
}

.auth-submit {
    width: 100%;
}

.auth-switch {
    text-align: center;
}

.auth-switch a {
    color: var(--accent-primary);
    font-weight: 600;
    text-decoration: none;
}

.auth-switch a:hover {
    text-decoration: underline;
}

@media (max-width: 480px) {
    .auth-page {
        padding-inline: 24px;
    }

    .auth-card {
        padding: 24px;
    }
}
</style>
