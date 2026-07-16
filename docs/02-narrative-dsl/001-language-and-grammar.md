# 语言、词法与文法

> 状态：详细设计，未实现

## 结论

第一版脚本使用 ASCII 源文件、显式分号、花括号和固定 token 规则。解析器采用递归下降。文法只在完整 First/Follow 测试通过后才宣称为 LL(1)。

脚本不包含人类可读文本。所有显示内容使用 `text:` 资源 ID。

## 源文件与词法

源文件编码为 ASCII。注释以 `//` 开始并持续到行尾。空白包含空格、制表符和换行，语义上等价。

| token | 形式 | 说明 |
| --- | --- | --- |
| `IDENT` | `[A-Za-z_][A-Za-z0-9_.-]*` | 关键字、局部名、方法名 |
| `INT` | `0` 或 `[1-9][0-9]*` | 非负十进制整数 |
| `BOOL` | `true` 或 `false` | 布尔常量 |
| `RESOURCE` | `IDENT ":" IDENT` | 稳定业务资源 ID |
| `PATH` | `/IDENT(/IDENT)*` | Ramus capability 路径 |
| `VAR` | `${IDENT}` | 脚本局部变量引用 |

字符串字面量不在第一版语言中。需要文字时写 `text:town_guard_warning`；需要复杂数据时引用已注册的资源或调用 provider 查询。

保留字为：`actor`、`script`、`intercept`、`if`、`else`、`choose`、`option`、`say`、`wait`、`end`、`true`、`false`。

## 符号域

| 形式 | 域 | 示例 | 解析后校验 |
| --- | --- | --- | --- |
| `IDENT` | DSL 局部域 | `threat_level` | scope 和保留字 |
| `RESOURCE` | 内容域 | `event:road_clear` | 资源 catalog 与类型前缀 |
| `PATH` | Ramus 能力域 | `/world/actor` | capability catalog |
| `VAR` | 局部值域 | `${actor}` | 当前 frame 的局部变量 |

第一版允许的 `RESOURCE` 前缀为 `actor`、`anchor`、`dir`、`event`、`item`、`script`、`text`。新增前缀需要同步增加内容 catalog、schema 和 fixture。

## 表面语法

属性只标记声明元数据。属性不产生任意代码执行。

```text
#[actor]
actor gate_guard {
  id: actor:gate_guard;
}

#[script]
script patrol(actor: actor:gate_guard) {
  move(to: anchor:north_gate);
  wait(event: event:road_clear);
  => script:return_to_post;
}

#[intercept(priority: 80)]
intercept danger_response(actor: actor:gate_guard) {
  if threat_level > 2 {
    /world/actor face(target: ${actor}, direction: dir:south);
    => script:retreat;
  } else {
    say(text: text:guard_warning);
  }
}
```

`script` 与 `intercept` 的参数是绑定默认值。运行时实例可以用经过 schema 校验的值覆盖它们。`=>` 只能跳转到 `script:` 资源。

## EBNF

下列 EBNF 定义第一版目标文法。`EOF` 由 lexer 在输入末尾产生。

```ebnf
Module          = Declaration* EOF ;
Declaration     = Attribute* ( ActorDecl | ScriptDecl | InterceptDecl ) ;
Attribute       = "#" "[" IDENT AttributeArgs? "]" ;
AttributeArgs   = "(" NamedArgs? ")" ;
ActorDecl       = "actor" IDENT "{" "id" ":" RESOURCE ";" "}" ;
ScriptDecl      = "script" IDENT "(" Parameters? ")" Block ;
InterceptDecl   = "intercept" IDENT "(" Parameters? ")" Block ;
Parameters      = Parameter ( "," Parameter )* ;
Parameter       = IDENT ":" RESOURCE ;
Block           = "{" Statement* "}" ;
Statement       = Builtin ";"
                | CapabilityCall ";"
                | Jump ";"
                | IfStmt
                | ChooseStmt ;
Builtin         = "say" "(" NamedArgs ")"
                | "wait" "(" NamedArgs ")"
                | "end" "(" ")"
                | IDENT "(" NamedArgs? ")" ;
CapabilityCall  = PATH IDENT "(" NamedArgs? ")" ;
Jump            = "=>" RESOURCE ;
IfStmt          = "if" Comparison Block ElsePart? ;
ElsePart        = "else" Block ;
Comparison      = Operand ( "==" | "<" | ">" ) Literal ;
ChooseStmt      = "choose" "{" Option+ "}" ;
Option          = "option" RESOURCE ":" Block ;
NamedArgs       = NamedArg ( "," NamedArg )* ;
NamedArg        = IDENT ":" Value ;
Value           = RESOURCE | VAR | INT | BOOL ;
Operand         = IDENT | VAR ;
Literal         = RESOURCE | INT | BOOL ;
```

