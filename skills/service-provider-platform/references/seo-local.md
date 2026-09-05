# Local SEO

## On-page checklist

- [ ] `{service} + {city}` in `meta.seo.title` and H1 (natural, not stuffed)
- [ ] Unique `meta.seo.description` per business (150–160 chars)
- [ ] NAP identical in footer, contact page, JSON-LD
- [ ] `tel:` links on all phone displays
- [ ] Alt text on every image: `{service} in {city}`
- [ ] Internal links: services ↔ areas ↔ contact
- [ ] `/sitemap.xml` — all public routes
- [ ] `/robots.txt` — allow public, disallow `/admin`

## JSON-LD (inject in root layout)

### LocalBusiness

```json
{
  "@context": "https://schema.org",
  "@type": "LocalBusiness",
  "name": "{businessName}",
  "image": "{ogImage}",
  "telephone": "{phone}",
  "email": "{email}",
  "address": {
    "@type": "PostalAddress",
    "streetAddress": "{street}",
    "addressLocality": "{city}",
    "addressRegion": "{state}",
    "postalCode": "{zip}"
  },
  "areaServed": [{ "@type": "City", "name": "{area}" }],
  "openingHoursSpecification": []
}
```

Map `hours` array to `openingHoursSpecification`.

### FAQPage (on /faq)

One `Question` / `Answer` per FAQ item.

### AggregateRating (on /reviews if ≥ 3 testimonials)

Compute average from `testimonials[].rating`.

## Open Graph

```html
og:title, og:description, og:image, og:url, og:type=website
```

Use hero image or first gallery image.

## Content tips by vertical

| Vertical | Target keywords |
|----------|-----------------|
| Roofer | roof repair {city}, roof replacement {city} |
| Mover | movers {city}, moving company {city} |
| Plumber | emergency plumber {city}, drain cleaning {city} |

Add one localized blog section only if user requests — not required for v1.

## Post-launch (tell user in handoff)

1. Google Business Profile with same NAP
2. Bing Places
3. Submit sitemap in Search Console
4. Collect Google reviews → paste into admin testimonials
