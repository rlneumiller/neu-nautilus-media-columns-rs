# Rust Nautilus Extension (Nautilus‑4, GTK4, Debian 13.5)

This repository contains a Rust-based Nautilus extension crate built as a native `.so` module for Nautilus 4 on Debian-like systems.

The current implementation is a single crate with manual `GTypeModule` registration and a minimal `NautilusColumnProvider` implementation.

---

## Project Structure

```text
.
  Cargo.toml
  README.md
  neu-install.sh
  src/
    lib.rs
    imp.rs
```

---

## Prerequisites

### System packages

Install Nautilus extension headers, pkg-config, and GObject Introspection tools:

```bash
sudo apt install \
  libnautilus-extension-dev \
  libglib2.0-dev \
  pkg-config \
  cargo
```

Confirm headers exist:

```bash
ls /usr/include/nautilus/libnautilus-extension/
```

Should produce something like:

```text
nautilus-column-provider.h
nautilus-column.h
nautilus-extension-enum-types.h
nautilus-file-info.h
nautilus-info-provider.h
nautilus-menu-provider.h
nautilus-menu.h
nautilus-properties-item.h
nautilus-properties-model-provider.h
nautilus-properties-model.h
```

### Rust toolchain

* Using Rust `edition = "2024"`
* `cargo` is required

This crate currently depends on local path crates from `neu-nautilus-extension-rs` for the Nautilus FFI bindings.

---

## Nautilus Extension Installation

Nautilus loads compiled extensions only from trusted system directories.

### Target path on Debian

```text
/usr/lib/x86_64-linux-gnu/nautilus/extensions-4/
```

The built Rust extension must be installed there.

---

## Building and Installing

### 1. Compile the release library

```bash
cargo build --release
```

Output file: `target/release/libnautilus4_media_columns_rs.so`

### 2. Install the extension

Use the provided install script from the repository root:

```bash
sudo ./neu-install.sh
```

The script copies the built `.so` file into `/usr/lib/x86_64-linux-gnu/nautilus/extensions-4/` and restarts Nautilus.

### 3. Uninstall the extension

```bash
sudo rm /usr/lib/x86_64-linux-gnu/nautilus/extensions-4/libnautilus4_media_columns_rs.so
nautilus -q
```

---

## Current implementation

### `src/lib.rs`

Exports the required Nautilus module entry points:
* `nautilus_module_initialize`
* `nautilus_module_shutdown`
* `nautilus_module_list_types`

It registers the extension type using `OnceLock<GType>` and keeps the registered type alive for the duration of the loaded module.

### `src/imp.rs`

Implements manual `GTypeModule` registration for `NautilusMediaColumns` and attaches the `NautilusColumnProvider` interface.

The current column provider returns one stub column:
* `media-duration`

---

## Notes

* The shared library filename is `libnautilus4_media_columns_rs.so`.
* `neu-install.sh` installs the built library into the Debian Nautilus extension path.

