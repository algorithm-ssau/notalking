<template>
    <main data-surface="marketing" class="marketing-page">
        <nav class="marketing-nav" aria-label="Landing navigation">
            <div class="marketing-nav__inner">
                <UiAppLogo />
                <div class="marketing-nav__links">
                    <a href="#features">Features</a>
                    <a href="#flow">How it works</a>
                    <a href="#stack">Stack</a>
                    <a href="#mcp">MCP</a>
                </div>
                <NuxtLink class="btn btn-primary nav-cta" to="/register">
                    Get started
                </NuxtLink>
            </div>
        </nav>

        <section class="hero" aria-labelledby="hero-title">
            <div class="hero__grid" aria-hidden="true" />
            <div class="hero__glow" aria-hidden="true" />
            <div class="hero__content rv">
                <p class="hero-badge">
                    <span class="status-dot" />
                    Local-first editor shell for agentic notes
                </p>
                <h1 id="hero-title" class="hero-title">
                    Notes that stay quiet until your <em>agent needs context</em>.
                </h1>
                <p class="hero-copy">
                    Notalking combines a block editor, semantic search, and optional
                    intelligence services so your notes remain usable even when AI is offline.
                </p>
                <div class="hero-actions">
                    <NuxtLink class="btn btn-primary" to="/register">
                        Start writing
                    </NuxtLink>
                    <NuxtLink class="btn btn-secondary" to="/login">
                        Sign in
                    </NuxtLink>
                </div>
            </div>

            <div class="app-preview rv" aria-label="Application preview">
                <div class="browser-chrome">
                    <span />
                    <span />
                    <span />
                    <div class="url-pill">notalking.local/app</div>
                </div>
                <div class="preview-shell">
                    <aside class="preview-sidebar">
                        <div class="preview-label">Notes</div>
                        <div class="preview-row is-active">Research index</div>
                        <div class="preview-row">Voice notes</div>
                        <div class="preview-row">Agent plan</div>
                    </aside>
                    <article class="preview-editor">
                        <h2>Research index</h2>
                        <p>Capture blocks, preserve structure, and let semantic search keep everything reachable.</p>
                        <p class="preview-line" />
                        <p class="preview-line short" />
                    </article>
                    <aside class="preview-agent">
                        <div class="preview-label gold">Agent</div>
                        <div class="chat-bubble ai">I found three related notes with matching blocks.</div>
                        <div class="chat-bubble user">Summarize the pattern.</div>
                    </aside>
                </div>
                <div class="preview-fade" aria-hidden="true" />
            </div>
        </section>

        <section id="features" class="marketing-section rv" aria-labelledby="features-title">
            <p class="section-kicker">Features</p>
            <h2 id="features-title">A calm workspace with agent hooks built in.</h2>
            <div class="feature-grid">
                <article v-for="feature in features" :key="feature.title" class="feature-card">
                    <span class="feature-icon"><UiAppIcon :name="feature.icon" :size="24" /></span>
                    <h3>{{ feature.title }}</h3>
                    <p>{{ feature.copy }}</p>
                </article>
            </div>
        </section>

        <section id="flow" class="marketing-section how-it-works rv" aria-labelledby="flow-title">
            <div>
                <p class="section-kicker">How it works</p>
                <h2 id="flow-title">Write first. Ask later.</h2>
                <div class="step-list">
                    <article v-for="(step, index) in steps" :key="step.title" class="step-card">
                        <span>{{ index + 1 }}</span>
                        <div>
                            <h3>{{ step.title }}</h3>
                            <p>{{ step.copy }}</p>
                        </div>
                    </article>
                </div>
            </div>
            <div class="conversation-panel">
                <div class="message ai">Core keeps notes available without Intelligence.</div>
                <div class="message user">What happens when AI returns?</div>
                <div class="message ai">The agent reconnects, fetches providers, and can cite note blocks.</div>
            </div>
        </section>

        <section id="stack" class="marketing-section rv" aria-labelledby="stack-title">
            <p class="section-kicker">Stack</p>
            <h2 id="stack-title">Explicit service boundaries, visible in the UI.</h2>
            <div class="stack-grid">
                <article v-for="item in stack" :key="item.name" class="stack-card">
                    <span :class="['stack-badge', item.tone]">{{ item.badge }}</span>
                    <h3>{{ item.name }}</h3>
                    <p>{{ item.copy }}</p>
                </article>
            </div>
        </section>

        <section id="mcp" class="marketing-section mcp-section rv" aria-labelledby="mcp-title">
            <div>
                <p class="section-kicker">MCP</p>
                <h2 id="mcp-title">Your notes are a tool surface, not a silo.</h2>
                <p>
                    Core exposes session-scoped MCP tools for resolving notes, reading blocks,
                    semantic search, and editor-equivalent mutations. External agents can work
                    with the same knowledge graph the browser edits.
                </p>
                <NuxtLink class="btn btn-secondary" to="/login">Open the app</NuxtLink>
            </div>
            <div class="mcp-diagram" aria-hidden="true">
                <div class="diagram-node browser">Browser</div>
                <div class="diagram-node core">Core</div>
                <div class="diagram-node mcp">MCP client</div>
                <div class="diagram-line one" />
                <div class="diagram-line two" />
            </div>
        </section>

        <section class="marketing-cta rv" aria-labelledby="cta-title">
            <p class="section-kicker">Ready</p>
            <h2 id="cta-title">Start with notes. Add intelligence when it is useful.</h2>
            <div class="hero-actions">
                <NuxtLink class="btn btn-primary" to="/register">Create account</NuxtLink>
                <NuxtLink class="btn btn-secondary" to="/login">Sign in</NuxtLink>
            </div>
        </section>

        <footer class="marketing-footer">
            <UiAppLogo />
            <p>Built for structured notes, semantic recall, and quiet AI assistance.</p>
        </footer>
    </main>
