<template>
    <main class="auth-page">
        <div class="auth-glow" aria-hidden="true" />
        <div class="auth-wrap">
            <UiAppLogo class="auth-logo" />
            <section
                class="auth-card surface-card"
                :class="{ 'is-shaking': shaking }"
                aria-labelledby="register-title"
            >
                <div class="auth-heading">
                    <h1 id="register-title">Create account</h1>
                    <p>Set up your Notalking workspace.</p>
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
    display: none;
}

.auth-wrap {
    position: relative;
    z-index: 1;
    display: grid;
    width: min(390px, 100%);
    justify-items: center;
    gap: 22px;
}

.auth-logo {
    justify-self: center;
}

.auth-card {
    width: 100%;
    border-color: var(--bg-3);
    border-radius: var(--r-card);
    background:
        linear-gradient(180deg, rgb(255 255 255 / 0.04), transparent),
        var(--bg-2);
    padding: 32px;
    box-shadow: var(--shadow-soft);
}

.auth-card.is-shaking {
    animation: shake 360ms ease both;
}

.auth-heading {
    text-align: center;
}

.auth-heading h1 {
    margin: 0;
    color: var(--text-primary);
    font-family: var(--font-heading);
    font-size: 32px;
    font-weight: 700;
    letter-spacing: -0.04em;
    line-height: 40px;
}

.auth-heading p,
.auth-switch {
    margin: 8px 0 0;
    color: var(--text-tertiary);
    font-size: 16px;
    line-height: 24px;
}

.auth-form {
    display: grid;
    gap: 14px;
    margin-top: 24px;
}

.forgot-link {
    justify-self: end;
    color: var(--text-secondary);
    font-size: 14px;
    line-height: 24px;
    text-decoration: none;
}

.forgot-link:hover,
.auth-switch a:hover {
    text-decoration: underline;
}

.strength {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
    margin-top: -6px;
}

.strength__segment {
    height: 4px;
    border-radius: var(--r-pill);
    background: var(--bg-4);
    transition: background-color 120ms ease;
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
    margin-top: 2px;
}

.auth-switch {
    text-align: center;
}

.auth-switch a {
    color: var(--text-primary);
    font-weight: 600;
    text-decoration: none;
}

@media (max-width: 480px) {
    .auth-page {
        padding-inline: 16px;
    }

    .auth-card {
        padding: 22px;
    }
}
</style>
