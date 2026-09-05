# Admin Dashboard

Route prefix: `/admin`. Middleware protects all routes except `/admin/login`.

## Layout

```
/admin
├── /login
├── / (overview)
├── /leads
├── /leads/[id]
├── /content
├── /content/preview   # draft render in iframe or new tab
├── /media
├── /ai
└── /settings
```

Sidebar nav: Overview, Leads, Edit Site, Media, AI Assistant, Settings.

## Overview (`/admin`)

| Widget | Data |
|--------|------|
| New leads (7d) | count status=new |
| Leads by status | small bar chart or counts |
| Quick actions | View leads, Edit homepage, Open AI, Publish if draft differs |
| Draft indicator | Banner if `draftJson !== publishedJson` |

## Content editor (`/admin/content`)

Tabbed sections matching SiteContent:

| Tab | Editor controls |
|-----|-----------------|
| Business | name, phone, email, address, hours (repeater), social links |
| Homepage | hero fields, trust bar, CTA band |
| Services | CRUD list — drag reorder, slug auto from name |
| Areas | CRUD cities |
| About | rich text (Tiptap or textarea), team members |
| Gallery | image picker + caption |
| Reviews | CRUD testimonials |
| FAQ | CRUD Q&A |
| SEO | title, description, keywords |

**Actions:**

- Save draft (PATCH `/api/content/draft`)
- Preview draft (opens `(preview)` route with `?draft=1` token or session flag)
- Publish (POST `/api/content/publish`) — confirm modal

## Media library (`/admin/media`)

- Upload drag-drop → `POST /api/upload`
- Grid with filename, size, used-in hint
- Copy URL to paste into content JSON fields
- Delete only if not referenced (soft check)

## Leads (`/admin/leads`)

- Data table: sort by date desc default
- Filters: status, type, date range
- Bulk: export CSV
- Row click → detail

## AI Assistant (`/admin/ai`)

Split pane:

- Left: chat history (persist thread in session or DB optional)
- Right: live diff preview of draft vs proposed patch

Flow:

1. User sends message
2. `POST /api/ai/chat` → `{ patch, explanation, affectedSections[] }`
3. UI shows human-readable summary + JSON diff (collapsible)
4. Buttons: **Apply to draft** | **Discard**
5. Apply → `POST /api/ai/apply` → refresh content editor state

Show disclaimer: "Review changes before publishing. AI can make mistakes."

## Settings (`/admin/settings`)

- Notification email(s) for leads
- Business timezone
- User management (owner only): invite staff
- Danger zone: reset draft from published

## UX standards

- Loading skeletons on all tables
- Toast on save success/error
- Empty states with guidance ("No leads yet — share your site link")
- Desktop-first; usable on tablet for owner in truck
