//! Windows-native Pokemon catalog editor.

#![forbid(unsafe_code)]

use std::{error::Error, path::PathBuf, sync::Arc};

use editor_application::{EditorCall, EditorDocumentId, EditorKind, EditorOperation};
use editor_ramus_adapter::{EditorRamusRouter, RoutedEditorIntent};
use editor_resource_adapter::EditorResourceRegistry;
use game_assets::{AssetKey, DecodedImage};
use game_native_target::{FramePlan, NativeAssets, NativeTarget, PresentOutcome, TextScale};
use pokemon_editor_core::{
    PokemonCatalog, PokemonEditCommand, PokemonEditorCommand, PokemonEditorModel, PokemonId,
};
use punctum_gpu::{PixelSize, Rgba8};
use punctum_ui::{
    CrossAlign, Dimension, FlexDirection, Insets, MainAlign, UiColor, UiContent, UiFrame,
    UiNode as RawUiNode, UiSize, UiStyle, UiTree,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, Ime, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

const DOC: &str = "kanto-hoenn-pokedex";
const NAME: u32 = 1;
const TYPE: u32 = 2;
const DOWN: u32 = 3;
const UP: u32 = 4;
const SAVE: u32 = 5;
const BG: UiColor = UiColor::new(18, 21, 25, 255);
const BTN: UiColor = UiColor::new(53, 67, 78, 255);
const SEL: UiColor = UiColor::new(42, 172, 148, 255);
const TXT: UiColor = UiColor::new(238, 242, 245, 255);
const MUTED: UiColor = UiColor::new(161, 177, 187, 255);
#[derive(Clone)]
enum Action {
    Id(u32),
}
type Node = RawUiNode<Action>;
struct App {
    registry: EditorResourceRegistry,
    router: EditorRamusRouter,
    document: EditorDocumentId,
    model: PokemonEditorModel,
    name: String,
    status: String,
    frame: Option<UiFrame<Action>>,
    cursor: Option<PhysicalPosition<f64>>,
    window: Option<Arc<Window>>,
    runtime: Option<NativeTarget<'static>>,
    assets: NativeAssets,
}
impl App {
    fn new() -> Result<Self, Box<dyn Error>> {
        let registry = EditorResourceRegistry::standard(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../"),
        )?;
        let document = EditorDocumentId::new(DOC)?;
        let model = load(&registry, &document)?;
        let name = model
            .catalog()
            .pokemon()
            .first()
            .map(|p| p.name().to_owned())
            .ok_or("pokemon catalog is empty")?;
        Ok(Self {
            registry,
            router: EditorRamusRouter::new()?,
            document,
            model,
            name,
            status: "已加载 Zigzagoon".into(),
            frame: None,
            cursor: None,
            window: None,
            runtime: None,
            assets: NativeAssets::new(vec![(
                AssetKey::new("solid/white")?,
                DecodedImage::solid(Rgba8::new(255, 255, 255, 255)),
            )])?,
        })
    }
    fn init(&mut self, l: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let w = Arc::new(
            l.create_window(
                Window::default_attributes()
                    .with_title("宝可梦编辑器")
                    .with_inner_size(LogicalSize::new(900., 620.)),
            )?,
        );
        self.runtime = Some(NativeTarget::new(
            w.clone(),
            px(w.inner_size()),
            &self.assets,
            Rgba8::new(18, 21, 25, 255),
        )?);
        w.set_ime_allowed(true);
        self.window = Some(w);
        self.draw_request();
        Ok(())
    }
    fn draw_request(&self) {
        if let Some(w) = &self.window {
            w.request_redraw()
        }
    }
    fn call(&mut self, op: EditorOperation, payload: serde_json::Value) -> Result<(), String> {
        let call = EditorCall::new(EditorKind::Pokemon, self.document.clone(), op, payload)
            .map_err(|e| e.to_string())?;
        let RoutedEditorIntent::Call(call) =
            self.router.route_call(call).map_err(|e| e.to_string())?
        else {
            return Err("unexpected resource open".into());
        };
        let command = match call.operation() {
            EditorOperation::Inspect => PokemonEditorCommand::Inspect,
            EditorOperation::Validate => PokemonEditorCommand::Validate,
            EditorOperation::Command => PokemonEditorCommand::Edit(
                serde_json::from_value(call.payload().clone()).map_err(|e| e.to_string())?,
            ),
            EditorOperation::Save => PokemonEditorCommand::Save,
        };
        let (next, _) = self.model.execute(command).map_err(|e| e.to_string())?;
        self.model = next;
        if matches!(op, EditorOperation::Save) {
            let json = self
                .model
                .catalog()
                .to_json_pretty()
                .map_err(|e| e.to_string())?;
            self.registry
                .save_text(EditorKind::Pokemon, &self.document, &json)
                .map_err(|e| e.to_string())?;
            self.model = self.model.clone().saved()
        }
        Ok(())
    }
    fn edit(&mut self, command: PokemonEditCommand) {
        match serde_json::to_value(command)
            .map_err(|e| e.to_string())
            .and_then(|v| self.call(EditorOperation::Command, v))
        {
            Ok(()) => self.status = "已修改宝可梦".into(),
            Err(e) => self.status = format!("错误：{e}"),
        }
        self.draw_request()
    }
    fn selected(&self) -> Option<PokemonId> {
        self.model
            .catalog()
            .pokemon()
            .first()
            .map(|p| p.id().clone())
    }
    fn click(&mut self) {
        let Some(c) = self.cursor else { return };
        let Some(Action::Id(id)) = self
            .frame
            .as_ref()
            .and_then(|f| f.action_hit_at(c.x.max(0.) as u32, c.y.max(0.) as u32))
            .map(|h| h.action.clone())
        else {
            return;
        };
        let Some(pokemon) = self.selected() else {
            return;
        };
        let current = self.model.catalog().pokemon().first();
        match id {
            TYPE => {
                let types =
                    if current.map(|p| p.types()) == Some([String::from("Normal")].as_slice()) {
                        vec!["Dark".into()]
                    } else {
                        vec!["Normal".into()]
                    };
                self.edit(PokemonEditCommand::SetTypes { pokemon, types })
            }
            DOWN | UP => {
                let hp = current.map(|p| p.base_hp()).unwrap_or(1);
                let hp = if id == DOWN {
                    hp.saturating_sub(1).max(1)
                } else {
                    hp.saturating_add(1).min(255)
                };
                self.edit(PokemonEditCommand::SetBaseHp {
                    pokemon,
                    base_hp: hp,
                })
            }
            SAVE => match self.call(EditorOperation::Save, serde_json::Value::Null) {
                Ok(()) => self.status = "已保存".into(),
                Err(e) => self.status = format!("错误：{e}"),
            },
            _ => {}
        }
        self.draw_request()
    }
    fn input(&mut self, text: String) {
        self.name.push_str(&text);
        if let Some(pokemon) = self.selected() {
            self.edit(PokemonEditCommand::SetName {
                pokemon,
                name: self.name.clone(),
            })
        }
    }
    fn back(&mut self) {
        self.name.pop();
        if let Some(pokemon) = self.selected() {
            self.edit(PokemonEditCommand::SetName {
                pokemon,
                name: self.name.clone(),
            })
        }
    }
    fn redraw(&mut self, l: &ActiveEventLoop) {
        let Some(size) = self.runtime.as_ref().map(NativeTarget::surface_size) else {
            return;
        };
        let Some(p) = self.model.catalog().pokemon().first() else {
            return;
        };
        let tree = ui(&self.name, p.types(), p.base_hp(), &self.status);
        let Ok(tree) = tree else { return };
        let Ok(frame) = tree.resolve(UiSize::new(size.width, size.height)) else {
            return;
        };
        let Ok(plan) = FramePlan::from_ui_frame(&frame, &self.assets, TextScale::new(1, 1, 16, 26))
        else {
            return;
        };
        self.frame = Some(frame);
        let (Some(w), Some(r)) = (&self.window, &mut self.runtime) else {
            return;
        };
        match r.present(&plan) {
            Ok(PresentOutcome::Reconfigured | PresentOutcome::SurfaceLost) => {
                r.resize(r.surface_size());
                w.request_redraw()
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("pokemon editor presentation failed: {e}");
                l.exit()
            }
        }
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, l: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(e) = self.init(l)
        {
            eprintln!("pokemon editor init failed: {e}");
            l.exit()
        }
    }
    fn window_event(&mut self, l: &ActiveEventLoop, id: WindowId, e: WindowEvent) {
        if self.window.as_ref().map(|w| w.id()) != Some(id) {
            return;
        }
        match e {
            WindowEvent::CloseRequested => l.exit(),
            WindowEvent::RedrawRequested => self.redraw(l),
            WindowEvent::Resized(s) => {
                if let Some(r) = &mut self.runtime {
                    r.resize(px(s))
                }
                self.draw_request()
            }
            WindowEvent::CursorMoved { position, .. } => self.cursor = Some(position),
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.click(),
            WindowEvent::Ime(Ime::Commit(t)) => self.input(t),
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && matches!(event.logical_key, Key::Named(NamedKey::Backspace)) =>
            {
                self.back()
            }
            _ => {}
        }
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let mut a = App::new()?;
    let l = EventLoop::new()?;
    l.run_app(&mut a)?;
    Ok(())
}
fn load(
    r: &EditorResourceRegistry,
    d: &EditorDocumentId,
) -> Result<PokemonEditorModel, Box<dyn Error>> {
    Ok(PokemonEditorModel::new(PokemonCatalog::from_json(
        &r.load_text(EditorKind::Pokemon, d)?,
    )?)?)
}
fn px(s: PhysicalSize<u32>) -> PixelSize {
    PixelSize::new(s.width.max(1), s.height.max(1))
}
fn ui(
    name: &str,
    types: &[String],
    hp: u16,
    status: &str,
) -> Result<UiTree<Action>, punctum_ui::UiBuildError> {
    UiTree::new(
        Node::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                direction: FlexDirection::Column,
                padding: Insets::all(24),
                gap: 12,
                ..UiStyle::default()
            })
            .with_content(UiContent::Fill(BG))
            .with_children([
                text("宝可梦编辑器", 24, TXT),
                text("结构化资源 kanto-hoenn-pokedex", 14, MUTED),
                button(NAME, &format!("名称：{name}"), true),
                button(
                    TYPE,
                    &format!("属性：{}（点击切换）", types.join("/")),
                    false,
                ),
                row([
                    button(DOWN, &format!("HP -  ({hp})"), false),
                    button(UP, "HP +", false),
                    button(SAVE, "保存", false),
                ]),
                text(status, 14, MUTED),
            ]),
    )
}
fn row(c: impl IntoIterator<Item = Node>) -> Node {
    Node::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(48),
            direction: FlexDirection::Row,
            gap: 8,
            ..UiStyle::default()
        })
        .with_children(c)
}
fn button(id: u32, s: &str, sel: bool) -> Node {
    Node::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(48),
            direction: FlexDirection::Stack,
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            padding: Insets::all(6),
            ..UiStyle::default()
        })
        .with_action(Action::Id(id))
        .with_content(UiContent::Fill(if sel { SEL } else { BTN }))
        .with_children([text(s, 16, TXT)])
}
fn text(s: &str, z: u32, c: UiColor) -> Node {
    Node::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(z + 12),
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        })
        .with_content(UiContent::Text {
            content: s.into(),
            color: c,
            font_size: z,
        })
}
