# 资源组织与存储迁移方案

## 目标

资源的产品语义必须由稳定的逻辑 ID 表达，而不是由文件名、目录深度、压缩包偏移或导入工具的命名规则表达。文件系统是当前开发态后端；未来可以在不改变游戏、视图或资源规划代码的前提下切换到单文件资源包。

本方案定义以下不变式：

- 上层只使用 `AssetKey`，不拼接文件路径，不解析文件名。
- 一个 `AssetKey` 对应一个确定的资源语义和内容约束。
- catalog 是 key 到资源描述的唯一真相；文件路径和包内 offset 都是 catalog/后端细节。
- 开发态目录与发布态资源包由同一份 catalog 产生，并可逐项验证字节和解码结果相同。
- 导入来源中的历史命名、拼写错误、图集行列号等只能保存在迁移映射中，不能成为长期 API。

## 当前基线

当前资源根目录为工作区级的 `assets`。其中约 6,666 个文件，`pokemons` 约 6,196 个，约 4,876 个文件名包含来源约定，例如：

```text
pokemons/normal/front/001_Front_0_C__frame_0.png
characters/red/actions/group-01/group-01_row-00.png
characters/red/actions/group-00/down_runn_3.png
```

现在的 `game-asset-plan::AssetRequest` 同时包含 `AssetKey` 和 `relative_path`。这使纯资源规划知道了文件系统布局，也使未来的包文件、内存测试资源或网络资源后端难以替换。

## 长期边界

```text
game-view / map-render / game-asset-plan
    只产生 AssetKey 与资源约束
                 |
                 v
AssetCatalog
    AssetKey -> AssetDescriptor
                 |
       +---------+---------+
       |                   |
       v                   v
FilesystemAssetSource   PackedAssetSource
  source_path              data_offset + data_length
       |                   |
       +---------+---------+
                 v
            AssetBytes -> decode -> NativeAssets
```

`game-assets` 应拥有 `AssetKey`、`AssetDescriptor`、资源约束和 catalog 的纯解析/验证模型。`game-asset-plan` 只产生请求。`game-fs-assets` 是当前文件系统实现；未来的 pack 读取器是新的 adapter crate。`game-host` 与 `map-editor` 只接收一个资源来源，不再计算资源根目录或文件名。

建议的请求边界如下：

```rust
pub struct AssetRequest {
    pub key: AssetKey,
    pub requirements: AssetRequirements,
}

pub struct AssetDescriptor {
    pub key: AssetKey,
    pub kind: AssetKind,
    pub codec: AssetCodec,
    pub dimensions: Option<PixelSize>,
    pub content_hash: ContentHash,
}

pub trait AssetSource {
    fn read_many(&self, requests: &[AssetRequest]) -> Result<Vec<AssetBytes>, AssetReadError>;
}
```

`AssetRequest` 不得再含 `relative_path`、`PathBuf`、文件扩展名或包偏移。读取失败报错应包含 `AssetKey` 和后端诊断，而非要求调用方理解存储布局。

## 逻辑 ID 规范

ID 使用小写 ASCII，分段符为 `/`；段内允许 `[a-z0-9][a-z0-9._-]*`。数字使用固定宽度，避免排序和序列化不稳定。ID 不包含文件扩展名。

| 资源类别 | 逻辑 ID 示例 |
| --- | --- |
| 宝可梦精灵 | `pokemon/0001/form/00/normal/front/00` |
| 宝可梦背面精灵 | `pokemon/0001/form/00/normal/back/01` |
| 宝可梦图标 | `pokemon/0001/form/00/icon/00` |
| 角色动作 | `character/red/down/walk/00` |
| 属性图标 | `ui/battle/type/fire` |
| 招式分类图标 | `ui/battle/move-category/physical` |
| UI 程序生成图形 | `ui/shape/rounded-rect` |
| 地图原子 tile | `map/tile/0101` |
| 静态游戏数据 | `data/game/current-dataset/v2` |

