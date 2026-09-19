# Rocktier PDF Squeeze Design Specification

> Nothing OS 4.0 / 5.0 inspired design system.

## 1. Visual Language

- **Minimalism**: Remove everything that doesn't serve a purpose.
- **Transparency**: Frosted glass, subtle borders, depth through blur.
- **Typography**: System-native, tight tracking, high readability.
- **Motion**: Purposeful only. No decoration.

## 2. Color Palette

| Role | Token | Value | Usage |
|------|-------|-------|-------|
| Background | `--bg-primary` | `#000000` | App background |
| Surface | `--bg-secondary` | `#0a0a0a` | Cards, panels |
| Card | `--bg-card` | `rgba(255,255,255,0.03)` | Default card |
| Card Hover | `--bg-card-hover` | `rgba(255,255,255,0.06)` | Card hover state |
| Border | `--border` | `rgba(255,255,255,0.08)` | Default border |
| Border Hover | `--border-hover` | `rgba(255,255,255,0.15)` | Border hover |
| Text Primary | `--text-primary` | `rgba(255,255,255,0.95)` | Headings, primary |
| Text Secondary | `--text-secondary` | `rgba(255,255,255,0.55)` | Secondary text |
| Text Tertiary | `--text-tertiary` | `rgba(255,255,255,0.35)` | Placeholder, disabled |
| Accent | `--accent` | `#ffffff` | Primary CTA |
| Success | `--success` | `rgba(52,199,89,0.8)` | Positive indicators |
| Danger | `--danger` | `rgba(255,59,48,0.8)` | Errors |

## 3. Typography

- **Font stack**: System UI fonts (SF Pro / Segoe UI / Roboto)
- **Mono**: SF Mono / JetBrains Mono / Fira Code
- **Base size**: 13–14px
- **Heading scale**: 22px / 17px / 15px / 13px
- **Weight**: 400 (body), 500 (labels), 600 (headings), 700 (CTA)

## 4. Spacing

- **App padding**: 24px
- **Section gap**: 20px
- **Card gap**: 8px
- **Inline gap**: 12px

## 5. Radius

| Token | Value | Usage |
|-------|-------|-------|
| `--radius-sm` | 12px | Icon buttons, tags |
| `--radius-md` | 20px | Cards, buttons |
| `--radius-lg` | 28px | Drop zone, modals |

## 6. Effects

- **Blur**: `backdrop-filter: blur(20px)` on all elevated surfaces
- **Transition**: `0.2s cubic-bezier(0.25, 0.1, 0.25, 1)`
- **Hover lift**: `translateY(-2px)` + `box-shadow`
- **Glow**: `0 0 30px var(--accent-glow)` on primary button hover
- **Selection**: `rgba(255,255,255,0.15)`

## 7. Components

### Button

```
Primary: white bg, black text, radius 20px
Secondary: glass bg, white border, white text
Icon: 28x28, transparent bg, border on hover
```

### Card

```
bg: var(--bg-card)
border: 1px solid var(--border)
radius: 20px
backdrop-filter: blur(20px)
hover: bg -> var(--bg-card-hover), border -> var(--border-hover)
```

### Drop Zone

```
border: 1.5px dashed var(--border)
radius: 28px
padding: 64px 24px
hover: border solid, translateY(-2px), glow shadow
```

## 8. Accessibility

- Minimum contrast ratio: 4.5:1 for text
- Focus states: `box-shadow: 0 0 0 3px rgba(255,255,255,0.05)`
- Interactive targets: minimum 44x44px
- Respect `prefers-reduced-motion`

## 9. Platform Adaptation

### macOS (Apple Silicon only)
- Use SF Pro font
- Vibrant materials (`NSVisualEffectView` via Tauri)
- Native window controls
- Minimum: macOS 11.0 (Big Sur)
- Architecture: ARM64 only (M1/M2/M3/M4)

### Windows
- Use Segoe UI
- Mica material (Windows 11)
- Native window controls
- Minimum: Windows 10 1809+ (recommended)
- Windows 7/8: ZIP portable may work if WebView2 runtime is installed manually
- Architecture: x64 only
