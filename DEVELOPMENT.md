# Development Guide

## Prerequisites

- Node.js 20+
- pnpm 9+ (or npm)
- Rust 1.75+ (via rustup)
- Go 1.22+

## Quick Start

```bash
# Install dependencies
npm install

# Prepare placeholder icons (needed for first build)
npm run prepare

# Run dev mode (frontend + Tauri)
npm run tauri:dev
```

## Project Structure

```
├── src/                    # React frontend
│   ├── components/         # UI components
│   ├── hooks/
│   ├── services/           # Tauri API wrappers
│   ├── styles/             # CSS + design tokens
│   ├── App.tsx
│   └── main.tsx
├── src-tauri/              # Tauri backend
│   ├── src/main.rs
│   ├── capabilities/
│   └── tauri.conf.json
├── go-engine/              # Go PDF engine
│   ├── main.go
│   ├── go.mod
│   └── internal/
├── scripts/                # Build/dev scripts
└── .github/workflows/      # CI/CD
```

## Adding a New Component

1. Create `src/components/MyComponent.tsx`
2. Add styles to `src/styles/app.css` or inline
3. Import in `App.tsx`

## Adding a New Tauri Command

1. Add function in `src-tauri/src/main.rs`
2. Register in `invoke_handler`
3. Add wrapper in `src/services/engine.ts`
4. Call from component/hook

## Go Engine Changes

```bash
cd go-engine
go build -o ../src-tauri/bin/pdf-engine .
```

## Debugging

- Frontend: browser DevTools (opened automatically in dev mode)
- Rust: `cargo log` or IDE debugger
- Go: `dlv debug` or print statements

## Platform Support

- **macOS**: Apple Silicon only (ARM64).
- **Windows**: x64 only. Builds produce MSI, NSIS installer, and ZIP portable.

## Commit Convention

- `feat:` new feature
- `fix:` bug fix
- `refactor:` code change that neither fixes a bug nor adds a feature
- `docs:` documentation only
- `style:` formatting, missing semicolons, etc.
- `test:` adding tests
- `chore:` maintenance
