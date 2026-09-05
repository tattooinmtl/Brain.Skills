# AI Site Editor

Structured editing only — the LLM never outputs React, HTML, or CSS files.

## System prompt (server-side template)

```
You are a website content editor for a local {vertical} business in {city}.
You receive the current SiteContent JSON and a user instruction.
Respond with JSON only:
{
  "explanation": "1-2 sentences for the business owner",
  "affectedSections": ["hero", "services"],
  "patch": { ... SiteContentPatch }
}
Rules:
- Patch only fields needed for the request
- Keep tone professional and local
- Do not change phone, email, or address unless the user explicitly asks
- For new services, include id (slug), slug, featured:false unless asked
- Max 3 testimonials changed per request unless asked
- No markdown in string values
```

## API: POST /api/ai/chat

```typescript
// Request
{ "message": string, "threadId"?: string }

// Response
{
  "explanation": string,
  "affectedSections": string[],
  "patch": SiteContentPatch,
  "valid": boolean,
  "errors"?: string[]  // Zod errors if invalid
}
```

Implementation:

1. Load `draftJson` for business
2. Build messages: system + current content (truncate gallery if > 20 items) + user message
3. Call OpenAI-compatible chat completions (`response_format: json_object` if supported)
4. Parse JSON → validate `patch` with Zod
5. Dry-run merge → validate full `SiteContent`
6. Return preview diff (`jsondiffpatch` or simple before/after per section)

## API: POST /api/ai/apply

```typescript
{ "patch": SiteContentPatch, "prompt": string }  // prompt = last user message for audit
```

1. Re-validate patch (never trust client)
2. Merge into draft
3. Insert `AiChangeLog`
4. Return updated `draftJson`

## Guardrails

| Risk | Mitigation |
|------|------------|
| Hallucinated services | Zod requires `id`, `slug`, `name` on each service |
| PII changes | Strip `meta.phone/email/address` from patch unless `allowPii: true` flag on request after user confirms in UI |
| Huge patches | Reject if patch JSON > 50KB |
| Off-topic | System prompt: refuse non-site requests politely |
| Cost | Rate limit 30 messages/day per business in dev; configurable |

## Example interactions

| User says | Expected patch |
|-----------|----------------|
| "Add gutter cleaning service" | `services` array with new item appended |
| "Emphasize free estimates in hero" | `hero.headline` or `hero.badges` |
| "We now serve Georgetown" | `serviceAreas` append |
| "Remove the emergency banner" | `emergency.enabled: false` |
| "Make FAQ answer about pricing shorter" | `faq` item by id match |

## UI diff preview

Show section cards:

- **Hero:** headline before → after
- **Services:** +1 added, ~2 modified
- **FAQ:** 1 answer changed

Color: green add, amber change, red remove.

## Optional: conversation memory

Store last 10 messages in `AiThread` table if user needs multi-turn ("also add that to FAQ"). For v1, single-turn is acceptable.

## Model config

```env
OPENAI_API_KEY=
OPENAI_BASE_URL=https://api.openai.com/v1  # optional, for compatible providers
AI_MODEL=gpt-4o-mini                       # default; user can override
```

Fallback message if key missing: disable AI tab with setup instructions.
