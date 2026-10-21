# README screenshots

Real lazykeifu PTY output, replayed in xterm.js and captured with Chrome. The `trail` repository and Mina's commits are synthetic. The capture script drives the debug server and asserts the screen contents before rendering.

Adapted from keifu's [capture tools](../screenshots/README.md); the original screenshots remain there for the archived upstream README.

```sh
cargo build --locked
npm install --prefix /path/to/capture-tools @xterm/xterm@5.5.0 @xterm/addon-unicode11@0.8.0 @xterm/addon-canvas@0.7.0 playwright-core@1.56.1
python3 docs/demo/capture.py \
  --node-modules /path/to/capture-tools/node_modules \
  --chrome /path/to/chrome \
  --font-dir /path/to/fonts
```

The font directory must contain `MoralerspaceArgon-Regular.ttf`, `MoralerspaceArgon-Bold.ttf` ([Moralerspace](https://github.com/yuru7/moralerspace), v2.0.0 used here), and `NotoColorEmoji.ttf`. Fonts are not bundled. Output: `overview.png`, `diff.png`, `commit.png`; temporary ANSI recordings and debug dumps are retained at the path printed by the script.