`PATH` 是单一 lexer token，因此 `CapabilityCall` 与普通 `IDENT` 调用的首 token 不重叠。`if`、`choose`、`=>` 和 `}` 也各有唯一首 token。实现时必须提供 First/Follow 表和自动化反例；如果任一可选分支冲突，删除该语法糖。

## 流式 parser 事件与 AST

生产编译器不要求保留完整模块 AST。parser 从纯 `ByteStream` 增量读取 ASCII chunk，维护有界 token 窗口和嵌套 block 栈，并按声明或语句边界发出事件。它不读取文件、不 seek、不写缓存，也不依赖全局可变状态。

```rust
pub trait ByteStream {
    fn next_chunk(&mut self) -> Option<Result<ByteChunk, SourceError>>;
}

pub enum ParseEvent {
    BeginDeclaration(DeclarationHeader),
    Statement(ParsedStatement),
    EndDeclaration,
    Diagnostic(ParseDiagnostic),
}
```

`ByteStream` 的文件、网络或内存实现都在 adapter/工具层。语言 core 只消费 chunk。编译器可以把 `ParseEvent` 直接交给名称绑定和 CPS builder；完整 AST 仅用于测试、格式化器或 GUI 编辑器，且由同一事件流可选构建。

每个事件和可选 AST 节点都携带 `SourceSpan`，用于稳定诊断。

```rust
pub struct Module {
    pub declarations: Vec<Declaration>,
}

pub enum Declaration {
    Actor(ActorDecl),
    Script(ScriptDecl),
    Intercept(InterceptDecl),
}

pub enum Statement {
    Builtin(BuiltinCall),
    Capability(CapabilityCall),
    Jump(ScriptRef),
    If { comparison: Comparison, then: Block, otherwise: Option<Block> },
    Choose { options: Vec<ChoiceOption> },
}
```

事件和 AST 不保存 `WorldState`、GPU 资源、文件路径或 provider 实现。资源引用在语法阶段只是带前缀的值，直到名称绑定阶段才变成类型化 ID。

## 静态校验

编译器按固定顺序执行：

1. ASCII、token 长度、文件大小和嵌套深度限制。
2. 将 chunk 解析为事件；测试工具可选择构建 AST。
3. 声明重复、属性组合和局部 scope 检查。
4. 资源 catalog、脚本跳转和 capability 路径解析。
5. 参数类型、`if` 比较类型、option ID 和调用 effect 检查。
6. 禁止递归跳转、无界循环和不可达块。
7. 流式生成确定性的 CPS `ScriptProgram` 与内容 hash。

跨脚本引用不通过“先读完整个包”解决。内容 bundle 先提供版本化 `ScriptInterfaceCatalog`，其中只含已导出的 `script:` ID、参数 schema 和 hash。每个脚本 body 可据此单遍编译。新增或删除导出时先重建接口 catalog，再并行编译各脚本 body。

诊断必须有稳定代码、source span、阶段和简短消息。例如 `E0201 unknown-resource`、`E0310 capability-denied`、`E0402 recursive-jump`。同一输入的诊断顺序不得依赖哈希表遍历。

## 语言测试

- 每个 token 规则都要有接受和拒绝 fixture。
- 非 ASCII 字节、未知前缀、空资源 ID、非法路径和错误插值必须在 lexer/parser 阶段失败。
- 每条产生式至少有一个正例和一个相邻歧义反例。
- AST snapshot 只在语言版本变更时更新。
- 编译后 `ScriptProgram` 的二进制或 JSON 产物必须具有稳定 hash。
