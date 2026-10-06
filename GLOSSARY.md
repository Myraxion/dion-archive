# Dion Domain Glossary

Dion 是一个轻量高性能的 Windows 原生 CLI 工具，用于以符合 Total Commander 标准的 UTF-8 编码读写和维护 `descript.ion` 文件备注。

## Language

**Target Path**:
用户在命令行中指定的待查看或操作备注的目标文件或目录路径。
_Avoid_: File path, Resource path

**Parent Directory**:
目标文件或目录在文件系统中的直接上级目录，亦为存储对应 `descript.ion` 的宿主目录。
_Avoid_: Working directory, Host directory

**Entry Name**:
在 `descript.ion` 中作为键名使用的单层文件名或目录名。若包含空格则由双引号包裹。
_Avoid_: File key, Record key, Key

**Comment**:
关联到特定目标文件的备注文本信息，支持单行或多行形式。
_Avoid_: Description, Annotation, Remark, Note

**Raw Comment**:
在 `descript.ion` 物理存储中序列化后的单行字符串表示（包含 `\n` 转义标记、`\\` 反斜杠转义及多行结束符 `\x04\xC3\x82`）。
_Avoid_: Stored text, Serialized comment

**Display Comment**:
经解码还原为操作系统原生换行（CRLF）与非转义字符的、面向用户展示的备注文本。
_Avoid_: Human comment, Formatted comment

**Hidden Attribute**:
Windows 文件系统中的 `FILE_ATTRIBUTE_HIDDEN` 属性，`descript.ion` 文件在创建和修改时均必须保持该属性。
_Avoid_: Secret file, Dotfile

**Case-Insensitive Match**:
在 Windows 上对 Entry Name 的匹配原则，不区分字母大小写，确保同一文件在 `descript.ion` 中至多存在一条记录。
_Avoid_: Exact match, Byte match

**Atomic Replacement**:
写入 `descript.ion` 时采用同目录临时文件写入并原子重命名覆盖目标文件的机制，保证断电或异常时数据不损坏。
_Avoid_: In-place overwrite, Truncate write

**Literal Newline Principle**:
仅将输入中的实际换行符（LF 或 CRLF）视作多行备注的分隔边界，参数中的字符序列 `\n` 始终视为普通文本，禁止隐式转义。
_Avoid_: Escaped newline, Synthetic break

**Standard Input Source**:
通过管道或标准输入（`-` 或 `--stdin`）向 `set` 命令注入备注内容的通道，用于避免外层 Shell 引号转义问题并支持任意复杂文本。
_Avoid_: Pipe input, Stream input

**Order Preservation**:
修改已有备注条目时维持其在 `descript.ion` 文件中原始所在行序，新增条目统一追加于文件末尾的存储策略。
_Avoid_: Auto-sorting, Random reordering

**Idempotent Unset**:
执行删除操作时，若目标条目本身不存在亦视作成功并返回零状态码的约定。
_Avoid_: Strict unset, Failing delete

**Exit Code Contract**:
严格约定的四级退出码体系（0: 成功或幂等成功, 1: 目标物理实体或备注未找到, 2: 用法错误, 3: 执行或格式错误），提供完全确定的自动化控制流支持。
_Avoid_: Generic exit code, Binary exit code

**Universal JSON Contract**:
`get` 与 `list` 命令统一支持的 `--json` 结构化输出协议，供脚本和 Agent 精准解析。
_Avoid_: Ad-hoc JSON, Text parsing

**Lexical Path Resolution**:
对目标路径仅执行纯字面消除 `.`/`..` 的层级解析，不跟随符号链接与 Junction 展开到物理目标，确保备注始终记录在直接上级目录。
_Avoid_: Canonical path, Symlink traversal

**Target Existence Validation**:
在执行写入操作前，通过无穿透元数据检查（`symlink_metadata`）确认 Target Path 对应的条目在父目录文件系统中实际存在的门禁机制。
_Avoid_: File existence check, Physical probing

**Orphan Entry**:
记录在 `descript.ion` 中，但在对应父目录文件系统中已无物理实体（文件、目录、符号链接）与之对应的历史残留备注项。
_Avoid_: Ghost entry, Dead record, Dangling comment

**Fail-Safe Validation**:
检测到现有 `descript.ion` 损坏、非法编码或存在冲突时，拒绝写入并以退出码 3 终止，杜绝静默破坏用户数据的原则。
_Avoid_: Silent fix, Force write

