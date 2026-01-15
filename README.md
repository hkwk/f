# f — File Manager (GUI)

`f` 是一个轻量级的双面板文件管理器（GUI），使用 Rust + `iced` 构建，参考 Total Commander 的交互模型。

## 功能 ✨

- 双面板文件浏览（左右面板）
- 文件筛选/搜索（支持输入匹配）
- 复制、删除、刷新等基本文件操作
- 可扩展：将会加入重命名、移动、预览、键盘快捷键等功能

## 安装

使用 cargo（开发/测试）：

```bash
cargo run --release
```

将来会支持通过 `cargo install` 或二进制发布。

## 使用示例

运行程序后会打开窗口，左右两侧列表分别代表两个目录。点击文件选择并使用下方按钮执行复制、删除或刷新操作。

## 致开发者

欢迎贡献：提交 issue、PR 或在 `feature/*` 分支上工作。 请遵循 `rustfmt` 和 `clippy` 规则。

## 许可证

本项目遵循 GPLv3 许可（详见 `LICENSE`）。

---

更多文档与 Roadmap 将在 crates.io 与 docs.rs 页面完善。