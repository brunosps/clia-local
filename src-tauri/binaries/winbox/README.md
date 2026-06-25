# Bundled WinBox CLI

This directory holds the `winbox` CLI binary that the **Deploy → Máquinas** panel uses to
create and manage local VMs. The binary is the `winbox-gui` release build, which exposes a
`--json` envelope contract (`version`, `distros`, `host-health`, `viewer-url`, `install`,
`start`, `stop`, `list`, …).

The binary itself is **not committed** (it is git-ignored, like the bundled RTK binary). It
is produced by:

```bash
corepack pnpm winbox:prepare
```

which copies the built `winbox-gui` binary into this folder. Set `WINBOX_GUI_BIN` to point at
a specific build, or keep a `winbox-gui` checkout next to this repo (built with
`cargo build --release`).

At runtime the app resolves this bundled binary automatically (no `WINBOX_BIN` needed).
Running the VMs themselves still requires **Docker** (+ KVM/QEMU) on the host.
