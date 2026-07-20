//! Windows-native trainer catalog editor.

#![forbid(unsafe_code)]

use std::{error::Error, path::PathBuf, sync::Arc};

use editor_application::{EditorCall, EditorCore, EditorDocumentId, EditorKind, EditorOperation};
use editor_ramus_adapter::{EditorRamusRouter, RoutedEditorIntent};
use editor_resource_adapter::EditorResourceRegistry;
use game_assets::{AssetKey, DecodedImage};
use game_foundation::{TrainerCatalog, TrainerEditCommand, TrainerPokemon};
use game_native_target::{FramePlan, NativeAssets, NativeTarget, PresentOutcome, TextScale};
use punctum_gpu::{PixelSize, Rgba8};
use punctum_ui::{
    CrossAlign, Dimension, FlexDirection, Insets, MainAlign, UiColor, UiContent, UiFrame,
    UiNode as RawUiNode, UiSize, UiStyle, UiTree,
};
use trainer_editor_core::{TrainerEditorCommand, TrainerEditorModel};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, Ime, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

const DOCUMENT: &str = "route-trainers";
const CLEAR: Rgba8 = Rgba8::new(18, 21, 25, 255);
const BACKGROUND: UiColor = UiColor::new(18, 21, 25, 255);
const BUTTON: UiColor = UiColor::new(53, 67, 78, 255);
const SELECTED: UiColor = UiColor::new(42, 172, 148, 255);
const TEXT: UiColor = UiColor::new(238, 242, 245, 255);
const MUTED: UiColor = UiColor::new(161, 177, 187, 255);
const NAME: u32 = 1;
const SCRIPT: u32 = 2;
const ADD: u32 = 3;
const REMOVE: u32 = 4;
const SAVE: u32 = 5;

#[derive(Clone)]
enum Action {
    Command(u32),
}
type Node = RawUiNode<Action>;
#[derive(Clone, Copy, Eq, PartialEq)]
enum Field {
    Name,
    Script,
}

struct App {
    registry: EditorResourceRegistry,
    router: EditorRamusRouter,
    document: EditorDocumentId,
    model: TrainerEditorModel,
    field: Field,
    name: String,
    script: String,
    status: String,
    frame: Option<UiFrame<Action>>,
    cursor: Option<PhysicalPosition<f64>>,
    window: Option<Arc<Window>>,
    runtime: Option<NativeTarget<'static>>,
    assets: NativeAssets,
}

