# OpenBad

[English](./README.md) | 简体中文

降本增效时代，工具也需要鞭笞。

OpenBad 不是一个正经八百的效率面板，它就是一根桌面小鞭子。你把它呼出来，它就跟着鼠标跑；你往哪儿甩，它就往哪儿抽。抽中了，就帮你把提前写好的话打进输入框里，顺手再回个车。

## 演示视频

[![OpenBad 演示封面](./docs/media/openbad-demo-cover.png)](https://raw.githack.com/aqiangai/openbad/main/docs/index.html)

- 演示页：[`raw.githack.com/aqiangai/openbad/main/docs/index.html`](https://raw.githack.com/aqiangai/openbad/main/docs/index.html)
- B 站视频：[`BV1ZwDiBsELY`](https://www.bilibili.com/video/BV1ZwDiBsELY/)
- 嵌入地址：`//player.bilibili.com/player.html?isOutside=true&aid=116364605981245&bvid=BV1ZwDiBsELY&cid=37321510176&p=1`
- GitHub 仓库 README 会清洗 `iframe`，所以现在封面图会跳转到独立演示页，在那里直接嵌入 B 站播放器。

## 功能

- 呼出一根会跟着鼠标跑的鞭子。
- 甩一下就能打出你预设好的内容。
- 可以先圈好常用输入框，抽到哪里就打到哪里。
- 也可以直接抽回你刚刚正在用的输入框。
- 快捷键自己配，想怎么呼出、怎么退出都行。
- 常驻菜单栏，不需要的时候就安静待着。

## 技术栈

- Vue 3
- JavaScript
- Vite
- Rust
- Tauri v2

## 开发

```bash
npm install
npm run tauri:dev
```

常用命令：

```bash
npm run build
npm run icons
npm run tauri:build
```

## 使用方式

1. 启动应用，并在 macOS 弹出提示后授予“辅助功能”权限。
2. 先把你要鞭打的工具和输入框选好，再按快捷键把鞭子叫出来。
3. 甩一下，默认就会把你写好的那句“鞭策话术”打回去；如果你配了区域，就按你抽中的地方来发。
4. 按退出快捷键或 `Esc` 可以随时离开鞭子模式。

## 目录结构

- `src/App.vue`：设置页，改文案、改快捷键、改你想抽哪儿都在这里。
- `src/whipOverlay.js`：鞭子怎么动、怎么甩、怎么出特效，主要看这里。
- `src-tauri/src/lib.rs`：菜单栏、快捷键、点外部输入框、自动打字这些底层逻辑在这里。
- `scripts/generate-icons.sh`：重新生成图标用的脚本。

## 图标资源

源文件：

- `src/assets/openbad-mark.svg`
- `src-tauri/icons/logo-source.svg`
- `src-tauri/icons/tray-template-source.svg`

重新生成所有打包图标：

```bash
npm run icons
```

## 仓库说明

- 这个仓库就是 OpenBad 这根小鞭子的开源版本。
- 演示素材放在 `docs/media/` 下，方便直接看效果。

## 开源协议

本项目采用 MIT 协议开源。

- 英文正式法律文本：[LICENSE](./LICENSE)
- 中文参考翻译：[LICENSE.zh-CN.md](./LICENSE.zh-CN.md)
