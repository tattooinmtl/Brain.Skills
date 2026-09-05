# Public Site Pages

All pages consume `publishedJson` via server component data fetch — no hardcoded marketing copy in JSX.

## Global layout

- **Header:** Logo (business name), nav (Services, Areas, About, Gallery, Reviews, FAQ, Contact), phone CTA button
- **Footer:** NAP, hours, quick links, license line, social, privacy/terms
- **Mobile:** Bottom bar — Call | Quote
- **Components:** `sections/Hero`, `TrustBar`, `ServicesGrid`, `ServiceAreas`, `Testimonials`, `FaqAccordion`, `CtaBand`, `QuoteFormSection`

## Page specs

### `/` (Home)

Sections in order:

1. Hero (headline, subhead, dual CTA, image/video)
2. Trust bar (4 items)
3. Services grid (featured only, max 6)
4. Why choose us (from `about.values` or dedicated block)
5. Testimonials (3)
6. Service areas teaser
7. Inline quote form (short — name, phone, city, message)
8. CTA band

### `/services`

- List all services with cards linking to `#service-{slug}` anchors or `/services/[slug]` if implemented
- Each service: long description, starting price if set, inline quote form

### `/service-areas`

- Grid of cities/neighborhoods from `serviceAreas`
- Map: embed Google Maps iframe OR static map image (user provides API key later)
- Copy emphasizing local response time

### `/about`

- Story (`about.body`)
- Team grid if `about.team.length > 0`
- Values cards
- CTA to quote

### `/gallery` or `/projects`

- Masonry or bento grid from `gallery`
- Before/after slider when both images exist
- Skip page if gallery empty — redirect to home or hide nav item

### `/reviews`

- Testimonial cards with star rating
- Aggregate rating display for JSON-LD alignment
- Link to leave review (external Google link placeholder)

### `/faq`

- Accordion from `faq` array
- FAQPage schema (see seo-local.md)

### `/contact`

- Full quote form (all base + vertical fields)
- NAP block, hours table, map
- Alternative: `mailto:` and `tel:` prominent

### `/privacy` & `/terms`

- Render `legal.privacySummary` / `legal.termsSummary`
- Last updated date from `publishedAt`

## Rendering rules

- `generateMetadata()` per page from `meta.seo` + page overrides
- 404 page branded with phone CTA
- `robots.txt` + `sitemap.xml` dynamic routes
- Images: `next/image`, explicit width/height, WebP from uploads

## Performance

- Server Components default; client only for forms, accordion, mobile menu
- Lazy-load below-fold gallery images
- `prefers-reduced-motion` respected (frontend-dev)