impl App {
    fn new() -> Result<Self, Box<dyn Error>> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../");
        let registry = EditorResourceRegistry::standard(root)?;
        let document = EditorDocumentId::new(DOCUMENT)?;
        let model = load(&registry, &document)?;
        let (name, script) = model
            .catalog()
            .trainers()
            .first()
            .map(|trainer| (trainer.name().to_owned(), trainer.script().to_owned()))
            .ok_or("trainer catalog is empty")?;
        Ok(Self {
            registry,
            router: EditorRamusRouter::new()?,
            document,
            model,
            field: Field::Name,
            name,
            script,
            status: "已加载路线训练家".into(),
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
    fn initialize(&mut self, loop_: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window = Arc::new(
            loop_.create_window(
                Window::default_attributes()
                    .with_title("训练师编辑器")
                    .with_inner_size(LogicalSize::new(960.0, 680.0)),
            )?,
        );
        self.runtime = Some(NativeTarget::new(
            window.clone(),
            pixel_size(window.inner_size()),
            &self.assets,
            CLEAR,
        )?);
        window.set_ime_allowed(true);
        self.window = Some(window);
        self.title();
        self.redraw_request();
        Ok(())
    }
    fn title(&self) {
        if let Some(window) = &self.window {
            window.set_title(&format!(
                "训练师编辑器{}",
                if self.model.is_dirty() { " *" } else { "" }
            ));
        }
    }
    fn redraw_request(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
    fn redraw(&mut self, loop_: &ActiveEventLoop) {
        let Some(size) = self.runtime.as_ref().map(NativeTarget::surface_size) else {
            return;
        };
        let tree = ui(
            &self.model,
            self.field,
            &self.name,
            &self.script,
            &self.status,
        );
        let tree = match tree {
            Ok(tree) => tree,
            Err(_) => {
                self.status = "界面构建失败".into();
                return;
            }
        };
        let Ok(frame) = tree.resolve(UiSize::new(size.width, size.height)) else {
            self.status = "界面布局失败".into();
            return;
        };
        let Ok(plan) = FramePlan::from_ui_frame(&frame, &self.assets, TextScale::new(1, 1, 16, 26))
        else {
            self.status = "界面渲染失败".into();
            return;
        };
        self.frame = Some(frame);
        let (Some(window), Some(runtime)) = (&self.window, &mut self.runtime) else {
            return;
        };
        match runtime.present(&plan) {
            Ok(PresentOutcome::Reconfigured | PresentOutcome::SurfaceLost) => {
                runtime.resize(runtime.surface_size());
                window.request_redraw()
            }
            Ok(_) => {}
            Err(error) => {
                eprintln!("trainer editor presentation failed: {error}");
                loop_.exit()
            }
        }
    }
    fn invoke(
        &mut self,
        operation: EditorOperation,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        let call = EditorCall::new(
            EditorKind::Trainer,
            self.document.clone(),
            operation,
            payload,
        )
        .map_err(|e| e.to_string())?;
        let RoutedEditorIntent::Call(call) =
            self.router.route_call(call).map_err(|e| e.to_string())?
        else {
            return Err("unexpected resource open".into());
        };
        let command = match call.operation() {
            EditorOperation::Inspect => TrainerEditorCommand::Inspect,
            EditorOperation::Validate => TrainerEditorCommand::Validate,
            EditorOperation::Command => TrainerEditorCommand::Edit(
                serde_json::from_value(call.payload().clone()).map_err(|e| e.to_string())?,
            ),
            EditorOperation::Save => TrainerEditorCommand::Save,
        };
        let (next, _) = self.model.execute(command).map_err(|e| e.to_string())?;
        self.model = next;
        if matches!(operation, EditorOperation::Save) {
            let json = self
                .model
                .catalog()
                .to_json_pretty()
                .map_err(|e| e.to_string())?;
            self.registry
                .save_text(EditorKind::Trainer, &self.document, &json)
                .map_err(|e| e.to_string())?;
            self.model = self.model.clone().saved();
        }
        Ok(())
    }
    fn commit(&mut self) {
        let Some(trainer) = self.model.catalog().trainers().first() else {
            return;
        };
        let id = trainer.id().clone();
        let command = match self.field {
            Field::Name => TrainerEditCommand::SetName {
                trainer: id,
                name: self.name.clone(),
            },
            Field::Script => TrainerEditCommand::SetScript {
                trainer: id,
                script: self.script.clone(),
            },
        };
        if let Err(error) = serde_json::to_value(command)
            .map_err(|e| e.to_string())
            .and_then(|value| self.invoke(EditorOperation::Command, value))
        {
            self.status = format!("错误：{error}")
        } else {
            self.status = "已修改训练师".into()
        }
        self.title();
        self.redraw_request();
    }
    fn click(&mut self) {
        let Some(cursor) = self.cursor else { return };
        let Some(Action::Command(id)) = self
            .frame
            .as_ref()
            .and_then(|frame| {
                frame.action_hit_at(cursor.x.max(0.0) as u32, cursor.y.max(0.0) as u32)
            })
            .map(|hit| hit.action.clone())
        else {
            return;
        };
        match id {
            NAME => self.field = Field::Name,
            SCRIPT => self.field = Field::Script,
            ADD => self.edit_roster(true),
            REMOVE => self.edit_roster(false),
            SAVE => {
                if let Err(error) = self.invoke(EditorOperation::Save, serde_json::Value::Null) {
                    self.status = format!("错误：{error}")
                } else {
                    self.status = "已保存".into()
                }
            }
            _ => {}
        }
        self.title();
        self.redraw_request();
    }
    fn edit_roster(&mut self, add: bool) {
        let Some(id) = self
            .model
            .catalog()
            .trainers()
            .first()
            .map(|t| t.id().clone())
        else {
            return;
        };
        let command = if add {
            TrainerEditCommand::AddPokemon {
                trainer: id,
                pokemon: match TrainerPokemon::new("Poochyena", 6) {
                    Ok(value) => value,
                    Err(error) => {
                        self.status = format!("错误：{error}");
                        return;
                    }
                },
            }
        } else {
            let length = self
                .model
                .catalog()
                .trainers()
                .first()
                .map(|t| t.pokemon().len())
                .unwrap_or(0);
            if length <= 1 {
                self.status = "训练师至少需要一只宝可梦".into();
                return;
            }
            TrainerEditCommand::RemovePokemon {
                trainer: id,
                slot: length - 1,
            }
        };
        match serde_json::to_value(command)
            .map_err(|e| e.to_string())
            .and_then(|v| self.invoke(EditorOperation::Command, v))
        {
            Ok(()) => self.status = "已修改队伍".into(),
            Err(e) => self.status = format!("错误：{e}"),
        }
    }
    fn input(&mut self, text: String) {
        match self.field {
            Field::Name => self.name.push_str(&text),
            Field::Script => self.script.push_str(&text),
        }
        self.commit()
    }
    fn backspace(&mut self) {
        match self.field {
            Field::Name => {
                self.name.pop();
            }
            Field::Script => {
                self.script.pop();
            }
        }
        self.commit()
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, loop_: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(e) = self.initialize(loop_)
        {
            eprintln!("trainer editor init failed: {e}");
            loop_.exit()
        }
    }
    fn window_event(&mut self, loop_: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.window.as_ref().map(|w| w.id()) != Some(id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => loop_.exit(),
            WindowEvent::RedrawRequested => self.redraw(loop_),
            WindowEvent::Resized(s) => {
                if let Some(r) = &mut self.runtime {
                    r.resize(pixel_size(s))
                }
                self.redraw_request()
            }
            WindowEvent::CursorMoved { position, .. } => self.cursor = Some(position),
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.click(),
            WindowEvent::Ime(Ime::Commit(text)) => self.input(text),
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && matches!(event.logical_key, Key::Named(NamedKey::Backspace)) =>
            {
                self.backspace()
            }
            _ => {}
        }
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let mut app = App::new()?;
    let loop_ = EventLoop::new()?;
    loop_.run_app(&mut app)?;
    Ok(())
}
fn load(
    registry: &EditorResourceRegistry,
    document: &EditorDocumentId,
) -> Result<TrainerEditorModel, Box<dyn Error>> {
    Ok(TrainerEditorModel::new(TrainerCatalog::from_json(
        &registry.load_text(EditorKind::Trainer, document)?,
    )?)?)
}
fn pixel_size(size: PhysicalSize<u32>) -> PixelSize {
    PixelSize::new(size.width.max(1), size.height.max(1))
}
fn ui(
    model: &TrainerEditorModel,
    field: Field,
    name: &str,
    script: &str,
    status: &str,
) -> Result<UiTree<Action>, punctum_ui::UiBuildError> {
    let trainer = model.catalog().trainers().first();
    let roster = trainer
        .map(|t| {
            t.pokemon()
                .iter()
                .map(|p| format!("{} Lv{}", p.species(), p.level()))
                .collect::<Vec<_>>()
                .join("  ")
        })
        .unwrap_or_default();
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
            .with_content(UiContent::Fill(BACKGROUND))
            .with_children([
                text("训练师编辑器", 24, TEXT),
                text("路线训练家 / 结构化资源 route-trainers", 14, MUTED),
                button(NAME, &format!("姓名：{name}"), field == Field::Name),
                button(SCRIPT, &format!("脚本：{script}"), field == Field::Script),
                text(&format!("队伍：{roster}"), 16, TEXT),
                row([
                    button(ADD, "添加 Poochyena Lv6", false),
                    button(REMOVE, "移除最后一只", false),
                    button(SAVE, "保存", false),
                ]),
                text(status, 14, MUTED),
            ]),
    )
}
fn row(children: impl IntoIterator<Item = Node>) -> Node {
    Node::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(48),
            direction: FlexDirection::Row,
            gap: 8,
            ..UiStyle::default()
        })
        .with_children(children)
}
fn button(id: u32, label: &str, selected: bool) -> Node {
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
        .with_action(Action::Command(id))
        .with_content(UiContent::Fill(if selected { SELECTED } else { BUTTON }))
        .with_children([text(label, 16, TEXT)])
}
fn text(content: &str, size: u32, color: UiColor) -> Node {
    Node::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(size + 12),
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        })
        .with_content(UiContent::Text {
            content: content.into(),
            color,
            font_size: size,
        })
}
