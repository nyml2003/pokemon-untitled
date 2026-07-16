# 地图项目压缩存储方案

## 结论

为 `MapProject` 新增 `crates/adapter/map-project-storage` crate，提供版本化的 `.g3mp` 二进制容器、纯内存的 `MapProjectReader` 与 `MapProjectWriter`，以及由 `map-editor-cli` 暴露的 `inspect`、`pack`、`unpack` 和 `verify` 命令。

`map-project` 继续只拥有地图模型、编辑规则和语义校验，不知道 JSON、压缩算法、字节序或文件路径。`map-project-storage` 是外部格式 adapter：它把已验证的 `MapProject` 编成字节，或把字节解析回 `MapProject`。文件系统、CLI 参数和 stdout 仍留在 runtime crate。

格式的首要目标不是把 JSON 原文套一层 zstd，而是消除地图模型的重复表示：材料 ID 转为索引、视觉格子转为调色板索引、碰撞格子转为位图、事件格子转为稀疏表。结构化 payload 再使用 zstd 压缩，并带有长度限制和 BLAKE3 校验。

## 现状与问题

`maps/demo-map.json` 当前为 345,031 bytes、72 x 56（4,032 个格子），包含 69 个 materials、44 种实际视觉材料、3,752 个阻挡格子、40 个事件格子和 1 个 actor。运行时的 `game-host`、`map-editor` 和 `map-editor-cli` 都通过文本 JSON 加载它。

视觉层不适合固定使用 RLE：它的逐行 run 数为 3,749，接近 4,032 个格子。因此 v1 不能假设“地图天然是大片同色区域”。碰撞层只有 102 个 run，事件层只有 40 个非空格，分别适合位图/自适应编码和稀疏编码。

## 范围

本方案覆盖静态 `MapProject` 的磁盘格式、完整读写、元数据检查、校验和 JSON 迁移。

本方案不覆盖运行中世界状态、地图增量 patch、跨地图索引、加密、网络传输和热更新。它也不改变 `MapProject` 的字段或编辑命令语义。

## Crate 与所有权

```text
map-project                  地图模型、MapError、validate
        ^
        |
map-project-storage          .g3mp 编码、解码、元数据、结构化错误
        ^
        |
map-editor-cli               CLI 参数、文件读写、JSON 输出、退出码
game-host / map-editor       组合资源和选择项目路径
```

`map-project-storage` 放在 `crates/adapter/`。它不访问 `std::fs`，但允许使用 `std::io::Read` 和 `std::io::Write` 作为字节流接口。该 crate 的测试全部使用内存 buffer；具体路径和原子写入策略由 CLI 或 runtime 负责。

依赖为 `map-project`、`blake3` 和 `zstd`。不使用 `bincode` 或直接对 `MapProject` 做 `serde` 二进制序列化，因为这些方式会把 Rust 字段布局和版本演进隐式变成文件协议。

## 对外 API

```rust
pub const FILE_EXTENSION: &str = "g3mp";

pub struct MapProjectReader;
pub struct MapProjectWriter {
    options: WriteOptions,
}

impl MapProjectReader {
    // 仅解析固定头与 manifest；不解压 payload，不需要资源 catalog。
    pub fn inspect(input: &[u8]) -> Result<MapProjectMetadata, MapStorageError>;

    // 解压、校验、解码，并调用 MapProject::validate(known_tiles)。
    pub fn read(
        input: &[u8],
        known_tiles: &BTreeSet<AtomicTileId>,
    ) -> Result<MapProject, MapStorageError>;

    pub fn read_from<R: Read>(
        input: R,
        known_tiles: &BTreeSet<AtomicTileId>,
    ) -> Result<MapProject, MapStorageError>;
}

impl MapProjectWriter {
    pub fn new(options: WriteOptions) -> Self;

    // 先调用 project.validate(known_tiles)，再生成确定性的字节流。
    pub fn write(
        &self,
        project: &MapProject,
        known_tiles: &BTreeSet<AtomicTileId>,
    ) -> Result<Vec<u8>, MapStorageError>;

    pub fn write_to<W: Write>(
        &self,
        output: W,
        project: &MapProject,
        known_tiles: &BTreeSet<AtomicTileId>,
    ) -> Result<(), MapStorageError>;
}
```

