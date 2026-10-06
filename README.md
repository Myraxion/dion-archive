# Dion (descript.ion CLI)

Dion 是一款轻量、高性能、零外部运行依赖的 Windows 原生命令行工具，主用于以 Total Commander 的 **UTF-8 Unicode 编码** 标准查看、设置、编辑与维护 `descript.ion` 文件备注。

## 文档导航

- 📘 [完整设计规约 (Design Specification)](docs/design.md)
- 📖 [领域词汇表 (Domain Glossary)](GLOSSARY.md)
- 🏛️ [架构决策记录 (Architecture Decision Records)](docs/adr/)
  - [ADR-0001: 严格 UTF-8 编码策略](docs/adr/0001-utf8-only-encoding.md)
  - [ADR-0002: 原子替换与隐藏属性不变性](docs/adr/0002-atomic-write-and-hidden-attribute.md)
  - [ADR-0003: 仅认真实换行作为多行边界](docs/adr/0003-literal-newlines-only.md)
  - [ADR-0004: 统一修改输入通道与无头支持](docs/adr/0004-unified-input-channels.md)
  - [ADR-0005: Unset 幂等性与条目顺序保持](docs/adr/0005-idempotent-unset-and-order-preservation.md)
  - [ADR-0006: 严格四级退出码与统一 JSON 契约](docs/adr/0006-standardized-exit-codes-and-json-contract.md)
  - [ADR-0007: 损坏数据拒绝写入原则](docs/adr/0007-fail-safe-on-malformed-data.md)

## 核心特性

- **TC 标准 UTF-8 兼容**：严格支持 `0xEFBBBF0D0A` 头部、多行转义 `\n`、特殊字符及 `0x04C382` 应用标记。
- **Agent & 脚本亲和**：支持管道标准输入（`--stdin` / `-`）、确定性四级退出码及纯净输出，支持 `--json`。
- **可靠与安全**：同目录临时文件原子重命名覆盖，始终保持 Windows `Hidden` 属性，4096 字节单行硬限制校验。
- **极致轻量**：无外部运行时依赖，静态编译单文件体积 < 1MB，冷启动 < 5ms。

## 快速上手 (Quick Start)

### 1. 查询文件备注 (`get`)
```bash
# 查看指定文件或文件夹的备注（别名：cat, view）
dion get file.txt

# 输出包含路径、文件名与备注的结构化 JSON 对象
dion get file.txt --json

# 查看未解码的物理存储格式（TC 原生 raw 字符串）
dion get file.txt --raw

# 静默模式（备注不存在时不输出错误，退出码仍为 1）
dion get file.txt -q
```

### 2. 设置与更新备注 (`set`)
```bash
# 设置单行备注（直接传参）
dion set file.txt "项目主文档"

# 从标准输入管道接收复杂多行文本（支持任意引号与特殊符号）
type notes.txt | dion set file.txt -
echo "第一行`n第二行" | dion set file.txt --stdin

# 调起系统默认文本编辑器进行交互式多行排版编辑
dion set file.txt -e
```

### 3. 删除备注 (`unset`)
```bash
# 幂等删除备注条目（别名：rm, del；条目已删除清空时自动清理物理 descript.ion）
dion unset file.txt
```

### 4. 目录扫描与递归列表 (`list`)
```bash
# 列出当前目录直接附属的全部备注（两列整齐对齐，别名：ls）
dion list

# 列出指定目录下的备注
dion list path/to/dir

# 递归遍历子目录树（默认不跨越符号链接与 Junction 重解析点，显示相对路径）
dion list -r

# 输出标准 JSON 数组（适合 Agent 和自动化脚本精准解析）
dion list -r --json
```

## 退出码契约 (Exit Codes)

| 退出码 | 语义 | 说明 |
| :---: | :--- | :--- |
| `0` | **成功 (Success)** | 操作成功完成；`unset` 目标不存在时亦幂等返回 0 |
| `1` | **未找到 (Not Found)** | `get` 查询的目标文件或目录未标注任何备注 |
| `2` | **用法错误 (Usage Error)** | 参数格式错误、输入源冲突或参数缺失 |
| `3` | **数据/IO 错误 (Data/IO Error)** | 文件损坏、非法编码、超 4096 字节限制或文件系统 I/O 异常 |

## 许可证 (License)

本项目遵循双重开源许可协议：
- [MIT 许可证](LICENSE-MIT) ([`LICENSE-MIT`](LICENSE-MIT))
- [Apache 2.0 许可证](LICENSE-APACHE) ([`LICENSE-APACHE`](LICENSE-APACHE))

您可以自由选择任一协议进行使用和分发。
