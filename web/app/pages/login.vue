<template>
    <main class="auth-page">
        <div class="auth-glow" aria-hidden="true" />
        <div class="auth-wrap">
            <UiAppLogo class="auth-logo" font-size="20px" />
            <section
                class="auth-card surface-card"
                :class="{ 'is-shaking': shaking }"
                aria-labelledby="login-title"
            >
                <div class="auth-heading">
                    <h1 id="login-title">Welcome back</h1>
                    <p>Sign in to your notes</p>
                </div>

                <form class="auth-form" @submit.prevent="doLogin">
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
                            autocomplete="current-password"
                            placeholder="Your password"
                            required
                        />
                    </label>

                    <a class="forgot-link" href="#" @click.prevent>Forgot password?</a>

                    <p v-if="authError" class="error-chip" role="alert">
                        {{ authError }}
                    </p>

                    <button class="btn btn-primary auth-submit" type="submit" :disabled="submitting">
                        {{ submitting ? "Signing in..." : "Sign in" }}
                    </button>
                </form>

                <p class="auth-switch">
                    Don't have an account?
                    <NuxtLink to="/register">Register</NuxtLink>
                </p>
            </section>
        </div>
    </main>
</template>

<script setup lang="ts">
import { getCoreErrorMessage } from "~/utils/coreErrors";

const api = useCoreApi();
const sessionStore = useSessionStore();
const email = ref("");
const password = ref("");
const authError = ref("");
const submitting = ref(false);
const shaking = ref(false);

async function doLogin() {
    if (submitting.value) {
        return;
    }
    authError.value = "";
    submitting.value = true;
    try {
        await api.login(email.value.trim(), password.value);
        sessionStore.clear();
        await navigateTo({ path: "/app" });
    } catch (e: unknown) {
        authError.value = getCoreErrorMessage(e, "Login failed");
        triggerShake();
    } finally {
        submitting.value = false;
    }
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

.auth-logo {
    justify-self: center;
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

.forgot-link {
    justify-self: end;
    color: var(--accent-primary);
    font-size: 12px;
    line-height: 24px;
    text-decoration: none;
}

.forgot-link:hover,
.auth-switch a:hover {
    text-decoration: underline;
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

@media (max-width: 480px) {
    .auth-page {
        padding-inline: 24px;
    }

    .auth-card {
        padding: 24px;
    }
}
</style>
