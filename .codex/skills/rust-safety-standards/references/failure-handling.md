# 显式失败处理

## 规则

- 预期失败必须保留在类型系统中。用 `Result<T, E>` 表示失败原因，用 `Option<T>` 表示可预期的缺失。
- 调用方需要区分失败原因时，定义或扩展错误枚举；不要只返回无上下文的字符串。
- 函数的失败不能被本层恢复时，修改返回类型并使用 `?` 向上传播。
- 函数能够恢复时，在本层用 `match`、`let ... else`、`if let` 或组合子处理成功与失败分支。
- 访问集合时优先使用 `get`、`get_mut`、`first`、`last`、`split_at_checked` 或业务对象提供的安全查询方法。
- 将可能失败的数字转换改为 `try_from`，并把转换失败映射为调用方能处理的错误。
- 构造器已验证的不变量也要通过类型、私有字段、返回值或本地控制流保持可证明；不要借 `expect` 把证明留在注释里。

## 推荐模式

```rust
fn load_name(input: &str) -> Result<Name, NameError> {
    Name::parse(input)
}

fn active_item(items: &[Item], index: usize) -> Result<&Item, InventoryError> {
    items.get(index).ok_or(InventoryError::MissingItem { index })
}

fn label(record: Option<&Record>) -> Result<&str, LookupError> {
    let Some(record) = record else {
        return Err(LookupError::NotFound);
    };
    Ok(record.label())
}
```

测试也使用相同方式。让测试函数返回 `Result<(), TestError>`，以 `?` 传播设置和调用失败；断言业务结果，而不是解包它。

```rust
#[test]
fn parses_valid_name() -> Result<(), NameError> {
    let name = Name::parse("red")?;
    assert_eq!(name.as_str(), "red");
    Ok(())
}
```

## 禁止模式

```rust
let value = operation().unwrap();
let value = option.expect("must exist");
let item = items[index];
let count = usize::try_from(value).unwrap();
```

不要把 `is_ok`、`is_some`、长度检查或注释当作继续使用 `unwrap`、`expect` 或索引的理由。把检查和成功值绑定在同一个表达式或分支中。

## Panic 边界

`assert!`、`assert_eq!` 和 `assert_ne!` 只用于测试断言。生产路径不要以断言、调试断言或 panic 表示可由输入、文件、平台、状态或依赖失败引起的情况。

真正无法继续的进程边界由 runtime、CLI 或框架入口决定；它们也应先格式化和报告结构化错误，再以合适的退出方式结束。不要让 domain、application、presentation 或 adapter 用 panic 把普通失败越过边界。
