# 翠径区域

这是一组可直接由地图编辑器打开的十张 `72x56` 野外地图。它们共享同一套草地、林冠、草丛和砂土小径材料；地图之间的公共边缘会保持通行地形和道路对齐，玩家跨图后道路和草地会继续，而不是由连续树墙遮断。

```text
  (0,0) northern-thicket -- (1,0) moss-pass -- (2,0) stoneleaf-rise
        |                         |                     |
  (0,1) western-meadow ---- (1,1) wayfarer-crossroads - (2,1) sunlit-clearing - (3,1) old-east-road
        |                         |                     |
  (0,2) southern-field ---- (1,2) fern-hollow -------- (2,2) quiet-grove
```

`wayfarer-crossroads` 是区域中心，也是当前游戏的出生地图。`game-host` 会读取本目录的 `world.json`，用 `WorldProject` 校验十张地图，并投影成一张 `288x168` 的可玩地图。玩家可直接跨过相邻地图的边缘。

没有邻接地图的一侧使用三格厚灌木封边。灌木外是尚未注册地图的 `EmptyChunk`，不可进入；这包括区域外轮廓和布局中的缺口。外缘不用多格树对象，避免在 72x56 分区边缘切断树图样。

## 制作原则

- 相邻图的道路入口为四格宽，入口位置会随地形错开；公共边缘的草地和道路材质、碰撞逐格相同。凹角处的灌木封边会同步到相邻分区的边缘格。
- 林冠做成内部成簇的景深和可绕行障碍，不沿地图边缘铺成墙。
- 只有外部边界使用树墙，明确区分可继续探索的相邻地图和当前世界的尽头。
- 草丛保持可走并设置 `encounter`；道路、空地和林缘提供明确的节奏变化。
- 每张图都保留独立出生点，方便从地图编辑器单独打开测试。

## 再生成

```text
python scripts/maps/generate_verdant_route.py
```

生成器使用已提交的 `maps/demo-map.json` 材料定义，输出只写入本目录，并在写入前检查所有公共边缘的逐格一致性。

地图辅助脚本统一使用 Python；具体约定见 [`scripts/maps/README.md`](../../scripts/maps/README.md)。