`form`、`normal`、`front`、`frame` 等维度必须以显式段表达。不能把 `0_C`、`S`、`row-00` 等未定义来源标记直接带入 ID。无法解释来源维度的资源必须阻止迁移，直到其产品语义被确认；不能把不确定性带入 catalog 或运行时。

Key 由领域枚举或受限构造器生成。例如 `PokemonFormId`、方向、动作、帧号应构造精灵 key；业务和视图代码不应调用自由格式的 `format!` 组装 key。

## 开发态目录与 Catalog

完成迁移后，开发态目录建议如下：

```text
assets/
  catalog/
    assets.v1.json
    assets.v1.lock.json
  source/
    pokemon/0001/form/00/normal/front/00.png
    pokemon/0001/form/00/normal/back/01.png
    character/red/down/walk/00.png
    ui/battle/type/fire.png
    map/tile/0101.png
    data/game/current-dataset/v2.json
  imports/
    README.md
```

`source/` 是规范化后的开发资源树，建议其路径与 key 保持一对一映射，以便人工查找；这只是便利，不是 API。原始下载或未处理资源放在 `imports/`，不被运行时读取。

可编辑地图工程属于工作区数据，而不是只读资源；它位于工作区根的 `maps/`，不写入 `assets/catalog/`，以保持地图编辑器的读写边界独立。

`assets.v1.json` 的每个条目至少包含：

```json
{
  "schema_version": 1,
  "assets": [
    {
      "key": "pokemon/0001/form/00/normal/front/00",
      "kind": "image",
      "codec": "png",
      "source": "source/pokemon/0001/form/00/normal/front/00.png",
      "dimensions": [64, 64],
      "sha256": "..."
    }
  ]
}
```

`assets.v1.lock.json` 由构建工具生成，记录稳定排序后的 key、内容 hash、长度和可选尺寸。提交 lock 文件可确保资源变更在代码审查中可见。catalog 中禁止重复 key、空 key、未声明 codec 或缺失 source；同一 key 的内容变更必须更新 hash。

例如，`001_Front_0_C__frame_0.png` 的一次性迁移目标是 `source/pokemon/0001/form/00/normal/front/00.png`。现有的 `down_runn_3.png` 迁移为 `source/character/red/down/run/02.png`。迁移完成后旧路径和旧拼写不存在于资源树、catalog 或运行时诊断中。

## Pack 格式

发布态不应把文件系统路径写入包内 API。建议的初始格式是一个可顺序写入、可随机读取的 `.pak`：

```text
Header
  magic: "PUAS"
  format_version: u16
  index_offset: u64
  index_length: u64
  catalog_hash: [u8; 32]

Payload blocks
  独立资源的原始或压缩字节

Sorted index
  key bytes, kind, codec, data_offset, stored_length,
  original_length, content_hash, optional dimensions
```

索引按完整 key 的字节序稳定排序。实现可先二分查找；若以后需要哈希索引，条目仍必须保留完整 key 以检测碰撞。offset 和长度使用 `u64`。包读取器必须在分配或读取前验证：header、版本、index 边界、每条记录边界、资源区间不重叠、长度上限和内容 hash。

PNG 等已经压缩的图片默认保持原始 payload；是否改为块压缩由基准测试决定。每个资源可独立读取，避免读取一个精灵而解压整个包。

## 两步迁移

迁移必须以“Python 生成可审查的数据，Rust 消费稳定的契约，测试接通两者”为单位推进。禁止先手工批量改名，再让 Rust 代码追逐新的路径。

### 第一步：一次性规范化迁移并接入 Catalog

这一阶段完成原子切换：Python 将旧资源树迁移为规范 `source/` 树并生成 catalog；Rust 在同一变更中切换到只认识规范 key 和规范 source 路径的读取契约。不存在旧路径回退、双读或兼容适配层。

Python 工具放在 `scripts/assets/`，只使用标准库，至少提供：

