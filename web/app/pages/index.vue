<template>
    <main class="landing-page">
        <nav class="landing-nav" aria-label="Landing navigation">
            <UiAppLogo />
            <div class="landing-links">
                <a href="#features">Features</a>
                <a href="#workflow">Workflow</a>
                <a href="#stack">Stack</a>
            </div>
            <NuxtLink class="btn btn-primary" to="/register">Get started</NuxtLink>
        </nav>

        <section class="hero" aria-labelledby="hero-title">
            <p class="hero-badge">
                <span class="status-dot" />
                Structured notes with optional AI assistance
            </p>
            <h1 id="hero-title">Notalking keeps your notes usable first.</h1>
            <p class="hero-copy">
                Write block-based notes, search your knowledge, and connect an assistant when Intelligence is available.
                Core editing stays fast and reliable even when AI services are offline.
            </p>
            <div class="hero-actions">
                <NuxtLink class="btn btn-primary" to="/register">Create account</NuxtLink>
                <NuxtLink class="btn btn-secondary" to="/login">Sign in</NuxtLink>
            </div>

            <div class="product-preview" aria-label="Notalking app preview">
                <aside class="preview-notes">
                    <p>Notes</p>
                    <span class="preview-search">Search notes</span>
                    <span class="is-active">Research index</span>
                    <span>Voice capture</span>
                    <span>Assistant setup</span>
                </aside>
                <article class="preview-editor">
                    <p>Current note</p>
                    <h2>Research index</h2>
                    <span>Capture blocks, preserve order, and keep every note ready for semantic recall.</span>
                    <div class="preview-line" />
                    <div class="preview-line short" />
                </article>
                <aside class="preview-agent">
                    <p>Assistant</p>
                    <div class="agent-status-card">Intelligence offline -- notes remain editable.</div>
                    <div class="agent-bubble">When enabled, the assistant can use your notes as context.</div>
                </aside>
            </div>
        </section>

        <section id="features" class="section" aria-labelledby="features-title">
            <p class="section-kicker">Features</p>
            <h2 id="features-title">Everything needed for Notalking's MVP.</h2>
            <div class="feature-grid">
                <article v-for="feature in features" :key="feature.title" class="feature-card">
                    <UiAppIcon :name="feature.icon" :size="24" />
                    <h3>{{ feature.title }}</h3>
                    <p>{{ feature.copy }}</p>
                </article>
            </div>
        </section>

        <section id="workflow" class="section split-section" aria-labelledby="workflow-title">
            <div>
                <p class="section-kicker">Workflow</p>
                <h2 id="workflow-title">Write, search, then ask when useful.</h2>
                <p>
                    Notalking separates the editor from optional intelligence. You can keep writing and managing notes
                    while provider-backed features reconnect in the assistant panel.
                </p>
            </div>
            <div class="workflow-card">
                <div v-for="(step, index) in workflow" :key="step.title" class="workflow-step">
                    <span>{{ index + 1 }}</span>
                    <div>
                        <h3>{{ step.title }}</h3>
                        <p>{{ step.copy }}</p>
                    </div>
                </div>
            </div>
        </section>

        <section id="stack" class="section" aria-labelledby="stack-title">
            <p class="section-kicker">Stack</p>
            <h2 id="stack-title">Built with Nuxt, Tailwind, Pinia, and TypeScript.</h2>
            <div class="stack-grid">
                <article v-for="item in stack" :key="item.name" class="stack-card">
                    <strong>{{ item.badge }}</strong>
                    <h3>{{ item.name }}</h3>
                    <p>{{ item.copy }}</p>
                </article>
            </div>
        </section>

        <footer class="landing-footer">
            <UiAppLogo />
            <p>Notalking -- structured notes with optional AI context.</p>
        </footer>
    </main>
</template>

<script setup lang="ts">
const features = [
    {
        icon: "pen",
        title: "Block editor",
        copy: "Create structured notes that Core can patch, move, delete, and expose consistently.",
    },
    {
        icon: "search",
        title: "Search",
        copy: "Jump to notes by title and use semantic search when embeddings are configured.",
    },
    {
        icon: "agent",
        title: "Optional Intelligence",
        copy: "Assistant and voice features stay behind a separate service so the editor remains available.",
    },
] as const;

const workflow = [
    { title: "Capture", copy: "Write notes without requiring any provider or model to be online." },
    { title: "Retrieve", copy: "Use the global search dialog to find notes and matching blocks." },
    { title: "Assist", copy: "Enable Intelligence for LLM and voice workflows when the service is ready." },
] as const;

const stack = [
    { badge: "Nuxt", name: "Web app", copy: "File-based routes and typed Vue components for the browser client." },
    { badge: "Tailwind", name: "Styling", copy: "Utility-ready styling backed by shared CSS variables." },
    { badge: "Pinia", name: "State", copy: "Session-wide state for settings and device management." },
    { badge: "TypeScript", name: "Contracts", copy: "Typed DTOs for notes, blocks, sessions, and search results." },
] as const;
</script>

<style scoped>
.landing-page {
    min-height: 100vh;
    overflow-x: hidden;
    background:
        radial-gradient(circle at 50% 0%, rgb(15 102 245 / 0.14), transparent 32rem),
        var(--bg-base);
    color: var(--text-primary);
}

.landing-nav,
.section,
.landing-footer {
    width: min(1040px, calc(100% - 48px));
    margin: 0 auto;
}

.landing-nav {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 24px;
    min-height: 72px;
}

