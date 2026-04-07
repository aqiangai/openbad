# OpenBad

[English](./README.md) | 简体中文

OpenBad 是一个基于 Tauri v2 的桌面挥鞭助手，当前主要面向 macOS 的快速文本注入工作流。它会渲染一个全屏透明鞭子 overlay，持续跟随鼠标，带有挥击特效，并且可以把预设文本发送到之前选中的输入框，或发送到你预先配置好的屏幕区域。

## 演示视频

[![OpenBad 演示封面](./docs/media/openbad-demo-cover.png)](./docs/media/openbad-demo.mp4)

- 演示文件：[`docs/media/openbad-demo.mp4`](./docs/media/openbad-demo.mp4)
- 视频规格：MP4，101 秒，1104x720，30 fps
- GitHub 仓库 README 里不会自动播放本地视频文件，点击上面的封面图即可打开演示视频。

## 功能

- 透明鞭子 overlay，支持跟随模式、挥击动画、命中特效和音效。
- 全局快捷键可配置，分别控制呼出、退出和手动发送默认文本。
- 目标区域映射：挥鞭命中特定屏幕区域后，自动点击、输入文本并回车。
- “之前选中的输入框” 模式，适合快速切回原输入框并提交文本。
- 支持中英文切换，同时更新设置窗口、托盘和顶部菜单文案。
- 自带应用图标、菜单栏模板图标和浏览器 favicon 资源。

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
2. 先把你要输入文字的目标应用和输入框选好，再按呼出快捷键拉起鞭子 overlay。
3. 挥鞭后会把默认文本发回之前选中的输入框；如果切到“预设区域”模式，则会按区域配置点击并发送对应文本。
4. 按退出快捷键或 `Esc` 可以随时离开鞭子模式。

## macOS 说明

- 当前自动化能力优先针对 macOS。外部点击和文本输入由 Core Graphics 事件完成。
- 如果你要让应用点击别的软件输入框并自动输入，至少要开启“辅助功能”权限。
- 当前版本已经改成原生切回上一个应用，不再依赖 `System Events`。
- 如果你是通过 Terminal、iTerm、Warp 等终端运行 `npm run tauri:dev`，也要给对应终端开启“辅助功能”权限。
- 应用现在使用 `ActivationPolicy::Accessory`，默认只保留菜单栏图标，不显示 Dock 图标。
- 菜单栏图标使用 template PNG，能自动适配 macOS 顶部状态栏的明暗显示。

## 目录结构

- `src/App.vue`：设置窗口 UI、快捷键录制、中英文文案、目标区域编辑。
- `src/whipOverlay.js`：鞭子绘制、运动逻辑、挥击判定、命中特效、目标区域轮廓。
- `src-tauri/src/lib.rs`：托盘和菜单初始化、快捷键注册、macOS 自动化桥接、设置持久化。
- `scripts/generate-icons.sh`：从 SVG 源文件重新生成应用图标、菜单栏模板图标和 favicon。

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

- 这个仓库面向开源发布，主要承载 macOS 优先的 Tauri 版本。
- 演示素材统一放在 `docs/media/` 下，方便仓库同时保留 README 封面图和原始录屏文件。

## 开源协议

本项目采用 MIT 协议开源。

- 英文正式法律文本：[LICENSE](./LICENSE)
- 中文参考翻译：[LICENSE.zh-CN.md](./LICENSE.zh-CN.md)
