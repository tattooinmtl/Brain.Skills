# Lead Forms & CRM-lite

## Form types

| Type | Where shown | Purpose |
|------|-------------|---------|
| `quote` | Home, services, sticky mobile bar | Primary conversion |
| `contact` | /contact | General inquiries |
| `callback` | Optional modal | "Call me back" |
| `emergency` | Plumber/HVAC/roofer storm | Urgent — highlight in admin |

## Quote form — base fields (Zod)

```typescript
const quoteLeadSchema = z.object({
  type: z.literal('quote'),
  name: z.string().min(2).max(100),
  phone: z.string().min(10).max(20),
  email: z.string().email().optional().or(z.literal('')),
  city: z.string().min(2).max(80),
  preferredContact: z.enum(['phone', 'email', 'text']),
  message: z.string().max(2000).optional(),
  verticalFields: z.record(z.string(), z.union([z.string(), z.boolean()])).optional(),
  honeypot: z.literal('').optional(), // must be empty
  turnstileToken: z.string().optional(),
});
```

## Vertical field extensions

Add to `verticalFields` per business vertical (see vertical-playbooks.md):

| Vertical | Extra fields |
|----------|--------------|
| Roofer | `roofType`, `issue`, `insuranceClaim` |
| Mover | `moveDate`, `fromZip`, `toZip`, `homeSize` |
| Plumber/HVAC | `urgency`, `issueType` |
| Landscaper | `lotSize`, `services` (array as comma string) |
| Cleaner | `sqft`, `bedrooms`, `frequency` |

## API: POST /api/leads

```
1. Rate limit by IP (e.g. 10/hour)
2. Reject if honeypot filled
3. Verify Turnstile if configured
4. Validate with Zod
5. Insert Lead { status: 'new', payload, sourceUrl, ipHash }
6. Send notification email (async, don't block response)
7. Return { ok: true, id } — never leak internal errors
```

## Admin lead UI

| Column | Notes |
|--------|-------|
| Received | Relative time + absolute on hover |
| Name / Phone | Click phone → `tel:` |
| Type | Badge color by type |
| Status | Dropdown: new, contacted, quoted, won, lost |
| Source | Page URL |
| Notes | Internal only, textarea on detail |

**Detail drawer:** full payload, timeline of status changes (optional `LeadEvent` table), "Mark contacted" quick action.

**Export:** CSV with columns matching payload + status + createdAt.

## Email notification template

Subject: `New {type} lead — {businessName}`

Body: name, phone, email, city, message, vertical fields, link to `/admin/leads/{id}`.

Use Resend default; fallback SMTP via nodemailer if `SMTP_HOST` set.

## Spam prevention

| Layer | Implementation |
|-------|----------------|
| Honeypot | Hidden `website` field, CSS off-screen |
| Rate limit | `@upstash/ratelimit` or in-memory for dev |
| Turnstile | Cloudflare widget on form when env set |
| Validation | Strict Zod, strip HTML from message |

Do not require account creation for public form submit.