.landing-links {
    display: flex;
    align-items: center;
    gap: 4px;
}

.landing-links a {
    border-radius: var(--r-pill);
    color: var(--text-muted);
    padding: 6px 12px;
    text-decoration: none;
}

.landing-links a:hover {
    background: var(--bg-3);
    color: var(--text-primary);
}

.landing-nav .btn {
    justify-self: end;
    min-height: 40px;
    padding: 8px 18px;
}

.hero {
    display: grid;
    width: min(1040px, calc(100% - 48px));
    justify-items: center;
    margin: 0 auto;
    padding: 96px 0 72px;
    text-align: center;
}

.hero-badge {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    border: 1px solid var(--bg-4);
    border-radius: var(--r-pill);
    background: var(--bg-2);
    color: var(--text-secondary);
    padding: 6px 12px;
}

.hero h1 {
    max-width: 820px;
    margin: 24px 0 0;
    font-size: clamp(44px, 7vw, 76px);
    line-height: 1;
    letter-spacing: -0.04em;
}

.hero-copy {
    max-width: 660px;
    margin: 24px 0 0;
    color: var(--text-tertiary);
    font-size: 18px;
    line-height: 30px;
}

.hero-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 12px;
    margin-top: 32px;
}

.product-preview {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr) 280px;
    width: 100%;
    min-height: 380px;
    overflow: hidden;
    margin-top: 64px;
    border: 1px solid var(--bg-3);
    border-radius: 28px;
    background: var(--bg-1);
    box-shadow: var(--shadow-soft);
    text-align: left;
}

.preview-notes,
.preview-agent {
    display: grid;
    align-content: start;
    gap: 10px;
    background: var(--bg-1);
    padding: 24px;
}

.preview-notes {
    border-right: 1px solid var(--bg-3);
}

.preview-agent {
    border-left: 1px solid var(--bg-3);
}

.preview-notes p,
.preview-agent p,
.preview-editor p,
.section-kicker {
    margin: 0;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
}

.preview-notes span,
.preview-search,
.agent-status-card,
.agent-bubble {
    border-radius: var(--r-item);
    color: var(--text-muted);
    padding: 8px 10px;
}

.preview-notes span.is-active,
.agent-status-card {
    background: var(--bg-3);
    color: var(--text-primary);
}

.preview-search {
    background: var(--bg-2);
    color: var(--text-disabled);
}

.preview-editor {
    padding: 48px 40px;
}

.preview-editor h2 {
    margin: 16px 0 12px;
    font-size: 32px;
    line-height: 40px;
    letter-spacing: -0.02em;
}

.preview-editor span,
.agent-bubble {
    display: block;
    color: var(--text-tertiary);
}

.preview-line {
    height: 12px;
    margin-top: 28px;
    border-radius: var(--r-pill);
    background: var(--bg-3);
}

.preview-line.short {
    width: 62%;
    margin-top: 12px;
}

.agent-bubble {
    border: 1px solid var(--bg-3);
    background: var(--bg-2);
}

.section {
    padding: 88px 0;
}

.section h2 {
    max-width: 740px;
    margin: 12px 0 32px;
    font-size: clamp(30px, 4vw, 48px);
    line-height: 1.08;
    letter-spacing: -0.03em;
}

.feature-grid,
.stack-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 16px;
}

.stack-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
}

.feature-card,
.stack-card,
.workflow-card {
    border: 1px solid var(--bg-3);
    border-radius: var(--r-card);
    background: var(--bg-1);
    padding: 24px;
}

.feature-card svg {
    color: var(--accent-gold);
}

.feature-card h3,
.stack-card h3,
.workflow-step h3 {
    margin: 18px 0 8px;
    font-size: 18px;
    line-height: 24px;
}

.feature-card p,
.stack-card p,
.workflow-step p,
.split-section > div > p {
    margin: 0;
    color: var(--text-tertiary);
}

.split-section {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(320px, 0.9fr);
    gap: 48px;
    align-items: start;
}

.workflow-card {
    display: grid;
    gap: 16px;
}

.workflow-step {
    display: flex;
    gap: 16px;
}

.workflow-step > span {
    display: grid;
    width: 34px;
    height: 34px;
    flex: 0 0 auto;
    place-items: center;
    border: 1px solid var(--bg-4);
    border-radius: var(--r-pill);
    color: var(--accent-gold);
    font-weight: 700;
}

.workflow-step h3 {
    margin-top: 2px;
}

.stack-card strong {
    display: inline-flex;
    border-radius: var(--r-pill);
    background: var(--bg-3);
    color: var(--accent-gold);
    padding: 4px 10px;
    font-size: 12px;
}

.landing-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    border-top: 1px solid var(--bg-3);
    padding: 28px 0 40px;
    color: var(--text-muted);
}

@media (max-width: 960px) {
    .landing-nav {
        grid-template-columns: 1fr auto;
    }

    .landing-links {
        display: none;
    }

    .product-preview,
    .feature-grid,
    .stack-grid,
    .split-section {
        grid-template-columns: 1fr;
    }

    .preview-notes,
    .preview-agent {
        display: none;
    }
}

@media (max-width: 640px) {
    .landing-nav,
    .section,
    .landing-footer,
    .hero {
        width: min(100% - 32px, 1040px);
    }

    .hero {
        padding-top: 64px;
    }

    .landing-footer {
        align-items: flex-start;
        flex-direction: column;
    }
}
</style>