`MapProjectMetadata` 包含容器版本、地图文档版本、地图 ID、tile size、宽高、cell/material/actor/event 数量、压缩算法、压缩前后长度和声明的 payload checksum。`inspect` 的 checksum 状态必须标记为 `not_verified`；只有 `read` 或 `verify` 解压全部 payload 后才能报告 `verified`。

`MapStorageError` 至少区分以下正常失败，调用方不需要从字符串猜原因：

| 错误 | 含义 |
| --- | --- |
| `Truncated` | 文件在头、manifest 或 payload 中途结束 |
| `BadMagic` / `UnsupportedContainerVersion` | 不是本格式，或版本不受支持 |
| `InvalidHeader` / `InvalidManifest` | 保留字、长度、计数或元数据不合法 |
| `LimitExceeded` | 声明长度、尺寸或分配超过安全上限 |
| `DecompressionFailed` / `ChecksumMismatch` | 压缩内容损坏或被改写 |
| `InvalidPayload` | section、索引、run、枚举或长度不合法 |
| `Map(MapError)` | payload 可解码但不满足地图领域不变量 |

## 文件容器

扩展名为 `.g3mp`，首个版本使用小端字节序。文件由 64-byte 固定头、未压缩 manifest 和压缩 payload 构成：

```text
+---------------------------+--------------------------------------+
| 区域                      | 内容                                 |
+---------------------------+--------------------------------------+
| header (64 bytes)         | magic、版本、长度、压缩方式、checksum |
| manifest (variable)       | 可直接读取的地图元数据                |
| compressed payload        | zstd(structured map payload)         |
+---------------------------+--------------------------------------+
```

固定头字段如下：

| offset | 字段 | 类型 | 说明 |
| --- | --- | --- | --- |
| 0 | magic | `[u8; 4]` | `G3MP` |
| 4 | container version | `u16` | v1 为 `1` |
| 6 | header length | `u16` | v1 必为 `64` |
| 8 | compression | `u8` | v1 为 `1`，即 zstd |
| 9 | reserved | `[u8; 3]` | 必为 0 |
| 12 | manifest length | `u32` | 未压缩 manifest 长度 |
| 16 | payload length | `u64` | 压缩 payload 长度 |
| 24 | raw payload length | `u64` | 解压后的最大允许长度 |
| 32 | raw payload BLAKE3 | `[u8; 32]` | 解压后 payload 的完整性校验 |

manifest 是有自身 revision 的紧凑二进制结构，不使用 JSON。它按顺序包含 `manifest_revision`、`MapProject::format_version`、map ID、tile size、地图尺寸、cell/material/atomic-tile/actor/event 数量和 payload schema revision。字符串使用 `u16 length + UTF-8 bytes`，所有 count 有明确上限。它只暴露检查地图所必需的元数据，不包含格子内容。

## Payload 编码

解压后的 payload 采用带长度的 section 流。每个 section 为 `section_id: u8 + byte_length: u32 + bytes`。v1 的必需 section 为 string table、materials、visual layer、collision layer、event layer 和 entities。未知的可选 section 跳过；未知的必需 section 或缺失/重复 v1 必需 section 都报错。这样后续可以新增层而不改变 v1 reader 的边界判断。

### 1. String table 和 materials

writer 收集所有 ID 和 appearance 字符串，按字典序去重写入 string table。materials 保持 `MapProject.materials` 的原始顺序，以确保读回的项目与输入完全相同；每个 material 写入 material ID 的字符串索引、layer 数量和 atomic tile 字符串索引。任何越界索引、空 layer 或重复 material 都是 `InvalidPayload`，最终仍须通过领域 `validate`。

### 2. Visual layer

视觉格子写成 `0 = None, material_index + 1 = Some(material)` 的无符号索引。index 宽度根据 material 数量自动选择 `u8`、`u16` 或 `u32`。

