/// 攻击属性相对于一只宝可梦全部属性的倍率。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeEffectiveness {
    Immune,
    Quarter,
    Half,
    Normal,
    Double,
    Quadruple,
}
