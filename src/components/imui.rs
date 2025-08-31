use dawn_assets::TypedAsset;
use dawn_graphics::gl::font::Font;
use glam::{Vec2, Vec4};

#[derive(Debug, Clone)]
pub struct Style {
    pub font: TypedAsset<Font>,
    pub scale: f32,
}

#[derive(Debug, Clone)]
pub enum UICommand {
    ApplyStyle(Style),
    ChangeColor(Vec4),
    StaticText(Vec2, &'static str),
    Text(Vec2, String),
    Box(Vec2, Vec2), // position, dimensions
}

// #[derive(Debug)]
// enum PositionedGuiElement {
//     StaticString(&'static str),
// }
//
// #[derive(Debug)]
// enum UnpositionedGuiElement {
//     ChangeColor(Vec4),
//     ResetColor,
// }
//
// #[derive(Debug)]
// enum CompiledGuiElement {
//     Unpositioned(UnpositionedGuiElement),
//     Positioned(Vec2, PositionedGuiElement),
// }
//
// pub struct GuiNil {}
//
// impl GuiNil {
//     pub fn new() -> Self {
//         GuiNil {}
//     }
// }
//
// pub struct GuiCons<H, T> {
//     head: H,
//     tail: T,
// }
//
// impl<H, T> GuiCons<H, T> {
//     pub fn new(head: H, tail: T) -> Self {
//         GuiCons { head, tail }
//     }
// }
//
// #[derive(Clone)]
// pub enum Alignment {
//     Start,
//     Center,
//     End,
// }
//
// pub trait GuiElement {
//     fn compile(&self, _result: &mut Vec<CompiledGuiElement>) -> anyhow::Result<Vec2> {
//         // Default implementation does nothing
//         Ok(Vec2::ZERO)
//     }
//
//     fn alignment(&self) -> Alignment {
//         Alignment::Start
//     }
//
//     fn dimensions(&self) -> Option<Vec2> {
//         None
//     }
// }
//
// impl GuiElement for GuiNil {}
//
// impl<H: GuiElement, T: GuiElement> GuiElement for GuiCons<H, T> {
//     fn compile(&self, result: &mut Vec<CompiledGuiElement>) -> anyhow::Result<Vec2> {
//         let bounding = self.head.compile(result)?;
//         let tail_bounding = self.tail.compile(result)?;
//         Ok(Vec2::max(bounding, tail_bounding))
//     }
// }
//
// macro_rules! construct_chain {
//     () => {
//         GuiNil::new()
//     };
//     ($head:expr $(, $tail:expr)*) => {
//         GuiCons::new($head, construct_chain!($($tail),*))
//     };
// }
