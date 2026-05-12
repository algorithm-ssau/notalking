# Git Workflow

This repository uses a `main <- dev <- feat/*` delivery flow.

## Branch roles

- `main` -- production branch. Only merge reviewed `dev` changes here.
- `dev` -- integration branch for the current delivery wave. Local testing and deployment workflow run from this branch before promotion to `main`.
- `feat/<topic>` -- short-lived implementation branches. Always branch from the current `dev`.
- `release/vX.Y.Z` -- optional release branch when a version-specific stabilization branch is needed. Do not use version numbers in feature branch names.

## Naming rules

- Feature branches use semantic names such as `feat/core`, `feat/agent-panel`, `feat/settings-super-prompt`.
- Commits use semantic prefixes such as `feat(core): add provider dropdown` or `fix(web): pin agent composer to the bottom`.
- Do not put versions like `v0.2.1` into commit messages.
- Version numbers belong only in tags, releases, and optional `release/vX.Y.Z` branches.

## Daily flow

1. Refresh `main` and `dev`.
2. Verify whether `dev` is still based on the latest required `main` state.
3. Create or update the working feature branch from `dev`.
4. Implement and validate the feature on `feat/<topic>`.
5. Merge `feat/<topic>` back into `dev`.
6. When all feature branches for the delivery wave are merged, open `main <- dev`.
7. Merge the PR to `main`.
8. Create the GitHub release and write the changelog.

## Command flow

### 1. Refresh the long-lived branches

```bash
git checkout main
git pull --ff-only origin main

git checkout dev
git pull --ff-only origin dev
```

If `dev` does not exist yet, create it from the latest `main`:

```bash
git checkout main
git pull --ff-only origin main
git checkout -b dev
git push -u origin dev
```

### 2. Start work from `dev`

```bash
git checkout dev
git pull --ff-only origin dev
git checkout -b feat/some-feature
```

### 3. Implement and commit

```bash
git status
git add <paths>
git commit -m "feat(scope): describe the change"
```

### 4. Merge feature work into `dev`

```bash
git checkout dev
git pull --ff-only origin dev
git merge --no-ff feat/some-feature
git push origin dev
```

Repeat for every active `feat/*` branch that belongs to the same delivery wave.

### 5. Open the promotion PR `main <- dev`

Use GitHub CLI:

```bash
gh pr create \
  --base main \
  --head dev \
  --title "release: promote dev to main" \
  --body-file docs/releases/vX.Y.Z.md
```

The PR body should summarize the release candidate. If a dedicated release note draft already exists, reuse it.

### 6. Merge the PR

```bash
gh pr merge --merge --delete-branch=false
```

After the PR is merged, fast-forward local `main`:

```bash
git checkout main
git pull --ff-only origin main
```

## Release flow

Create a release only after `dev` is already merged into `main`.

1. Prepare release notes in `docs/releases/vX.Y.Z.md`.
2. The document must include a `## Changes` section.
3. Tag and publish the release with GitHub CLI.

Example:

```bash
gh release create vX.Y.Z \
  --target main \
  --title "vX.Y.Z" \
  --notes-file docs/releases/vX.Y.Z.md
```

Recommended release note template:

```md
# vX.Y.Z

## Changes

- Item 1
- Item 2
- Item 3
```

## Rules for this repository

- Never merge feature branches directly into `main`.
- Always branch feature work from `dev`.
- Always push `dev` before opening the `main <- dev` PR.
- Use GitHub CLI (`gh`) for PR creation, PR merge, and release creation.
- Keep release notes versioned and stored in `docs/releases/`.
