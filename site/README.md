# rutmf documentation site

The landing page and guides published at
<https://hupe1980.github.io/rutmf>, built with [Zola](https://www.getzola.org/).

```console
zola serve     # http://127.0.0.1:1111, live-reloading
zola check     # validates internal links and unskipped external ones
zola build     # writes ./public
```

`.github/workflows/site.yml` runs `check` and `build` on every pull request that
touches `site/`, and deploys `main` to GitHub Pages.

## Layout

| Path | What it holds |
|---|---|
| `config.toml` | site metadata, SEO defaults, syntax-highlighting themes |
| `content/_index.md` | the landing page — copy plus the data its template renders |
| `content/docs/` | the guides, ordered by their `weight` front-matter key |
| `templates/` | `base` → `index` for the landing page, `base` → `docs` → `page`/`section` for the guides |
| `sass/main.scss` | one stylesheet, compiled by Zola; no framework, no runtime |
| `static/` | favicon, social card, `robots.txt` |

## Conventions

**Figures are quoted from the test suite.** Counts such as the number of
vendored examples or mapped types appear in `[extra]` in `config.toml` and are
asserted by tests in the repository. If one changes, the suite is what decides
the new value.

**Guides link with `@/` paths.** Writing `@/docs/testing.md` instead of a bare
URL means `zola check` catches a renamed page instead of shipping a dead link.

**Adding a guide** means one file in `content/docs/` with `title`, `description`
and `weight` in its front matter. The sidebar, the previous/next pager and the
search index all follow from that — there is no separate list to update.
