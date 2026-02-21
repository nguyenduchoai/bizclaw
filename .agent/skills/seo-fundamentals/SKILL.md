---
name: seo-fundamentals
description: SEO best practices, technical SEO, schema markup, and search optimization. Use when implementing SEO, creating sitemaps, adding meta tags, or optimizing for search engines.
---

# SEO Fundamentals Skill

> Master search engine optimization for better visibility and rankings.

## When to Activate

- Building new websites/pages
- Optimizing existing content
- Implementing structured data
- Creating sitemaps
- Fixing SEO issues

## Essential Meta Tags

```html
<head>
  <!-- Title: 50-60 characters, keyword-first -->
  <title>Primary Keyword - Secondary | Brand</title>
  
  <!-- Description: 150-160 characters, compelling with CTA -->
  <meta name="description" content="Unique, engaging description with keywords and call-to-action." />
  
  <!-- Canonical URL -->
  <link rel="canonical" href="https://example.com/page" />
  
  <!-- Open Graph -->
  <meta property="og:title" content="Title for Social" />
  <meta property="og:description" content="Description for social sharing" />
  <meta property="og:image" content="https://example.com/og-1200x630.jpg" />
  <meta property="og:url" content="https://example.com/page" />
  
  <!-- Twitter Card -->
  <meta name="twitter:card" content="summary_large_image" />
  
  <!-- Robots -->
  <meta name="robots" content="index, follow" />
</head>
```

## Next.js Metadata API

```tsx
// Static metadata
export const metadata: Metadata = {
  title: 'Page Title',
  description: 'Page description',
  openGraph: {
    title: 'OG Title',
    description: 'OG Description',
    images: [{ url: '/og.jpg', width: 1200, height: 630 }],
  },
};

// Dynamic metadata
export async function generateMetadata({ params }): Promise<Metadata> {
  const product = await getProduct(params.id);
  return {
    title: product.name,
    description: product.description,
  };
}
```

## Structured Data (JSON-LD)

### Organization
```json
{
  "@context": "https://schema.org",
  "@type": "Organization",
  "name": "Company",
  "url": "https://example.com",
  "logo": "https://example.com/logo.png"
}
```

### Product
```json
{
  "@context": "https://schema.org",
  "@type": "Product",
  "name": "Product Name",
  "offers": {
    "@type": "Offer",
    "price": "99.99",
    "priceCurrency": "USD"
  }
}
```

### Article
```json
{
  "@context": "https://schema.org",
  "@type": "Article",
  "headline": "Article Title",
  "author": { "@type": "Person", "name": "Author" },
  "datePublished": "2026-01-21"
}
```

## Technical SEO Checklist

- [ ] HTTPS enabled
- [ ] Mobile-friendly (responsive)
- [ ] Core Web Vitals passing
- [ ] sitemap.xml created and submitted
- [ ] robots.txt configured
- [ ] Canonical URLs set
- [ ] No broken links (404s)
- [ ] Fast page load (< 3s)
- [ ] Clean URL structure
- [ ] Heading hierarchy (H1 → H6)

## sitemap.xml

```xml
<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://example.com/</loc>
    <lastmod>2026-01-21</lastmod>
    <priority>1.0</priority>
  </url>
</urlset>
```

## robots.txt

```txt
User-agent: *
Allow: /
Disallow: /api/
Disallow: /admin/

Sitemap: https://example.com/sitemap.xml
```

## Best Practices

| Element | Best Practice |
|---------|--------------|
| Title | Unique, 50-60 chars, keyword first |
| URL | Clean, readable, includes keyword |
| H1 | One per page, includes keyword |
| Images | Descriptive alt text, WebP format |
| Links | Descriptive anchor text |
| Content | 300+ words, unique, valuable |
