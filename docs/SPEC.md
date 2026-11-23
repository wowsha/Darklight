# Darklight specification

Version 0.1 of this document. It is a living spec: change it in the same pull request as the
behaviour it describes.

Requirement IDs (`FR-*` functional, `NFR-*` non-functional) are stable. Do not renumber.

## 1. Goals and non-goals

### Goals

- A real, usable web browser whose engine is written from scratch in Rust.
- Runs on Linux (x86_64, distributed as an AppImage) and Android (aarch64, distributed as an APK)
  from one codebase.
- Memory-safe by construction: no `unsafe` in the engine crates.
- Private by default: no telemetry, no accounts, no background network traffic the user did not
  initiate.
- Small enough for one person to understand end to end. Prefer simple, correct code over clever
  code.

### Non-goals (for now)

- Matching Chrome, Firefox or Safari in web compatibility. We target a well-defined subset and
  grow it.
- macOS, Windows and iOS builds.
- Browser extensions, sync, password management.
- DRM / EME, WebRTC, WebGPU exposed to pages, WebAssembly (revisit after the JS engine exists).

## 2. From-scratch policy

The line is drawn between the **engine** (ours) and **platform plumbing** (borrowed).

| Area | Policy | Crate(s) |
| --- | --- | --- |
| HTML tokenizer, tree builder, DOM | **Write ourselves** | `darklight-html` |
| CSS parsing, selectors, cascade, computed style | **Write ourselves** | `darklight-css` (+ style crate later) |
| Layout (block, inline, later flex/grid) | **Write ourselves** | `darklight-layout` |
| Display list, painting, compositing | **Write ourselves** | `darklight-paint` |
| JavaScript engine and DOM bindings | **Write ourselves** (Phase 3) | new crates |
| Windowing and input | Borrow | `winit` |
| GPU access | Borrow | `wgpu` |
| URL parsing | Borrow (WHATWG URL is subtle and security-relevant) | `url` |
| HTTP and TLS | Borrow | `ureq`, `rustls` |
| Font rasterization | Borrow (decided) | `fontdue` |
| Text shaping (ligatures, complex scripts) | Borrow, deferred to Phase 2 | `rustybuzz` |
| Image decoding | Borrow small crates (decided) | `png`, `zune-jpeg`; GIF/WebP later |

Rules:

1. Engine crates depend on as little as possible. Today they have **zero** third-party
   dependencies (`darklight-paint` depends only on our own crates).
2. Any new third-party dependency needs a one-line justification in the pull request and must
   have a permissive licence (MIT, Apache-2.0, BSD, ISC, Zlib, Unicode, MPL-2.0 for unmodified
   use). No GPL/AGPL/LGPL. Bundled font files may use the SIL OFL 1.1.
3. If we borrow something in a "write ourselves" row, that is a spec change, not a quiet
   dependency bump.

## 3. Target platforms

| Platform | Artifact | Minimum | Notes |
| --- | --- | --- | --- |
| Linux x86_64 | AppImage | glibc of the CI image (Ubuntu 22.04) | Wayland and X11 via `winit`; needs a Vulkan or OpenGL driver |
| Android aarch64 | APK (sideload) | Android 8.0 (API 26) | NativeActivity via `android-activity`; Vulkan or GLES |

Later candidates: Android armv7 / x86_64 (emulator), Linux aarch64.

## 4. Standards targeted

We follow the specifications below, in the order they are needed. "Subset" means we document
what is and is not implemented in the relevant crate's module docs.

| Layer | Specification | When |
| --- | --- | --- |
| URLs | WHATWG URL Standard (via `url`) | Phase 0 |
| HTTP | RFC 9110 / 9112 (HTTP/1.1), via `ureq` | Phase 0 |
| HTML parsing | WHATWG HTML: tokenization and tree construction | Phase 1 (conformant) |
| Character encoding | WHATWG Encoding (UTF-8 first, then windows-1252, ISO-8859-x) | Phase 1 |
| DOM | WHATWG DOM, read-only subset first | Phase 1 |
| CSS syntax | CSS Syntax Module Level 3 | Phase 1 |
| Selectors | Selectors Level 3, then Level 4 subset | Phase 1 / 2 |
| Cascade and inheritance | CSS Cascading and Inheritance Level 4 | Phase 1 |
| Box model, normal flow | CSS 2.1 chapters 8-10, CSS Display 3 | Phase 1 |
| Text | CSS Text 3, CSS Fonts 4 (subset), Unicode UAX #14 line breaking, UAX #9 bidi | Phase 1 / 2 |
| Flexbox | CSS Flexible Box Layout Level 1 | Phase 2 |
| Forms | HTML forms, Fetch (request side) | Phase 2 |
| JavaScript | ECMAScript (staged subset, test262 as the oracle) | Phase 3 |
| Storage | Cookies (RFC 6265bis), localStorage | Phase 2 / 3 |

## 5. Functional requirements

### 5.1 Milestone v0.1: "static pages"

