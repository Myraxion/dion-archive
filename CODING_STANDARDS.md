# Rust Best Practices for Dion

Dion 是一个无外部运行时依赖、追求冷启动 < 5ms、单二进制 ≤ 2MB 的 Windows 原生极简 CLI。编写与审查 Dion 的 Rust 代码时，严格遵守以下准则。

---

## 1. 核心心智：KISS 与反过度抽象
- **重复优于错误的抽象**：遵循 Rule of Three（三次法则）。一段代码仅出现 1~2 次时保持内联；不要为了消除几行表象相似的代码而引入泛型、布尔标志位（Flag Arguments）或复杂的 Trait。
- **拒绝过度工程**：Dion 是极简同步工具，禁止引入复杂的类型状态模式（Type State）、多层 Trait 抽象或异步运行时（如 Tokio）。保持代码直观可读、直接操作数据结构。
- **关注本质生命周期**：根据 `GLOSSARY.md`，数据流转核心围绕 `(ParentDirectory, EntryName)` 与 `Codec` 展开，优先通过纯函数和不可变数据流转换。

---

## 2. 所有权与零成本借用 (Ownership & Memory)
- **参数首选借用**：函数参数优先使用 `&str`、`&[u8]` 或 `&Path`，避免要求所有权 `String`、`Vec<u8>` 或 `PathBuf`。
- **禁止防御性克隆**：
  - 严禁为了平息借用检查器而随意调用 `.clone()`。
  - 需要条件修改时使用 `Cow<'_, str>`（例如转义 `\n` / `\\` 时，未命中转义字符时零分配直接借用原切片）。
- **小类型按值传递**：`Copy` 且体积小（≤ 24 字节）的类型直接按值复制，无需传递引用。
- **避免中间分配**：流式解析时优先链式迭代器，避免在循环体内提前或反复 `.collect::<Vec<_>>()`.

---

## 3. 错误处理与退出码契约 (Error Handling)
- **绝对禁止 `unwrap()` / `expect()`**：除单元测试外，生产代码严禁出现 `unwrap()` 或 `expect()`。
- **基于 `thiserror` 的强类型错误**：
  - 核心解析（`Codec`）、路径处理（`PathResolver`）与存储（`Storage`）模块必须使用 `thiserror` 定义明确的 `Error` 枚举。
  - 禁止在核心层使用 `anyhow`：`anyhow` 会擦除具体错误类型，导致无法按 [ADR-0006](docs/adr/0006-standardized-exit-codes-and-json-contract.md) 精准映射到四级退出码（0: 成功, 1: 未找到, 2: 参数错误, 3: 格式或 I/O 错误）。
- **优先使用 `?` 冒泡错误**：扁平化错误流，避免深度嵌套的 `match` 或 `if let`。

---

## 4. 文件系统与 Windows 契约规范
- **原子写入与隐藏属性**：文件写入必须严格遵守 [ADR-0002](docs/adr/0002-atomic-write-and-hidden-attribute.md)，临时文件创建即设 `FILE_ATTRIBUTE_HIDDEN`，写入完成 `FlushFileBuffers` 后通过 `ReplaceFileW` / `MoveFileExW` 原子覆盖。
- **4096 字节单行硬限制**：严格执行 [ADR-0007](docs/adr/0007-fail-safe-on-malformed-data.md)，每行物理 UTF-8 字节超限或格式损坏时直接拒绝写入并返回退出码 3。
- **编码洁癖**：强制 [ADR-0001](docs/adr/0001-utf8-only-encoding.md) 策略，仅认 `0xEFBBBF0D0A` 头部，纯净处理 CRLF。

---

## 5. 测试原则：DAMP 优于 DRY
- **测试自解释 (DAMP)**：每个测试必须完整呈现 Setup -> Action -> Assert。测试代码容忍适度复制，避免过度抽取辅助函数导致排查断言失败时需要跨层跳转。
- **测试公开接缝 (Seam)**：测试公开接口行为，不探测私有实现细节。
- **表驱动与真实用例**：充分利用单元测试和固件（Fixtures）覆盖 UTF-8 BOM 边缘情况、特殊字符、大小写不敏感匹配及多行转义。

---

## 6. 代码质量门禁 (Linting & Verification)
- **定期执行严格检查**：
  ```bash
  cargo clippy --all-targets --all-features -- -D warnings
  ```
- **关键 Clippy 规则关注**：
  - `clippy::redundant_clone`（多余克隆）
  - `clippy::needless_collect`（多余的中间收集）
  - `clippy::large_enum_variant`（过大枚举体，必要时 Box）
- **禁止无脑 allow**：若有特定场景需要豁免，优先使用 `#[expect(clippy::lint_name, reason = "...")]` 并写明理由。