每行独立选择以下更小表示，选择相同大小时固定选择 raw，保证输出确定性：

1. `raw`：依次写入该行的固定宽度索引。
2. `rle`：写入 `(run length, material index)`，run 不能为 0，且行内长度和必须恰为地图宽度。

`demo-map` 的 visual layer 会选 raw，而不是 RLE。索引化已把 4,032 个 JSON object/string 引用降为约 4 KB 的格子数据；外层 zstd 还能处理重复纹理。

### 3. Collision layer

collision 只有 `walkable` 与 `blocked` 两个状态。writer 计算 raw bitset 与逐行 RLE 两种表示的长度，选择更小者；bitset 按行从低 bit 到高 bit 写入，1 表示 blocked。解码后必须恰好得到 `width * height` 个值。

### 4. Event layer

v1 事件写成稀疏表：`entry_count` 后接按 cell index 严格递增的 `(delta_index, event_kind)`。`event_kind` 使用显式枚举值，v1 的 `Encounter = 1`。这避免为 4,032 个 `null` 写入任何数据，并保留以后增加事件类型的空间。

### 5. Entities

entities section 写 player spawn 的 x/y，再写 actor 数量及每个 actor 的 ID string index、坐标、facing enum 和 appearance string index。坐标先在 decoder 做尺寸范围检查，再由 `MapProject::validate` 检查与碰撞、spawn 和其他 actor 的关系。

## 解析流程

`inspect` 只走步骤 1 到 4；完整 reader 走全部步骤：

1. 确认输入至少 64 bytes，读取固定头，不允许未知 compression、非零 reserved bytes 或版本不匹配。
2. 对 manifest length、payload length、raw payload length 和总文件长度做 checked arithmetic，确认每个 slice 正好落在输入内且没有尾随字节。
3. 在分配前执行限制：v1 默认最大宽/高各 4,096、最大 cell 数 4,194,304、最大 manifest 64 KiB、最大压缩 payload 64 MiB、最大解压 payload 256 MiB。限制是 API 常量，可在未来通过显式 `ReadLimits` 配置收紧。
4. 解析 manifest，检查 `cell_count == width * height`、所有 UTF-8 字符串合法且 count 不超过限制，返回 `MapProjectMetadata` 或继续。
5. 使用声明的 raw length 作为 zstd 输出上限解压；解压结果长度必须精确匹配头字段，随后验证 BLAKE3。
6. 用有界 cursor 解析每个 payload section。每次读整数、字符串索引、run 和 section length 都先检查剩余字节和数值范围；不允许隐式截断或裸 `as` 转换。
7. 先建立 typed ID，再构造 `MapProject`。解析期检查格子索引、枚举、section 完整性和坐标；最后调用 `project.validate(known_tiles)`，把未知 atomic tile、spawn blocked、actor overlap 等语义问题交给领域模型。
8. 成功时不保留压缩 buffer 的借用，返回独立的 `MapProject`。失败时返回结构化 `MapStorageError`，不 panic。

## Writer 流程与确定性

1. 调用 `project.validate(known_tiles)`；无效项目绝不写出部分结果。
2. 构建排序的 string table，但保留 material、actor 和 cell 的领域顺序。
3. 编码各 section；对 visual/collision 层以实际 byte length选择 raw 或 RLE，平局规则固定。
4. 计算 raw payload BLAKE3，以固定 zstd level `3` 压缩 payload。
5. 从项目和 payload 统计构造 manifest，最后一次性写 header、manifest 和 payload。

同一 `MapProject`、同一 writer options 和同一 zstd 版本应产生字节相同的输出。测试中将确定性作为合同，但不把跨 zstd 大版本的 bit-for-bit 相同性视为文件兼容性前提；reader 的兼容性只由容器和 payload schema 版本保证。

## CLI 设计

扩展现有 `map-editor-cli`，保留当前 JSON Lines 编辑模式。新子命令在进入交互循环前解析，因此不会破坏现有默认调用。

