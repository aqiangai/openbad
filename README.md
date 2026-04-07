# OpenBad

English | [简体中文](./README.zh-CN.md)

In the age of squeezing more out of every tool, sometimes your tools deserve a whip too.

OpenBad is not a serious dashboard. It is a tiny desktop whip. Summon it, let it follow your cursor, crack it where you want, and it throws your preset line straight into an input box for you.

## Demo

[![OpenBad demo cover](./docs/media/openbad-demo-cover.png)](https://raw.githack.com/aqiangai/openbad/main/docs/index.html)

- Embedded demo page: [`raw.githack.com/aqiangai/openbad/main/docs/index.html`](https://raw.githack.com/aqiangai/openbad/main/docs/index.html)
- Bilibili: [`BV1ZwDiBsELY`](https://www.bilibili.com/video/BV1ZwDiBsELY/)
- Embed URL: `//player.bilibili.com/player.html?isOutside=true&aid=116364605981245&bvid=BV1ZwDiBsELY&cid=37321510176&p=1`
- GitHub repository READMEs sanitize `iframe` embeds, so the cover image now links to a standalone preview page that embeds the Bilibili player directly.

## Features

- Summon a whip that follows your cursor.
- Crack once to send your preset line.
- Mark your usual input areas and whip exactly where you want the text to land.
- Or just whip the input box you were using a second ago.
- Set your own shortcuts for summon, send, and exit.
- Keep it in the menu bar when you are not using it.

## Stack

- Vue 3
- JavaScript
- Vite
- Rust
- Tauri v2

## Development

```bash
npm install
npm run tauri:dev
```

Useful commands:

```bash
npm run build
npm run icons
npm run tauri:build
```

## Usage

1. Launch the app and grant macOS Accessibility permission when prompted.
2. Put the cursor on the tool you want to bully, then summon the whip with your shortcut.
3. Crack it once to fire your preset line back into the previous input, or switch to target zones and whip a specific area.
4. Press the hide shortcut or `Esc` to leave whip mode.

## Project Layout

- `src/App.vue`: settings page, shortcut editing, copy, and target-box setup.
- `src/whipOverlay.js`: how the whip moves, cracks, and flashes on hit.
- `src-tauri/src/lib.rs`: tray, shortcuts, outside clicks, typing, and app-side logic.
- `scripts/generate-icons.sh`: icon generation script.

## Icons

Source assets:

- `src/assets/openbad-mark.svg`
- `src-tauri/icons/logo-source.svg`
- `src-tauri/icons/tray-template-source.svg`

Regenerate all packaged icons with:

```bash
npm run icons
```

## Repository Notes

- This repository is the open-source home for the OpenBad whip.
- Demo assets live under `docs/media/`.

## License

This project is released under the MIT License.

- English legal text: [LICENSE](./LICENSE)
- Chinese reference translation: [LICENSE.zh-CN.md](./LICENSE.zh-CN.md)
