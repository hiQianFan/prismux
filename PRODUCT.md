# Product

## Register

product

## Users

AI coding tool power users who run Codex, Claude Code, Gemini CLI, or similar tools locally and maintain multiple accounts or provider profiles. They are usually in a working session and need to know whether the current platform/account pool can continue serving work before switching targets.

## Product Purpose

OpenMux is a local account and provider-profile control plane for AI coding tools. It keeps platform account pools visible, lets users switch the active target safely, and separates account quota, local token usage, and provider/profile configuration into clear mental models.

## Brand Personality

Calm, technical, reliable. The interface should feel like a compact status instrument rather than a marketing surface or auth-file editor.

## Anti-references

Do not look like a generic SaaS dashboard, an email/account dump, or a decorative glassmorphism toy. Avoid making local token usage look like provider quota, and avoid hiding operational state behind vague badges.

## Design Principles

- Start from the whole pool, then drill into one provider.
- Show active target as context, not as the whole story.
- Keep quota, local usage, and profile/provider config visually distinct.
- Prefer dense native macOS controls over custom decorative affordances.
- Treat switching as a confirmed backend operation, not optimistic UI.

## Accessibility & Inclusion

Target WCAG AA contrast. Color must be paired with text labels for active, stale, warning, and failure states. Motion should be minimal and non-blocking, with reduced-motion behavior respected where animations are introduced.
