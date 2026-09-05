# Security & RBAC

## Roles

| Role | Permissions |
|------|-------------|
| `owner` | All admin, publish, settings, users, AI apply, delete media |
| `staff` | Leads CRUD, content draft edit, AI apply, media upload — **no** publish, **no** user settings |

Enforce in API middleware: `requireRole('owner')` on publish and settings.

## Auth

- Auth.js with session cookie `httpOnly`, `secure` in production, `sameSite: lax`
- Passwords: bcrypt cost 12 minimum
- Optional magic link for owners who prefer email login
- No public signup — seed first owner or CLI `npm run create-owner`

## API security

```
✅ Zod validate all inputs
✅ Rate limit POST /api/leads and /api/ai/*
✅ CSRF not needed for JSON API if SameSite cookies + no cookie auth on public lead POST
✅ Sanitize lead message — strip tags
✅ Hash IP for leads (sha256 + salt), never store raw IP in logs long-term
✅ helmet-style headers via next.config headers

❌ Never expose draftJson on public routes
❌ Never return stack traces in production API
❌ Never commit .env
```

## Upload security

- Allow: `image/jpeg`, `image/png`, `image/webp`, `video/mp4` (max 20MB image, 100MB video)
- Rename files to uuid; serve from `/uploads/` or CDN
- Scan not required v1 — document as future improvement

## AI-specific

- Log prompts in `AiChangeLog` — business owners may contain customer info in chat; warn in UI
- Do not send full lead database to LLM — only `SiteContent`

## Multi-tenant note

v1 = one `Business` row per deployment. For SaaS multi-tenant later: add `businessId` to session and scope every query.