```text
map-editor-cli inspect maps/demo-map.g3mp --json
map-editor-cli verify maps/demo-map.g3mp --assets assets
map-editor-cli pack maps/demo-map.json maps/demo-map.g3mp --assets assets
map-editor-cli unpack maps/demo-map.g3mp maps/demo-map.json --assets assets
```

`inspect` 只调用 `MapProjectReader::inspect`，不读 tile catalog、不解压 payload，适合构建脚本和快速诊断。其 JSON 输出应稳定，至少包括：

```json
{
  "path": "maps/demo-map.g3mp",
  "container_version": 1,
  "document_format": "gen3-map-v2",
  "map_id": "demo-map",
  "tile_size": { "width": 16, "height": 16 },
  "dimensions": { "width": 72, "height": 56, "cells": 4032 },
  "materials": 69,
  "actors": 1,
  "events": 40,
  "compression": "zstd",
  "payload_bytes": 0,
  "raw_payload_bytes": 0,
  "integrity": "not_verified"
}
```

上例中的长度仅为字段形状，真实值由文件头提供。`verify` 和 `unpack` 必须加载 assets catalog，调用完整 reader，并将 checksum 与地图领域校验均验证完毕。`pack` 先使用现有 JSON 路径构造并验证 `MapProject`，再调用 writer；`unpack` 输出当前 `to_json_pretty` 的稳定文本，用于代码审阅和紧急人工修复。

## 迁移

第一阶段保留 `demo-map.json` 为兼容输入和回归 fixture，新增 `.g3mp` 的读写与 CLI，不改变默认运行时路径。

第二阶段由 `pack` 生成 `maps/demo-map.g3mp`，`game-host` 和 `map-editor` 按“显式传入路径优先，其次 `.g3mp`，最后 JSON 兼容回退”加载。地图编辑器保存到 `.g3mp` 时使用 writer；保存到 `.json` 时保留现有 JSON writer。这样迁移可以逐个地图进行，而不是一次性切断编辑能力。

第三阶段在团队确认二进制地图作为主资产后，把 JSON 从常规运行时路径移除，但保留 `unpack` 作为审阅、修复和版本迁移工具。二进制差异不如 JSON 直观，因此 PR 中涉及地图变化应附带 `map-editor-cli unpack` 的文本 diff 或结构化 `inspect`/`verify` 输出。

## 测试与验收

`map-project-storage` 需要覆盖：

1. `demo-map.json -> MapProject -> .g3mp -> MapProject` 的完全相等 round trip。
2. 同一项目两次写入字节完全相同。
3. visual raw、visual RLE、collision bitset、collision RLE、无事件和稀疏事件的边界用例。
4. 每类错误：截断、伪 magic、版本不兼容、长度溢出、校验失败、zstd 损坏、越界索引、错误 run、未知 section 和未知 atomic tile。
5. `inspect` 对合法文件不解压即可返回 manifest；`verify` 对同一文件完成完整性和领域校验。
6. CLI `inspect --json` 的字段稳定性，以及 `pack/unpack` 的 JSON 语义等价。

实现完成后记录 `demo-map` 的实际压缩比。验收下限是 `.g3mp` 小于原 JSON 的 10%，并且完整 read/validate 不得比当前 JSON 加载引入可感知的启动延迟。若真实数据未达标，先检查 string table、section 表示和 zstd 输入，再考虑更复杂的二维块压缩；不要先把 RLE 强加给视觉层。

## 已否定的方案

| 方案 | 不采用原因 |
| --- | --- |
| `gzip(JSON)` | 实现很快，但保留 4,032 个对象、重复键和字符串解析成本，CLI 元数据仍须解压 JSON。 |
| `bincode(MapProject)` | Rust 结构演进会隐式改变磁盘协议，不能稳定跳过新增字段或独立检查元数据。 |
| 固定视觉 RLE | `demo-map` 的视觉层 run 数过高，固定采用会比 raw palette index 更大。 |
| 让 `map-project` 直接读写 `.g3mp` | 会把存储协议和 zstd 依赖反向带入领域 crate。 |
| 仅保存 checksum，不保留 manifest | CLI 无法快速读取地图 ID、尺寸和版本，损坏诊断也更差。 |