A page with HTML and CSS but no JavaScript can be loaded from a URL and read.

- **FR-1** The browser fetches `http://` and `https://` URLs. TLS certificate errors are a hard
  failure with an error page. There is no "proceed anyway" button in v0.1.
- **FR-2** Redirects are followed (limit 10). The final URL is shown in the address bar.
- **FR-3** Responses are decoded as UTF-8; invalid bytes become U+FFFD. (Full encoding sniffing
  is Phase 1.)
- **FR-4** HTML is parsed per the WHATWG tree-construction algorithm, including implied
  `html/head/body`, void elements, implied end tags, and error recovery for the common cases.
- **FR-5** `<style>` elements and linked stylesheets (`<link rel=stylesheet>`) are loaded and
  applied. User-agent default stylesheet included.
- **FR-6** Supported selectors: type, universal, class, id, attribute (`=`), descendant and child
  combinators, `:hover` is not required.
- **FR-7** Supported properties (initial): `display` (`block`, `inline`, `none`), `color`,
  `background-color`, `font-size`, `font-weight`, `font-family` (generic families),
  `margin`, `padding`, `border` (solid, width, color), `width`, `height`, `text-align`,
  `line-height`, `text-decoration`.
- **FR-8** Block and inline formatting contexts, line breaking, and margin collapsing per CSS 2.1.
- **FR-9** Text is rendered with anti-aliased glyphs from a bundled font, with fallback.
- **FR-10** Images (`<img>`) in PNG and JPEG are decoded and drawn, with intrinsic sizing.
- **FR-11** Links are clickable and navigate. Scrolling works with wheel, keyboard and touch.
- **FR-12** Window resize and Android rotation re-layout the page.

### 5.2 Browser shell (v0.1)

- **FR-20** One tab. Address bar, back, forward, reload, stop.
- **FR-21** Typing a non-URL in the address bar is treated as a URL (no search engine in v0.1).
- **FR-22** A built-in `about:` page set: `about:blank`, `about:version`.
- **FR-23** Android: back button maps to history back; the app survives suspend/resume without
  crashing (the GPU surface is recreated; see ARCHITECTURE).

### 5.3 Later milestones (summary)

See `ROADMAP.md`: tabs, forms, cookies, flexbox, then the JavaScript engine, then process
isolation and developer tools.

## 6. Non-functional requirements

- **NFR-1 Memory safety.** Engine crates use `#![forbid(unsafe_code)]`. `unsafe` is allowed only
  in the app shell where a platform API requires it (e.g. the Android entry point), and each use
  must have a `// SAFETY:` comment.
- **NFR-2 Robustness.** No page can crash the browser by being malformed. Parsers must never
  panic on any input. Enforced by fuzzing (Phase 1) and by tests with garbage input.
- **NFR-3 Resource limits.** Hard limits on DOM depth, node count, response size, redirect count
  and stylesheet size, so hostile pages cannot exhaust memory.
- **NFR-4 Privacy.** No telemetry, no analytics, no phone-home. Network requests happen only for
  user navigation and the subresources of the page being viewed.
- **NFR-5 Performance (targets, not gates, until Phase 2).** First paint of a typical static article
  under 500 ms on a mid-range phone after bytes arrive; 60 fps scrolling for static pages.
- **NFR-6 Build.** `cargo test --workspace`, `cargo clippy -- -D warnings` and `cargo fmt --check`
  pass on every commit. CI enforces it.
- **NFR-7 Reproducibility.** `Cargo.lock` is committed; CI builds with `--locked`.
- **NFR-8 Licensing.** MIT for our code. Dependency licences checked in CI with `cargo-deny`
  (Phase 1).

## 7. Security model

Phase 0-2 (single process):

- Only `http` and `https` are navigable. `file:`, `data:` navigation and similar are rejected
  until explicitly designed.
- TLS verification is always on, using `rustls` with the platform-independent Mozilla root set.
- Same-origin policy applies as soon as scripts or cross-document access exist (Phase 3).
- Subresource loads follow the page's scheme: an `https` page does not load `http` subresources
  (mixed content blocked).

Phase 4+ (multi-process):

- Renderer processes are separate from the browser process, with OS sandboxing (seccomp on Linux,
  isolated process on Android). The renderer has no direct network or filesystem access; it asks
  the browser process.

## 8. Testing strategy

1. **Unit tests** in every crate (they exist today and run in CI).
2. **Golden tests**: HTML in, serialized DOM out; CSS in, parsed rules out. Phase 1.
3. **html5lib-tests** tree-construction corpus run against the parser. Phase 1.
4. **Reftests** (render a page, compare with a reference page that should look identical). Phase 1.
5. **Fuzzing** with `cargo-fuzz` for the HTML and CSS tokenizers. Phase 1.
6. **test262** for the JS engine. Phase 3.
7. **Web Platform Tests**: a curated subset run in CI, with an allow-list of expected failures that
   only shrinks. Phase 2+.

## 9. Open questions

Tracked in `ROADMAP.md` under "Open questions" so they have one home.
