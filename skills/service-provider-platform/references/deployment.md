# Deployment

## Recommended targets

| Target | Notes |
|--------|-------|
| **Vercel** + Neon Postgres | Fastest for Next.js |
| **Railway** / **Render** | App + Postgres single project |
| **Docker** | `Dockerfile` + `docker-compose` with postgres volume |

## Pre-deploy checklist

- [ ] `prisma migrate deploy` in CI/CD
- [ ] `AUTH_SECRET` — `openssl rand -base64 32`
- [ ] `DATABASE_URL` — production Postgres (not SQLite)
- [ ] `NEXT_PUBLIC_SITE_URL` — canonical HTTPS URL
- [ ] Seed or migrate initial `publishedJson`
- [ ] Create owner account
- [ ] Configure Resend domain or SMTP
- [ ] S3/R2 for uploads if not using Vercel blob
- [ ] Turnstile keys for production forms

## Build commands

```bash
npm ci
npx prisma generate
npx prisma migrate deploy
npm run build
npm start
```

## Smoke tests (post-deploy)

```bash
curl -s https://example.com/api/health
curl -s https://example.com/ | head
curl -X POST https://example.com/api/leads -H "Content-Type: application/json" -d '{"type":"contact","name":"Test","phone":"5555550100","city":"Austin","preferredContact":"phone","honeypot":""}'
# Expect 200 + lead id; delete test lead in admin
```

Login to `/admin`, edit hero headline, publish, verify public change.

Send AI chat "Change tagline to X", apply, publish, verify.

## Environment matrix

| Var | Dev | Prod |
|-----|-----|------|
| DATABASE_URL | local Postgres or Docker | managed Postgres |
| AUTH_SECRET | dev secret | strong random |
| OPENAI_API_KEY | optional | required for AI tab |
| RESEND_API_KEY | optional | recommended |
| S3_* | optional | recommended |

## Monitoring (handoff suggestions)

- Vercel Analytics or Plausible for traffic
- Sentry for API errors
- Uptime on `/api/health` and `/`

## Backup

- Daily Postgres backup (provider default)
- Export `SiteContent.publishedJson` monthly via admin script optional
