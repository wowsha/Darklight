# Darklight architecture

## 1. Pipeline

A page load flows through these stages. Each stage is a pure transformation of data from the
previous stage, which keeps them testable without a window or a network.

```
 URL ──► net ──► bytes ──► decode ──► html tokenizer ──► tree builder ──► DOM
                                                                          │
                          user-agent + author CSS ──► css parser ──► rules│
                                                                          ▼
                                                  style (selector match + cascade)
                                                                          │ styled tree
                                                                          ▼
                                                  layout (box tree, geometry)
                                                                          │ layout tree
                                                                          ▼
                                                  paint (display list)
                                                                          │ display list
                                                                          ▼
                                                  renderer (wgpu / software canvas) ──► window
```

Status today: `net`, `html`, `css` (parse only), box-model types in `layout`, and the
display list plus a software canvas in `paint` exist. The shell wires `html` and `css`
together for the welcome page and clears the window through `wgpu`. `style`, real `layout` and
the display-list renderer are Phase 1.

## 2. Crates

| Crate | Depends on | Responsibility |
| --- | --- | --- |
| `darklight-html` | nothing | Tokenizer, tree builder, arena DOM |
| `darklight-css` | nothing | Stylesheet / selector / value parsing, specificity, simple-selector matching |
| `darklight-layout` | nothing | `Rect`, edge sizes, box model; later the layout algorithms |
| `darklight-paint` | `css`, `layout` | `DisplayList`, software `Canvas` |
| `darklight-net` | `url`, `ureq` | URL input handling, blocking HTTP(S) GET |
| `darklight` | all of the above, `winit`, `wgpu` | App shell: window, event loop, GPU surface; `darklight` binary on Linux, `android_main` on Android |

Planned crates (names provisional):

- `darklight-style`: selector matching against the DOM, cascade, computed values.
- `darklight-text`: font loading, shaping, line breaking.
- `darklight-image`: image decoding, if we write it ourselves (see roadmap).
- `darklight-js-*`: lexer, parser, bytecode, VM, DOM bindings (Phase 3).

Dependency rule: arrows only point down the pipeline. `html` and `css` never depend on layout
or paint; only the shell knows about windowing and GPUs. This is what lets the engine run
in tests, fuzzers and (later) a separate renderer process.

## 3. Key design decisions

### Arena DOM with integer ids

`Document` owns a `Vec<Node>`; nodes refer to each other by `NodeId` (an index). No `Rc<RefCell<..>>`,
no lifetimes leaking through the API, and cheap to clone or send between threads. The cost is that
removing nodes needs care (tombstones or a free list). Revisit when scripts can mutate the DOM.

### Parsers never panic

Tokenizers and tree builders accept any input and recover, as the HTML and CSS specs require. Code
paths that can fail return `Option`/`Result`; indexing is guarded. Fuzzing (Phase 1) enforces this.

### Blocking network, off the UI thread

`darklight-net::fetch` is blocking. The shell will call it from a worker thread and send the result
to the event loop with `winit`'s user events. This keeps the network crate trivial while the UI
stays responsive. Streaming and HTTP/2 are not goals until profiling says otherwise.

### One renderer abstraction, two backends

The display list is renderer-agnostic. The software `Canvas` is the reference implementation for
tests (pixel-exact assertions, no GPU needed). The `wgpu` renderer is what ships. Reftests compare
the two.

## 4. Threading model

Phase 0-2: UI thread (event loop, layout, paint submission) plus short-lived worker threads for
network. Phase 3 adds a JS execution context on the UI thread (single-threaded, like every web
engine's main thread). Phase 4 moves the engine into renderer processes.

## 5. Platform notes

### Linux

- `winit` handles Wayland and X11; the libraries are loaded dynamically at runtime, so the binary does
  not need their development packages to build.
- `wgpu` picks Vulkan, falling back to OpenGL. A GL fallback also makes it work under software
  rasterizers such as llvmpipe, which helps in CI and on machines without GPU drivers.
- Packaged as an AppImage: the binary is bundled with a `.desktop` file and an icon. GPU drivers are
  taken from the host, which is what we want.

### Android

- Entry point is `android_main` (NativeActivity through `android-activity`), compiled as a `cdylib`
  from the same crate (`crate-type = ["lib", "cdylib"]`). `cargo-apk` packages it.
- Lifecycle: Android destroys the native window on `suspended` and creates a new one on `resumed`.
  The shell drops all GPU state in `suspended` and rebuilds it in `resumed`. Anything that must
  survive (page, history, scroll position) lives outside the GPU state.
- Permissions: only `INTERNET`.
- Input: touch becomes scroll and tap events through `winit`; the back button is mapped to history
  back (FR-23, not wired yet).
- The APK is signed with the debug keystore for now (see the roadmap for real release signing).

## 6. Logging

The `log` facade everywhere. `env_logger` on desktop (`RUST_LOG=info`), `android_logger` on Android
(visible with `adb logcat -s darklight`).