</template>

<script setup lang="ts">
const features = [
    {
        icon: "pen",
        title: "Block-native notes",
        copy: "Write in small structured blocks that Core can patch, move, search, and expose to tools.",
    },
    {
        icon: "search",
        title: "Semantic recall",
        copy: "Search note titles locally and ask Core for embedding-backed block matches when configured.",
    },
    {
        icon: "agent",
        title: "Optional Intelligence",
        copy: "Agent and voice features degrade inside the panel when the Intelligence service is unavailable.",
    },
] as const;

const steps = [
    {
        title: "Capture",
        copy: "Create notes and blocks in the editor without needing any AI service online.",
    },
    {
        title: "Retrieve",
        copy: "Use global search to jump to recent notes, title matches, or semantic block results.",
    },
    {
        title: "Delegate",
        copy: "When Intelligence is reachable, providers and agent features appear in their own panel.",
    },
] as const;

const stack = [
    { badge: "Rust", tone: "rust", name: "Core", copy: "Axum owns auth, notes, blocks, sessions, MCP, and semantic search." },
    { badge: "Python", tone: "python", name: "Intelligence", copy: "FastAPI hosts LLM and voice adapters without blocking the editor." },
    { badge: "Nuxt", tone: "nuxt", name: "Web", copy: "Nuxt 4 renders the editor shell, auth pages, modals, and client state." },
    { badge: "Data", tone: "storage", name: "Storage", copy: "PostgreSQL, SQLite dev mode, Redis, and Qdrant stay behind adapters." },
] as const;

let revealObserver: IntersectionObserver | null = null;

onMounted(() => {
    const els = Array.from(document.querySelectorAll<HTMLElement>(".rv"));
    revealObserver = new IntersectionObserver(
        (entries) => {
            for (const entry of entries) {
                if (entry.isIntersecting) {
                    entry.target.classList.add("vis");
                    revealObserver?.unobserve(entry.target);
                }
            }
        },
        { threshold: 0.12 },
    );
    els.forEach((el) => revealObserver?.observe(el));
});

onUnmounted(() => revealObserver?.disconnect());
</script>

