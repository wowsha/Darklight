# Darklight

A web browser written from scratch in Rust. MIT licensed.

> **Status: pre-alpha (v0.0.1).** Today Darklight opens a window on Linux and Android and
> runs a tiny HTML/CSS parsing pipeline. It cannot browse the web yet. The plan is in
> [`docs/`](docs/).

## What "from scratch" means here

We write the browser engine ourselves: HTML parsing, the DOM, CSS parsing and cascade,
layout, painting, and later a JavaScript interpreter. We use existing crates only for
platform plumbing: windowing (`winit`), GPU access (`wgpu`), networking and TLS
(`ureq`/`rustls`/`url`), and later fonts. The full policy is in
[`docs/SPEC.md`](docs/SPEC.md#2-from-scratch-policy).

## Documents

| Document | Contents |
| --- | --- |
| [`docs/SPEC.md`](docs/SPEC.md) | Goals, requirements, standards targeted, security model |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Crate layout, rendering pipeline, platform notes |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Phases, milestones, exit criteria, open questions |

## Repository layout

```
crates/
  darklight/          app shell: window, event loop, wgpu surface (Linux bin + Android lib)
  darklight-html/     HTML tokenizer, tree builder, DOM
  darklight-css/      CSS parser and value types
  darklight-layout/   box model and (soon) layout
  darklight-paint/    display list and software canvas
  darklight-net/      URL handling and HTTP(S) fetching
packaging/linux/      .desktop file and icon used by the AppImage build
.github/workflows/    CI, AppImage and APK builds
```

## Building

You need a recent stable Rust toolchain (`rustup` picks it up from `rust-toolchain.toml`).

```sh
cargo run            # run the browser on Linux
cargo test --workspace
```

### Linux AppImage

CI builds an x86_64 AppImage on every push to `main` and attaches it to tagged releases.
To build one locally, follow the steps in
[`.github/workflows/release.yml`](.github/workflows/release.yml) (the `appimage` job).

### Android APK

CI builds an `aarch64` APK with [`cargo-apk`](https://github.com/rust-mobile/cargo-apk).
Locally you need the Android SDK and NDK, the `aarch64-linux-android` Rust target, and:

```sh
cargo install cargo-apk --locked
cargo apk build --release --lib -p darklight
```

The APK is signed with the Android debug keystore (`~/.android/debug.keystore`). That is fine
for sideloading and testing, and not suitable for an app store. See the roadmap.

## Contributing

Issues and pull requests are welcome. Please read the spec first, and keep engine crates
free of `unsafe` (`#![forbid(unsafe_code)]` is enabled in all of them).

## License

[MIT](LICENSE)
