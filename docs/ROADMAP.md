# Darklight roadmap

Each phase ends with something a person can run and judge. Phases are not time-boxed; a phase is
done when its exit criteria are met. Scope inside a phase can shift, exit criteria should not.

## Phase 0: foundations (this commit)

- [x] MIT license, README, spec, architecture, roadmap
- [x] Cargo workspace with engine crates (`html`, `css`, `layout`, `paint`, `net`) and the app shell
- [x] Window opens on Linux through `winit` + `wgpu`; Android entry point written
- [x] CI: format, clippy, tests
- [x] CI: Linux AppImage build (workflow written, first run pending)
- [x] CI: Android APK build (workflow written, first run pending)

**Exit criteria:** CI is green on `main`, the AppImage starts on a Linux desktop, the APK installs
and opens on an Android 8+ device and shows the dark window. The Android and AppImage builds have not
been exercised end to end yet; the first CI run is the real test, expect small fixes.

## Phase 1: v0.1 "static pages"

Goal: load a static HTML/CSS page from the network and read it. Covers FR-1 to FR-12 and FR-20 to
FR-23 in the spec.

1. **Real HTML parsing.** *In progress.*
   - [x] Insertion-mode tree builder: implied `html`/`head`/`body`, scope checks, implied end
     tags, `li`/`dd`/`dt`/`p`/heading handling, table-lite, RCDATA for `title`/`textarea`,
     nesting depth limit, html5lib-format tree dump, 3000-case garbage-input test.
   - [ ] Adoption agency algorithm and active formatting elements (`<b><i></b></i>`).
   - [ ] Real table insertion modes (implied `tbody`, foster parenting), `select`, `template`.
   - [ ] Tokenizer as a proper state machine (named character references table, CDATA, bogus
     comments, EOF-in-tag rules), then encoding sniffing.
   - [ ] Run the html5lib tree-construction corpus in CI with an allow-list of known failures.
2. **CSS Syntax L3 tokenizer**, replace the hand-rolled rule parser; selectors with combinators and
   attributes; `darklight-style` crate with cascade, inheritance, computed values, UA stylesheet.
3. **Text.** `fontdue` + bundled Inter, glyph atlas on the GPU, line breaking.
4. **Layout.** Block formatting context, inline formatting context, margin collapsing, floats
   deferred. Box tree plus display list generation.
5. **Renderer.** Turn the display list into `wgpu` draw calls (rects, glyphs, borders, images);
   software canvas as the test reference.
6. **Images.** PNG and JPEG via the `png` and `zune-jpeg` crates.
7. **Shell.** Address bar, back/forward/reload, scrolling, link clicks, error pages, network on a
   worker thread.
8. **Quality gates.** Fuzz the HTML and CSS tokenizers, `cargo-deny`, reftests.

**Exit criteria:** Darklight loads and legibly renders a handful of static pages (a plain-HTML
article, a documentation page, the Darklight README rendered as HTML) on Linux and Android,
without crashing on any input from the fuzz corpus.

## Phase 2: v0.2 "everyday basics"

- Tabs, bookmarks, persistent history.
- Flexbox, more selectors and properties, `position`, floats, tables.
- Forms and form submission (GET, POST), cookies.
- Web fonts (WOFF2), SVG as images, GIF/WebP.
- Find in page, text selection, copy.
- Proper Android UI chrome (touch-friendly toolbar, share, downloads).
- Release signing for Android with a real keystore; consider F-Droid and Play listings.

## Phase 3: v0.3 "scripts"

- JavaScript engine from scratch: lexer, parser, AST, bytecode compiler, VM, garbage collector.
  Staged against test262 (expressions first, then functions and closures, objects and prototypes,
  then ES2015+ features).
- DOM bindings, events, timers, `fetch`, `localStorage`.
- Same-origin policy and CSP basics.

## Phase 4: v0.4+ "hardening"

- Multi-process architecture with OS sandboxing.
- Developer tools (DOM inspector, network panel, console).
- CSS Grid, animations/transitions, canvas 2D.
- Accessibility tree.
- Performance work: incremental layout, GPU rasterization, parallel style.

## Decisions

Recorded so we do not relitigate them. Change one by editing this list and `SPEC.md` together.

- **Images:** use small permissive crates, `png` and `zune-jpeg` (both MIT/Apache-2.0). Image
  decoding is plumbing, not the engine.
- **Fonts:** `fontdue` for rasterization (pure Rust, MIT/Apache-2.0/Zlib) with our own glyph atlas
  on the GPU. Shaping with `rustybuzz` is deferred to Phase 2; Phase 1 does simple left-to-right
  layout. Bundled UI and default page font: **Inter** (SIL OFL 1.1), with a system-font fallback
  later. Fallback fonts are a Phase 2 item.
- **Name:** no name-collision check is planned; the project stays "Darklight".

## Open questions

Each gets a decision (and an entry in `SPEC.md`) before the work that needs it starts.

1. **Rendering approach.** Immediate-mode wgpu with a glyph atlas and instanced rects (simple) versus
   a tile-based compositor (faster to scroll, much more work). Default: simple first.
2. **Android build tooling.** `cargo-apk` is used today. If it stalls, alternatives are `xbuild` or
   `cargo-ndk` with a Gradle project (which also gives us a path to AAB for the Play Store).
3. **Search and default homepage.** Which search engine (if any) in the address bar, and whether the
   default page is local. Privacy-preserving default needed. *Phase 2.*
4. **Contribution policy.** DCO sign-off or no formality, code of conduct, issue templates.