<style scoped>
.marketing-page {
    min-height: 100vh;
    overflow-x: hidden;
    background:
        radial-gradient(circle at 50% 8%, rgb(201 169 110 / 0.12), transparent 34rem),
        var(--bg-base);
    color: var(--text-primary);
}

.marketing-nav {
    position: fixed;
    top: 0;
    right: 0;
    left: 0;
    z-index: 40;
    height: 56px;
    border-bottom: 1px solid rgb(255 255 255 / 0.04);
    background: rgb(0 0 0 / 0.75);
    backdrop-filter: blur(20px);
}

.marketing-nav__inner {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    width: min(1040px, calc(100% - 32px));
    height: 100%;
    margin: 0 auto;
}

.marketing-nav__links {
    display: flex;
    align-items: center;
    gap: 4px;
}

.marketing-nav__links a {
    border-radius: var(--r-pill);
    padding: 6px 12px;
    color: var(--text-muted);
    font-size: 14px;
    line-height: 24px;
    text-decoration: none;
    transition: 150ms ease;
}

.marketing-nav__links a:hover {
    background: var(--bg-3);
    color: var(--text-primary);
}

.nav-cta {
    justify-self: end;
    min-height: 40px;
    padding: 8px 20px;
    font-size: 14px;
}

.hero {
    position: relative;
    display: grid;
    min-height: 100vh;
    place-items: center;
    padding: 120px 24px 40px;
}

.hero__grid {
    position: absolute;
    inset: 0;
    opacity: 0.35;
    background-image:
        linear-gradient(rgb(255 255 255 / 0.08) 1px, transparent 1px),
        linear-gradient(90deg, rgb(255 255 255 / 0.08) 1px, transparent 1px);
    background-size: 60px 60px;
    mask-image: radial-gradient(circle at center, #000 0, transparent 72%);
}

.hero__glow {
    position: absolute;
    top: 18%;
    left: 50%;
    width: 520px;
    height: 260px;
    border-radius: 999px;
    background: rgb(201 169 110 / 0.18);
    filter: blur(70px);
    transform: translateX(-50%);
}

.hero__content {
    position: relative;
    z-index: 1;
    display: grid;
    justify-items: center;
    max-width: 860px;
    text-align: center;
}

.hero-badge {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    border: 1px solid var(--bg-4);
    border-radius: var(--r-pill);
    background: var(--bg-2);
    padding: 6px 12px;
    color: var(--text-secondary);
    font-size: 12.5px;
    font-weight: 500;
    line-height: 24px;
}

.hero-title {
    max-width: 860px;
    margin: 24px 0 0;
    font-family: var(--font-heading);
    font-size: clamp(44px, 7.5vw, 86px);
    font-weight: 700;
    letter-spacing: -0.035em;
    line-height: 0.95;
}

.hero-title em {
    color: var(--accent-gold);
    font-style: italic;
}

.hero-copy {
    max-width: 480px;
    margin: 24px 0 0;
    color: var(--text-muted);
    font-size: 18px;
    font-weight: 300;
    line-height: 1.7;
}

.hero-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 12px;
    margin-top: 32px;
}

.app-preview {
    position: relative;
    z-index: 1;
    width: min(920px, 100%);
    margin-top: -7vh;
    overflow: hidden;
    border: 1px solid var(--bg-3);
    border-radius: 24px;
    background: var(--bg-1);
    box-shadow: 0 40px 120px rgb(0 0 0 / 0.7);
}

.browser-chrome {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 44px;
    border-bottom: 1px solid var(--bg-3);
    padding: 0 16px;
}

.browser-chrome span {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--bg-4);
}

.url-pill {
    flex: 1;
    max-width: 340px;
    margin-left: 8px;
    border-radius: var(--r-pill);
    background: var(--bg-3);
    color: var(--text-muted);
    padding: 3px 12px;
    font-size: 12px;
}

.preview-shell {
    display: grid;
    grid-template-columns: 200px minmax(0, 1fr) 260px;
    min-height: 360px;
}

