# domain 层类型建模规范（临时草案）

> 状态：临时草案 · 待讨论。
> 本草案先用于逐 crate 迁移验证，迁移中发现的问题回改到本草案，讨论定稿后再移除临时标记。

## 适用范围

只适用于**无状态 domain crate**：

- 不访问文件、网络、窗口、真实时间、环境变量。
- 没有全局可变状态（`static mut`、单例）。
- 状态由调用方持有，通过参数传入、返回值传出。
- 规则与状态转换确定、可测试。

有状态或含平台 IO 的 crate（application、presentation、adapter、runtime）不适用。

## 目录布局

domain crate 内部按类型形态组织模块，每个类型归入三类之一：

```
src/
├── lib.rs        # crate 文档 + 私有 mod 声明 + 域级常量 + 逐项 pub use
├── value/        # （仅当存在跨聚合共享的独立值时）值对象
│   └── id.rs
└── aggregate/    # 聚合根及与其绑定的 key、组成部分
    ├── coord.rs
    ├── placed.rs
    └── world.rs
```

lib.rs 只保留 crate 文档、`mod` 声明、域级常量（如标准尺寸）和逐项 `pub use`，不承载实现。

value/ 目录只在存在**跨聚合共享的独立值**时创建；聚合根私有的 key、构造输入、查询结果就近放 aggregate/。data/ 目录（数据/计算型）暂未实例化，出现真实 SoA/AoS 批量容器后再建。

## 三类形态

### 值对象（value/）

**跨聚合共享、独立可复用的领域值**。不可变、按值比较、类型安全。构造不校验（校验在边界），字段 pub，取值用解构，不写 getter。

审查清单：

- 不可变，无 `&mut` 改字段的方法。
- 字段 pub，无 getter。
- 领域逻辑用行为方法，不在外部解构做运算。
- 解构只出现在 adapter 的序列化边界。
- 只被单个聚合根使用的小 key/组成部分不放这里，就近放 aggregate/。

实例：跨聚合引用的 ID、可复用的数值。

### 数据/计算型（data/）

数据量大、计算密集、可并行、需要缓存优化。内部布局用 SoA/AoS 连续数组。

审查清单：

- 字段私有或 `pub(crate)`，外部不摸内部数组。
- 对外提供 batch 函数与语义查询。
- batch 函数只读，返回计算结果，不改自身。
- 外部不依赖内部布局（SoA/AoS 可切换）。

实例：批量投影数据、布局数据。

### 聚合根（aggregate/）

业务逻辑复杂、具体实例少、有行为不变量。状态只能通过行为方法到达。

审查清单：

- 字段私有。
- 状态只能经行为方法或构造到达。
- 行为不变量在构造或方法处保证。
- 无绕过方法直接改状态的路径。
- 不保留全局状态。

实例：`Battle`、`World`、`WorldProject`。

## 错误归属

错误类型跟随拥有运算的聚合根，不单列 `error.rs`。

- 值对象无错误类型（构造不校验，非法值在边界被拒）。
- 聚合根的错误放所属聚合根文件（`aggregate/` 下）。
- 数据/计算型的批量函数若失败，错误归调用它的聚合根，或自身携带。

## 横切原则

1. **校验只放在外部数据边界**。非法数据只在边界（解析、反序列化、导入、用户指令）校验，域内构造不重复校验。
2. **序列化下沉 adapter**。domain 不绑定 serde；adapter 在边界序列化值对象，取值用解构。
3. **不写伪 getter，不写 setter**。需要读值用解构（值对象）或语义查询（聚合根、数据容器）；需要修改用行为方法。
4. **不引入 ECS 基础设施**。数据裸加规则独立用纯函数加单向状态流实现。

## 引用既有规范

注释、文档、错误处理与 lint 规则分别遵循 `rust-commenting`、`rust-crate-documentation`、`rust-safety-standards`、`rust-crate-structure`。

## 理论依据

- 校验放边界与类型保证：Alexis King, *Parse, don't validate*（2019）。
- 前置条件与类不变量：Bertrand Meyer, *Design by Contract*。
- 非法状态不可表达：Richard Feldman, *Making illegal states unrepresentable*。
- 数据导向布局：Mike Acton, *Data-Oriented Design*；ECS（Storage + Query）。
- 聚合根边界：Eric Evans, *Domain-Driven Design*。

## 迁移记录

逐 crate 迁移，一个验证通过再下一个。

**world-project（试点，已迁移）**：

- 目录：全部归 `aggregate/`（`coord.rs` 的 `WorldChunkCoord`、`placed.rs` 的 `PlacedMap`/`PreloadSlot`、`world.rs` 的 `WorldProject`/`WorldProjectError`）。无跨聚合共享值，value/ 目录未建。
- `WorldChunkCoord` 字段 `pub(crate)`（对外 crate 隐藏，域内聚合根可读以做布局计算），`offset` 为 `pub(crate)` 行为方法。
- `PreloadSlot` 字段 pub、保留 `is_empty` 语义查询。
- 合成大地图是 runtime 职责：`WorldProject` 提供 `size()`、`origin_of()` 两个语义查询，边界与尺寸计算收进聚合根，不引入中间布局结构体。
- 常量 `STANDARD_MAP_WIDTH/HEIGHT` 作为域级常量放 lib.rs。
- data/ 无实例，目录未建。

**待迁移**：`map-project`、`world-domain` 及其余 domain crate。
