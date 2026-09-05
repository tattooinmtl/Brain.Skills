# SiteContent Schema

Single source of truth for public site rendering and admin/AI editing. All public UI reads `publishedJson`; admin and AI edit `draftJson`.

## TypeScript shape (implement in `src/features/content/types.ts`)

```typescript
export type SiteContent = {
  meta: {
    businessName: string;
    tagline: string;
    vertical: string; // roofer | mover | plumber | hvac | landscaper | cleaner | pest | general
    primaryCity: string;
    state: string;
    phone: string;
    email: string;
    address: { street: string; city: string; state: string; zip: string };
    hours: { day: string; open: string; close: string; closed?: boolean }[];
    licenseNumber?: string;
    insuranceText?: string;
    yearEstablished?: number;
    social: { platform: string; url: string }[];
    seo: {
      title: string;
      description: string;
      ogImage?: string;
      keywords: string[];
    };
  };
  theme: {
    accent: string; // hex, saturation < 80%
    fontHeadline: string;
    fontBody: string;
  };
  hero: {
    headline: string;
    subheadline: string;
    ctaPrimary: { label: string; href: string };
    ctaSecondary?: { label: string; href: string };
    image?: string;
    video?: string;
    badges: string[]; // "Licensed & Insured", "Free Estimates"
  };
  trustBar: { icon: string; label: string }[];
  services: {
    id: string;
    name: string;
    slug: string;
    shortDescription: string;
    longDescription: string;
    icon?: string;
    image?: string;
    startingPrice?: string;
    featured: boolean;
  }[];
  serviceAreas: {
    name: string;
    description?: string;
  }[];
  about: {
    headline: string;
    body: string;
    team: { name: string; role: string; image?: string }[];
    values: { title: string; description: string }[];
  };
  gallery: {
    id: string;
    title: string;
    beforeImage?: string;
    afterImage?: string;
    image?: string;
    caption: string;
  }[];
  testimonials: {
    id: string;
    author: string;
    location: string;
    rating: number;
    quote: string;
    service?: string;
  }[];
  faq: { id: string; question: string; answer: string }[];
  ctaBand: {
    headline: string;
    subheadline: string;
    buttonLabel: string;
  };
  emergency?: {
    enabled: boolean;
    headline: string;
    phone: string;
  };
  legal: {
    privacySummary: string;
    termsSummary: string;
  };
};
```

## SiteContentPatch (AI editor)

Partial deep merge only. Forbidden keys: `meta.phone`, `meta.email`, `meta.address` unless user explicitly asks in chat (PII guard).

```typescript
export type SiteContentPatch = Partial<{
  hero: Partial<SiteContent['hero']>;
  services: SiteContent['services']; // full array replace when adding/removing
  serviceAreas: SiteContent['serviceAreas'];
  about: Partial<SiteContent['about']>;
  gallery: SiteContent['gallery'];
  testimonials: SiteContent['testimonials'];
  faq: SiteContent['faq'];
  ctaBand: Partial<SiteContent['ctaBand']>;
  emergency: Partial<SiteContent['emergency']>;
  meta: Partial<Pick<SiteContent['meta'], 'tagline' | 'seo'>>;
}>;
```

## Prisma models (minimum)

```prisma
model Business {
  id        String   @id @default(cuid())
  slug      String   @unique
  name      String
  vertical  String
  timezone  String   @default("America/Chicago")
  users     User[]
  content   SiteContent?
  leads     Lead[]
  createdAt DateTime @default(now())
}

model SiteContent {
  id            String    @id @default(cuid())
  businessId    String    @unique
  business      Business  @relation(fields: [businessId], references: [id])
  draftJson     Json
  publishedJson Json
  publishedAt   DateTime?
  updatedAt     DateTime  @updatedAt
}

model Lead {
  id          String   @id @default(cuid())
  businessId  String
  business    Business @relation(fields: [businessId], references: [id])
  type        String   // quote | contact | callback | emergency
  status      String   @default("new")
  payload     Json
  notes       String?
  sourceUrl   String?
  ipHash      String?
  createdAt   DateTime @default(now())
  updatedAt   DateTime @updatedAt
}

model AiChangeLog {
  id         String   @id @default(cuid())
  businessId String
  userId     String
  prompt     String
  patch      Json
  appliedAt  DateTime @default(now())
}
```

## Merge rules

1. `applyPatch(draft, patch)` — deep merge objects; **replace** arrays when patch includes that array key
2. Validate full result with `SiteContentSchema` (Zod) after merge
3. On publish: `publishedJson = draftJson`, set `publishedAt = now()`
