//! 独立动画调试台：单独预览天气覆盖与粒子动画，便于迭代 shader 效果。

#![forbid(unsafe_code)]

use std::{error::Error, sync::Arc};

use game_assets::{AssetKey, DecodedImage};
use game_native_target::{
    FramePlan, NativeAssets, NativeTarget, PresentOutcome, TextScale, instance_for_event_loop,
};
use punctum_gpu::{PixelSize, Rgba8};
use punctum_ui::{
    Dimension, FlexDirection, Insets, Position, UiBorderRadius, UiColor, UiContent, UiNode, UiSize,
    UiStyle, UiTree,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

const TEXT_SCALE: TextScale = TextScale::new(3, 5, 10, 28);
const BG: Rgba8 = Rgba8::new(30, 42, 56, 255);

#[derive(Clone, Copy, PartialEq, Eq)]
enum WeatherKind {
    None,
    Rain,
    Sandstorm,
    Sun,
    Hail,
}

impl WeatherKind {
    fn params(self) -> Option<(u32, Rgba8)> {
        match self {
            WeatherKind::None => None,
            WeatherKind::Rain => Some((0, Rgba8::new(120, 160, 220, 90))),
            WeatherKind::Sandstorm => Some((1, Rgba8::new(205, 180, 110, 110))),
            WeatherKind::Sun => Some((2, Rgba8::new(255, 220, 140, 80))),
            WeatherKind::Hail => Some((3, Rgba8::new(180, 210, 235, 90))),
        }
    }

    fn label(self) -> &'static str {
        match self {
            WeatherKind::None => "无",
            WeatherKind::Rain => "雨天",
            WeatherKind::Sandstorm => "沙暴",
            WeatherKind::Sun => "晴天",
            WeatherKind::Hail => "冰雹",
        }
    }

    fn from_key(code: KeyCode) -> Option<Self> {
        match code {
            KeyCode::Digit0 | KeyCode::Numpad0 => Some(WeatherKind::None),
            KeyCode::Digit1 | KeyCode::Numpad1 => Some(WeatherKind::Rain),
            KeyCode::Digit2 | KeyCode::Numpad2 => Some(WeatherKind::Sandstorm),
            KeyCode::Digit3 | KeyCode::Numpad3 => Some(WeatherKind::Sun),
            KeyCode::Digit4 | KeyCode::Numpad4 => Some(WeatherKind::Hail),
            _ => None,
        }
    }

    fn next(self) -> Self {
        match self {
            WeatherKind::None => WeatherKind::Rain,
            WeatherKind::Rain => WeatherKind::Sandstorm,
            WeatherKind::Sandstorm => WeatherKind::Sun,
            WeatherKind::Sun => WeatherKind::Hail,
            WeatherKind::Hail => WeatherKind::None,
        }
    }
}

struct LabApp {
    window: Option<Arc<Window>>,
    target: Option<NativeTarget<'static>>,
    assets: Option<NativeAssets>,
    weather: WeatherKind,
    frame: u64,
}

impl LabApp {
    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("动画调试台（0-4 切换天气，空格轮换）")
                        .with_inner_size(LogicalSize::new(960.0, 720.0)),
                )
                .map_err(|error| format!("create window: {error}"))?,
        );
        let instance = instance_for_event_loop(event_loop);
        let white = DecodedImage::solid(Rgba8::new(255, 255, 255, 255));
        let assets = NativeAssets::new(vec![(
            AssetKey::from_resource_template("solid/white".into()),
            white,
        )])
        .map_err(|error| format!("build assets: {error}"))?;
        let target = NativeTarget::new(
            &instance,
            window.clone(),
            pixel_size(window.inner_size()),
            &assets,
            BG,
        )
        .map_err(|error| format!("build target: {error}"))?;
        window.request_redraw();
        self.window = Some(window);
        self.assets = Some(assets);
        self.target = Some(target);
        Ok(())
    }

    fn build_tree(&self) -> Result<UiTree, punctum_ui::UiBuildError> {
        let mut children = vec![
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    ..UiStyle::default()
                })
                .with_content(UiContent::Fill(ui_color(BG))),
        ];
        if let Some((pattern, color)) = self.weather.params() {
            children.push(
                UiNode::auto()
                    .with_style(UiStyle {
                        position: Position::Absolute { left: 0, top: 0 },
                        width: Dimension::Fill,
                        height: Dimension::Fill,
                        ..UiStyle::default()
                    })
                    .with_content(UiContent::Weather {
                        pattern,
                        frame: self.frame as u32,
                        color: ui_color(color),
                    }),
            );
        }
        children.push(
            UiNode::auto()
                .with_style(UiStyle {
                    position: Position::Absolute { left: 16, top: 16 },
                    padding: Insets::all(10),
                    border_radius: UiBorderRadius::all(10),
                    ..UiStyle::default()
                })
                .with_content(UiContent::Fill(UiColor::new(10, 14, 20, 200)))
                .with_children([UiNode::auto()
                    .with_style(UiStyle {
                        padding: Insets::all(2),
                        ..UiStyle::default()
                    })
                    .with_content(UiContent::Text {
                        content: format!("天气：{}    帧 {}", self.weather.label(), self.frame),
                        color: UiColor::new(240, 230, 190, 255),
                        font_size: 18,
                    })]),
        );
        UiTree::new(
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    direction: FlexDirection::Stack,
                    ..UiStyle::default()
                })
                .with_children(children),
        )
    }

    fn redraw(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let size = UiSize::new(window.inner_size().width, window.inner_size().height);
        let tree = match self.build_tree() {
            Ok(tree) => tree,
            Err(error) => {
                eprintln!("tree build failed: {error}");
                return;
            }
        };
        let frame = match tree.resolve(size) {
            Ok(frame) => frame,
            Err(error) => {
                eprintln!("layout failed: {error}");
                return;
            }
        };
        let (Some(target), Some(assets)) = (&mut self.target, &mut self.assets) else {
            return;
        };
        let plan = match FramePlan::from_ui_frame(&frame, assets, TEXT_SCALE) {
            Ok(plan) => plan,
            Err(error) => {
                eprintln!("planning failed: {error}");
                return;
            }
        };
        match target.present(&plan) {
            Ok(PresentOutcome::Reconfigured | PresentOutcome::SurfaceLost) => {
                target.resize(target.surface_size());
            }
            Ok(_) => {}
            Err(error) => {
                eprintln!("presentation failed: {error}");
                return;
            }
        }
        self.frame = self.frame.wrapping_add(1);
        window.request_redraw();
    }
}

impl ApplicationHandler for LabApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.initialize(event_loop)
        {
            eprintln!("lab initialization failed: {error}");
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        if window.id() != window_id {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(target) = &mut self.target {
                    target.resize(pixel_size(size));
                }
                window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(target) = &mut self.target {
                    target.resize(pixel_size(window.inner_size()));
                }
                window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if let Some(weather) = WeatherKind::from_key(code) {
                        self.weather = weather;
                    } else if code == KeyCode::Space {
                        self.weather = self.weather.next();
                    }
                }
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }
}

fn pixel_size(size: PhysicalSize<u32>) -> PixelSize {
    PixelSize::new(size.width, size.height)
}

fn ui_color(color: Rgba8) -> UiColor {
    UiColor::new(color.red, color.green, color.blue, color.alpha)
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = LabApp {
        window: None,
        target: None,
        assets: None,
        weather: WeatherKind::None,
        frame: 0,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
