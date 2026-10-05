# Dion (descript.ion CLI)

Dion 是一款轻量、高性能、零外部运行依赖的 Windows 原生命令行工具，主用于以 Total Commander 的 **UTF-8 Unicode 编码** 标准查看、设置、编辑与维护 `descript.ion` 文件备注。

## 文档导航

- 📘 [完整设计规约 (Design Specification)](file:///d:/Workspace/code/dion/docs/design.md)
- 📖 [领域词汇表 (Domain Glossary)](file:///d:/Workspace/code/dion/GLOSSARY.md)
- 🏛️ [架构决策记录 (Architecture Decision Records)](file:///d:/Workspace/code/dion/docs/adr/)
  - [ADR-0001: 严格 UTF-8 编码策略](file:///d:/Workspace/code/dion/docs/adr/0001-utf8-only-encoding.md)
  - [ADR-0002: 原子替换与隐藏属性不变性](file:///d:/Workspace/code/dion/docs/adr/0002-atomic-write-and-hidden-attribute.md)
  - [ADR-0003: 仅认真实换行作为多行边界](file:///d:/Workspace/code/dion/docs/adr/0003-literal-newlines-only.md)
  - [ADR-0004: 统一修改输入通道与无头支持](file:///d:/Workspace/code/dion/docs/adr/0004-unified-input-channels.md)
  - [ADR-0005: Unset 幂等性与条目顺序保持](file:///d:/Workspace/code/dion/docs/adr/0005-idempotent-unset-and-order-preservation.md)
  - [ADR-0006: 严格四级退出码与统一 JSON 契约](file:///d:/Workspace/code/dion/docs/adr/0006-standardized-exit-codes-and-json-contract.md)
  - [ADR-0007: 损坏数据拒绝写入原则](file:///d:/Workspace/code/dion/docs/adr/0007-fail-safe-on-malformed-data.md)

## 核心特性

- **TC 标准 UTF-8 兼容**：严格支持 `0xEFBBBF0D0A` 头部、多行转义 `\n`、特殊字符及 `0x04C382` 应用标记。
- **Agent & 脚本亲和**：支持管道标准输入（`--stdin` / `-`）、确定性四级退出码及纯净输出，支持 `--json`。
- **可靠与安全**：同目录临时文件原子重命名覆盖，始终保持 Windows `Hidden` 属性，4096 字节单行硬限制校验。