.preview-sidebar,
.preview-agent {
    background: var(--bg-1);
    padding: 20px;
}

.preview-sidebar {
    border-right: 1px solid var(--bg-3);
}

.preview-agent {
    border-left: 1px solid var(--bg-3);
}

.preview-label {
    margin-bottom: 16px;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
}

.preview-label.gold {
    color: var(--accent-gold);
}

.preview-row {
    border-radius: var(--r-item);
    padding: 8px 10px;
    color: var(--text-muted);
    font-size: 14px;
}

.preview-row.is-active {
    background: var(--bg-3);
    color: var(--text-primary);
}

.preview-editor {
    padding: 48px 32px;
}

.preview-editor h2 {
    margin: 0;
    font-family: var(--font-heading);
    font-size: 28px;
    letter-spacing: -0.02em;
}

.preview-editor p {
    max-width: 480px;
    color: var(--text-muted);
}

.preview-line {
    height: 12px;
    border-radius: var(--r-pill);
    background: var(--bg-3);
}

.preview-line.short {
    width: 62%;
}

.chat-bubble {
    max-width: 210px;
    border-radius: 16px;
    padding: 10px 12px;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 20px;
}

.chat-bubble + .chat-bubble {
    margin-top: 12px;
}

.chat-bubble.ai {
    border: 1px solid var(--bg-3);
    background: var(--bg-2);
}

.chat-bubble.user {
    margin-left: auto;
    background: var(--bg-3);
}

.preview-fade {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    height: 110px;
    background: linear-gradient(to top, #000, transparent);
    pointer-events: none;
}

.marketing-section,
.marketing-cta,
.marketing-footer {
    width: min(1040px, calc(100% - 48px));
    margin: 0 auto;
}

.marketing-section {
    padding: 96px 0;
}

.section-kicker {
    margin: 0 0 12px;
    color: var(--accent-gold);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
}

.marketing-section h2,
.marketing-cta h2 {
    max-width: 720px;
    margin: 0 0 32px;
    font-family: var(--font-heading);
    font-size: clamp(30px, 4vw, 48px);
    font-weight: 700;
    letter-spacing: -0.03em;
    line-height: 1.04;
}

.feature-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1px;
    overflow: hidden;
    border: 1px solid var(--bg-2);
    border-radius: var(--r-card);
    background: var(--bg-2);
}

.feature-card {
    min-height: 250px;
    background: var(--bg-1);
    padding: 32px 28px;
    transition: background-color 150ms ease;
}

.feature-card:hover {
    background: var(--bg-2);
}

.feature-icon {
    display: inline-grid;
    width: 42px;
    height: 42px;
    place-items: center;
    border: 1px solid var(--bg-4);
    border-radius: var(--r-item);
    background: var(--bg-3);
    color: var(--accent-gold);
}

.feature-card h3,
.step-card h3,
.stack-card h3 {
    margin: 20px 0 8px;
    font-size: 16px;
    font-weight: 600;
}

.feature-card p,
.step-card p,
.stack-card p,
.mcp-section p {
    margin: 0;
    color: var(--text-muted);
    font-size: 14px;
    font-weight: 300;
    line-height: 24px;
}

.how-it-works,
.mcp-section {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(320px, 0.86fr);
    gap: 48px;
    align-items: center;
}

.step-list {
    display: grid;
    gap: 16px;
}

.step-card {
    display: flex;
    gap: 16px;
    border-radius: var(--r-card);
    padding: 12px;
    transition: background-color 150ms ease;
}

.step-card:hover {
    background: var(--bg-1);
}

.step-card span {
    display: grid;
    width: 34px;
    height: 34px;
    flex: 0 0 auto;
    place-items: center;
    border: 1px solid var(--bg-4);
    border-radius: var(--r-pill);
    color: var(--text-muted);
    transition: 150ms ease;
}

.step-card:hover span {
    border-color: var(--accent-gold);
    color: var(--accent-gold);
}

.step-card h3 {
    margin-top: 2px;
}