1. `plan_asset_migration.py --check`：遍历当前资源树，按已审查的文件名规则输出稳定的规范化迁移计划；任何冲突、缺失维度或无法解释的来源记号都失败。
2. `migrate_assets.py --apply`：执行计划，移动资源到 `source/`，生成 `catalog/assets.v1.json` 与 `assets.v1.lock.json`，并确认旧路径全部消失。
3. `verify_catalog.py`：只验证规范树，检查 key 唯一、每个 source 存在、hash/长度/尺寸正确，以及 source 路径符合 key 规范。

Rust 在同一步完成契约切换：

1. `game-assets` 定义 key 语法、`AssetDescriptor`、`AssetCatalog` 与 `AssetSource`。
2. `game-asset-plan::AssetRequest` 删除 `relative_path`，只保留 key 和资源约束。
3. `game-fs-assets` 读取 catalog，以 key 查出规范 `source/` 路径；它不识别旧文件名或旧目录。
4. `game-host`、`map-editor` 通过注入的 `AssetSource` 加载资源，不再计算资源根目录或直接拼接路径。

该步的接通测试必须覆盖：

1. 每个由 `asset_requests`、地图和编辑器产生的 key 都能由新 catalog 解析。
2. catalog 给出的字节可解码，且满足请求声明的尺寸约束。
3. Python 在执行迁移前后生成内容清单，断言资源数量、按内容 hash 排序的集合和总字节数不变。
4. Rust 集成测试使用新文件树加载固定种子 roster 与固定地图，并断言 `NativeAssets` 资源 key、尺寸和像素 hash。
5. Python `verify_catalog.py` 和 Rust catalog 测试在 CI 中同时执行；任一方失败都阻止合并。

### 第二步：在规范文件树上加入 Pack 后端

第一步稳定后，Python 只以规范 catalog 与 `source/` 为输入生成 pack。此阶段不再读取、识别或保留旧命名。

Python 工具新增：

1. `build_asset_pack.py`：以规范 catalog 和 source 生成 `.pak`，并输出 package manifest hash。

Rust 在同一步新增 `PackedAssetSource`，实现与文件系统后端完全相同的 `AssetSource` 契约。启动配置选择后端，但业务、视图和资源规划 crate 不增加后端分支。

该步的接通测试必须覆盖：

1. 对 catalog 全集，规范文件树和 pack 后端返回相同的 key、内容 hash 与解码结果。
2. 损坏 pack 的 header、index、offset、长度和 hash 都得到结构化错误。
3. 文件系统后端保留给编辑器、热重载和开发调试；pack 后端用于发布读取。

## 验收

- catalog 中的 key 唯一、排序稳定，所有 source 文件 hash 与 lock 一致。
- 当前游戏和地图编辑器请求的每个 key 都能在文件系统 catalog 中解析。
- 重命名迁移前后，同一逻辑 key 的图片尺寸、像素 hash 和渲染 atlas 结果一致。
- 文件系统后端和 pack 后端对同一 key 集合返回相同内容 hash；解码结果一致。
- `game-asset-plan`、`game-view`、`map-render`、`game-host` 和 `map-editor` 中不再出现资源相对路径、旧导入文件名或资源根目录推导。
- pack 读取器对损坏 header、未知版本、越界 offset、重复 key、hash 不匹配和超大声明长度均返回结构化错误。

## 非目标

- 本迁移不改变 `AssetKey` 的业务含义，不重做图形格式或 atlas 算法。
- 不以资源包替代开发态文件树；编辑和调试仍优先使用 catalog 加文件系统后端。
- 不把资源读取副作用移入领域、应用或纯呈现 crate。

## 已知问题

- `pokemon/0351/form/00/normal/back/{00,01}` 当前不在源资源树或 catalog 中。默认队伍随机到该形态作为玩家侧时无法加载背面精灵；在补齐这两个资源前，完整图集尺寸测试保持忽略状态。
