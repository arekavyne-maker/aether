# Aether — Website

This folder contains the website narrative and a working one-page design preview.

## Files

| File | Purpose |
|---|---|
| `COPY.md` | The full copy deck: positioning, section-by-section wording, template mapping, claims ledger, visual direction, and launch checklist. |
| `index.html` | A self-contained, responsive one-page implementation of the narrative. No build step, no framework. |

## Preview locally

```bash
cd site
python3 -m http.server 8765
```

Open `http://localhost:8765/index.html`.

## What is real in this preview

- All copy is derived from the repository's architecture, contracts, reference runs, and status documents.
- The numbers used on the page are the project's own: 24 contracts, 12 invariants, 11 benchmarks (optional-tool skips explicit), a 24-step egress-free dark build, 15 reference plugins, and three real physics implementations.
- The page deliberately labels the spanwise twin demonstration as `SYNTHETIC` and states the current build as Phase 0–2 substrate + golden path + scientific loop, now extended with the first Aether 3.0 Phase 1 primitives.
- No team names, customer logos, testimonials, awards, or prices are invented. Team cards use role monograms; engagement tiers use `First run` and `Your site` instead of fabricated prices.

## Before launch

1. **Wire the contact form.** The preview intercepts submit and displays a notice. Replace it with your endpoint and add a privacy note.
2. **Replace the domain.** Set the canonical URL and Open Graph URL to the production domain, and add a 1200×630 OG image.
3. **Self-host the fonts.** The page currently loads Inter and Instrument Serif from Google Fonts. For an air-gapped or offline-first deployment, self-host the WOFF2 files and remove the Google Fonts links.
4. **Add real documentation links.** The footer's Architecture, Contracts, Benchmarks, and License links currently point to on-page anchors.
5. **Supply team details.** Add names, portraits, and real affiliations when approved.
6. **Confirm claims at launch.** Benchmark counts and test counts are build-specific; re-check them against [`docs/verification/STATUS.md`](../docs/verification/STATUS.md) and [`workflows/paper-run/gaps.md`](../workflows/paper-run/gaps.md).
7. **Legal review.** Confirm the proprietary-license footer line and any funding/affiliation language.
8. **Keep the page and the status document in sync.** The page is a narrative; `docs/verification/STATUS.md` is the source of truth for counts.

## Design notes

- The visual language mirrors the supplied template: near-black canvas, oversized type, one italic serif phrase per section, hairline outlined cards, roman numerals, and a single iridescent focal object.
- The hero object is a bespoke SVG: an engineering sphere made of graph nodes and orbital rings, with a stylized wing-rib section inside and one bright thread passing through it. It is not a stock render.
- Motion is limited to a slow thread animation, a floating orb, and section reveals. `prefers-reduced-motion` disables all of it.
- The page is responsive down to 360px. The nav collapses to a menu below 960px.
