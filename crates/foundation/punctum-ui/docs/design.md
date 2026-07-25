# 设计说明

## 模块边界

`model` 负责与渲染器无关的几何、样式、内容和节点数据。
`tree` 负责自动 ID 分配和树校验。
`layout` 负责类 Flex 布局、裁剪、绘制命令生成和命中测试。
`error` 负责构建和解析返回的结构化失败。
`lib.rs` 是稳定的根导出面。

## 不变量

结构 ID 和非空动态 key 在一棵树内唯一。
比例尺寸、缩放文本和逻辑画布必须使用非零基数。
绘制命令和命中区域按树顺序产生，因此反向遍历会选中视觉上最顶层的命中区域。

## 演进约束

该 crate 不依赖渲染后端、资源、窗口系统或应用状态。
新的渲染器消费 `UiFrame`，不能向布局模块加入平台调用。

## 固定单列滚动窗口

`KeyboardSingleColumnFixedHeightScrollView` 是纯 UI 状态和树构建器。它只保存 item 数量、当前游标、第一可见项、固定 item 高度、间距和 overscan 数量。

窗口使用绝对定位槽位和 `clip`，因此不会把整列 item 放入布局计算；调用方只构建 `render_range` 返回的节点。键盘层负责调用 `move_up`、`move_down`、`move_to_top` 和 `move_to_bottom`，页面层负责把游标映射为业务 intent。
