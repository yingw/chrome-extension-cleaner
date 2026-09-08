# Chrome 扩展清理

> [English version](README.md) · [下载](releases/latest)

查找并删除**所有 Chrome Profile 下扩展的旧版本目录**——Chrome 在自动更新扩展时，
有时会把旧的 `<version>_0` 目录留在磁盘上，体积大的扩展（广告拦截、AI 助手、
语法工具等）每次更新都可能留下几百 MB 再也不会被用到的垃圾。独立开源项目，
与 Google 无关。

<table>
<tr>
<td><img src="assets/screenshots/screenshot_1.png" width="380"></td>
<td><img src="assets/screenshots/screenshot_2.png" width="380"></td>
</tr>
<tr>
<td><img src="assets/screenshots/screenshot_3.png" width="380"></td>
<td><img src="assets/screenshots/screenshot_4.png" width="380"></td>
</tr>
</table>

## 安装前须知

支持的平台：macOS Apple Silicon、Windows x64、Linux x64

- 仅支持 **Google Chrome Stable 默认用户目录**，不支持 Edge、Brave、
  Chromium、Beta、Dev、Canary，也不支持自定义 `--user-data-dir`。
- 构建为**未签名版本，无自动更新**。首次启动时 macOS Gatekeeper 与
  Windows SmartScreen 都会警告，在你的系统安全设置里允许一次即可。
- 本工具只删除扩展的旧版本目录，**不会**卸载扩展、不动设置、不读取浏览数据。

## 安装

- **macOS**（`.dmg`）：打开镜像，把应用拖到应用程序。首次启动右键应用 →
  打开 → 确认（或去系统设置 → 隐私与安全性里允许一次）。
- **Windows**（`.exe`）：SmartScreen 会警告——点“更多信息” → “仍要运行”。
- **Linux**（AppImage / `.deb`）：支持的发行版和 WebView 依赖见 release 说明。

## 用法

1. **建议先完全退出 Chrome。** Chrome 仍在运行时应用会警告。
2. 打开应用——自动扫描所有 Chrome Profile，按可释放空间列出有旧版的扩展。
3. 勾选旧版本（已验证的默认勾上），点**清理**，核对清单后开始清理。

要卸载**整个扩展**，请用 Chrome 的扩展管理页
（`chrome://extensions`）；本工具不卸载扩展。

## 安全与隐私

只删你明确勾选的：

- 严格**旧于**在用版本的目录，且来自已验证的标准 Chrome 应用店安装：
  目录名、自己的 `manifest.json`、Chrome 自己的 `Secure Preferences`
  记录三方一致。

永远不删：

- 任何扩展的在用版本，或**新于**在用版本的目录（可能是 Chrome 暂存待切换的）。
- 无法验证的扩展、未知目录结构（如 `Temp`）、非标准安装
  （企业策略、外部 CRX、开发者模式）——这些只展示，不可删。
- 扩展设置、浏览数据、IndexedDB、缓存，以及
  `<Profile>/Extensions/<id>/<version>_0` 之外的任何东西。

每个目录在删除前一刻会重新验证，逐个删除并逐项报告结果，每个扩展至少保留
一个版本。所有扫描与删除**完全在本机完成**——无遥测，也不上传你的 Chrome
配置、Profile 或扩展清单。Chrome 运行时，更新进行中或配置写回可能干扰——
退出 Chrome 即可避免该风险。

安全政策见 [SECURITY.md](SECURITY.md)。

## 限制与支持

- 首次启动时 Windows 可能为 `msedgewebview2.exe` 弹防火墙提示。那是系统
  WebView 运行时，不是本应用——本应用没有任何网络请求，拒绝即可。
- 发现 bug？开 issue。**不要**附上你的 Chrome Profile、账号邮箱或
  `Secure Preferences` 文件——它们可能含个人隐私。
- 删除是永久的，无法撤销。不确定就先备份 Chrome Profile。

## 协议

MIT——见 [LICENSE](LICENSE)。

## 更新日志

### 1.0.0
首个正式版本（2026-09-08）。