.conversation-panel {
    border: 1px solid var(--bg-3);
    border-radius: var(--r-card);
    background: var(--bg-1);
    padding: 24px;
}

.message {
    max-width: 78%;
    border-radius: 16px;
    padding: 12px 14px;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 300;
}

.message + .message {
    margin-top: 12px;
}

.message.ai {
    border: 1px solid var(--bg-3);
    background: var(--bg-2);
}

.message.user {
    margin-left: auto;
    background: var(--bg-3);
}

.stack-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 16px;
}

.stack-card {
    min-height: 190px;
    border: 1px solid var(--bg-3);
    border-radius: var(--r-card);
    background: var(--bg-1);
    padding: 24px;
}

.stack-badge {
    display: inline-flex;
    border-radius: var(--r-pill);
    padding: 4px 10px;
    font-size: 12px;
    font-weight: 700;
}

.stack-badge.rust {
    background: rgb(230 126 34 / 0.16);
    color: #e67e22;
}

.stack-badge.python {
    background: rgb(52 152 219 / 0.16);
    color: #6bb7f0;
}

.stack-badge.nuxt {
    background: rgb(0 220 130 / 0.16);
    color: #00dc82;
}

.stack-badge.storage {
    background: rgb(39 174 96 / 0.16);
    color: var(--success);
}

.mcp-section p {
    max-width: 560px;
    margin-bottom: 28px;
    font-size: 16px;
    line-height: 28px;
}

.mcp-diagram {
    position: relative;
    min-height: 320px;
    border: 1px solid var(--bg-3);
    border-radius: var(--r-card);
    background:
        radial-gradient(circle at 70% 30%, rgb(15 102 245 / 0.12), transparent 12rem),
        var(--bg-1);
}

.diagram-node {
    position: absolute;
    z-index: 1;
    border: 1px solid var(--bg-4);
    border-radius: var(--r-pill);
    background: var(--bg-2);
    padding: 10px 16px;
    color: var(--text-secondary);
    font-size: 14px;
}

.diagram-node.browser {
    top: 46px;
    left: 32px;
}

.diagram-node.core {
    top: 132px;
    left: 50%;
    color: var(--accent-gold);
    transform: translateX(-50%);
}

.diagram-node.mcp {
    right: 32px;
    bottom: 52px;
}

.diagram-line {
    position: absolute;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--accent-gold), transparent);
    transform-origin: left center;
}

.diagram-line.one {
    top: 100px;
    left: 116px;
    width: 220px;
    transform: rotate(18deg);
}

.diagram-line.two {
    right: 104px;
    bottom: 112px;
    width: 220px;
    transform: rotate(-24deg);
}

.marketing-cta {
    display: grid;
    justify-items: center;
    padding: 120px 24px;
    text-align: center;
}

.marketing-cta h2 {
    margin-bottom: 0;
}

.marketing-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    border-top: 1px solid var(--bg-2);
    padding: 32px 0 48px;
    color: var(--text-muted);
    font-size: 14px;
}

.rv {
    opacity: 0;
    transform: translateY(18px);
}

.rv.vis {
    animation: fade-up 520ms var(--ease-out) both;
}

@media (max-width: 900px) {
    .marketing-nav__inner {
        grid-template-columns: 1fr auto;
    }

    .marketing-nav__links {
        display: none;
    }

    .preview-shell {
        grid-template-columns: minmax(0, 1fr);
    }

    .preview-sidebar,
    .preview-agent {
        display: none;
    }

    .feature-grid,
    .how-it-works,
    .stack-grid,
    .mcp-section {
        grid-template-columns: 1fr;
    }

    .stack-grid {
        gap: 12px;
    }
}

@media (max-width: 560px) {
    .hero {
        padding-top: 104px;
    }

    .hero-actions,
    .marketing-footer {
        align-items: stretch;
        flex-direction: column;
    }

    .hero-actions .btn,
    .marketing-cta .btn {
        width: 100%;
    }

    .marketing-section,
    .marketing-cta,
    .marketing-footer {
        width: min(100% - 32px, 1040px);
    }
}
</style>
