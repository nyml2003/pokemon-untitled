//! 不包含地图渲染的玩家页面 demo 原生壳。
//!
//! 该二进制只将鼠标点击转换为 `PageIntent`，并将纯页面 frame 提交给原生目标。
//! 产品命令和存档请求只显示反馈，不会在此处执行。

#![forbid(unsafe_code)]

use std::{error::Error, fmt, sync::Arc};

use game_assets::{AssetKey, DecodedImage};
use game_native_target::{
    FramePlan, NativeAssets, NativeTarget, PresentOutcome, TextScale, instance_for_event_loop,
};
use game_page_model::{
    PageDemo, PageDemoContext, PageEffect, PageIntent, PageState, demo_named, project_page,
};
use game_view::project_page_model_with_notice;
use punctum_gpu::{PixelSize, Rgba8};
use punctum_ui::{UiFrame, UiSize};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

const CLEAR_COLOR: Rgba8 = Rgba8::new(14, 22, 32, 255);
const TEXT_SCALE: TextScale = TextScale::new(1, 1, 14, 28);
const DEFAULT_DEMO: &str = "world-starting-town";

struct PageDemoApp {
    demo: PageDemo,
    context: PageDemoContext,
    state: PageState,
    status: Option<String>,
    assets: NativeAssets,
    frame: Option<UiFrame<PageIntent>>,
    cursor: Option<PhysicalPosition<f64>>,
    window: Option<Arc<Window>>,
    runtime: Option<NativeTarget<'static>>,
}

impl PageDemoApp {
    fn new(demo: PageDemo) -> Result<Self, Box<dyn Error>> {
        let context = demo.context()?;
        let state = demo.initial_state()?;
        let white = AssetKey::from_resource_template("solid/white".into());
        let assets = NativeAssets::new(vec![(
            white,
            DecodedImage::solid(Rgba8::new(255, 255, 255, 255)),
        )])?;
        Ok(Self {
            demo,
            context,
            state,
            status: None,
            assets,
            frame: None,
            cursor: None,
            window: None,
            runtime: None,
        })
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title(self.title())
                    .with_inner_size(LogicalSize::new(960.0, 720.0)),
            )?,
        );
        let instance = instance_for_event_loop(event_loop);
        let runtime = NativeTarget::new(
            &instance,
            window.clone(),
            pixel_size(window.inner_size()),
            &self.assets,
            CLEAR_COLOR,
        )?;
        window.set_ime_allowed(false);
        window.request_redraw();
        self.window = Some(window);
        self.runtime = Some(runtime);
        Ok(())
    }

    fn title(&self) -> String {
        match &self.status {
            Some(status) => format!("玩家页面 Demo · {} · {status}", self.demo.id().as_str()),
            None => format!("玩家页面 Demo · {}", self.demo.id().as_str()),
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn update_title(&self) {
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(surface_size) = self.runtime.as_ref().map(NativeTarget::surface_size) else {
            return;
        };
        let model = match project_page(
            self.context.content(),
            self.context.snapshot(),
            self.state.route(),
        ) {
            Ok(model) => model,
            Err(error) => {
                eprintln!("page demo model construction failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let tree = match project_page_model_with_notice(&model, self.status.as_deref()) {
            Ok(tree) => tree,
            Err(error) => {
                eprintln!("page demo tree construction failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let frame = match tree.resolve(UiSize::new(surface_size.width, surface_size.height)) {
            Ok(frame) => frame,
            Err(error) => {
                eprintln!("page demo layout failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let plan = match FramePlan::from_ui_frame(&frame, &self.assets, TEXT_SCALE) {
            Ok(plan) => plan,
            Err(error) => {
                eprintln!("page demo GPU planning failed: {error}");
                event_loop.exit();
                return;
            }
        };
        self.frame = Some(frame);
        let (Some(window), Some(runtime)) = (&self.window, &mut self.runtime) else {
            return;
        };
        match runtime.present(&plan) {
            Ok(PresentOutcome::Reconfigured | PresentOutcome::SurfaceLost) => {
                runtime.resize(runtime.surface_size());
                window.request_redraw();
            }
            Ok(
                PresentOutcome::Presented
                | PresentOutcome::PresentedAndReconfigured
                | PresentOutcome::SkippedMinimized
                | PresentOutcome::SkippedTimeout
                | PresentOutcome::SkippedOccluded,
            ) => {}
            Err(error) => {
                eprintln!("page demo presentation failed: {error}");
                event_loop.exit();
            }
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if let Some(runtime) = &mut self.runtime {
            runtime.resize(pixel_size(size));
        }
        self.request_redraw();
    }

    fn click(&mut self) {
        let Some(cursor) = self.cursor else {
            return;
        };
        let Some((x, y)) = cursor_position(cursor) else {
            return;
        };
        let Some(intent) = self
            .frame
            .as_ref()
            .and_then(|frame| frame.hit_action(x, y))
            .cloned()
        else {
            return;
        };
        match self.state.clone().transition(intent) {
            Ok((state, effect)) => {
                self.state = state;
                self.status = effect.map(effect_status);
            }
            Err(error) => self.status = Some(format!("操作未执行：{error}")),
        }
        self.update_title();
        self.request_redraw();
    }
}

impl ApplicationHandler for PageDemoApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.initialize(event_loop)
        {
            eprintln!("page demo initialization failed: {error}");
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window.as_ref().map(|window| window.id()) != Some(window_id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            WindowEvent::Resized(size) => self.resize(size),
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = &self.window {
                    self.resize(window.inner_size());
                }
            }
            WindowEvent::CursorMoved { position, .. } => self.cursor = Some(position),
            WindowEvent::CursorLeft { .. } => self.cursor = None,
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.click(),
            _ => {}
        }
    }
}

fn effect_status(effect: PageEffect) -> String {
    match effect {
        PageEffect::SubmitProduct(_) => String::from("产品命令已发出；demo 不会修改产品状态"),
        PageEffect::RequestSave => String::from("存档请求已发出；demo 不会写入文件"),
    }
}

fn cursor_position(position: PhysicalPosition<f64>) -> Option<(u32, u32)> {
    let x = coordinate(position.x)?;
    let y = coordinate(position.y)?;
    Some((x, y))
}

fn coordinate(value: f64) -> Option<u32> {
    if !value.is_finite() || value.is_sign_negative() || value > f64::from(u32::MAX) {
        return None;
    }
    Some(value as u32)
}

fn pixel_size(size: PhysicalSize<u32>) -> PixelSize {
    PixelSize::new(size.width, size.height)
}

#[derive(Debug)]
struct DemoSelectionError(String);

impl fmt::Display for DemoSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for DemoSelectionError {}

fn selected_demo(
    arguments: impl IntoIterator<Item = String>,
) -> Result<PageDemo, DemoSelectionError> {
    let mut arguments = arguments.into_iter();
    let Some(first) = arguments.next() else {
        return demo_named(DEFAULT_DEMO).ok_or_else(|| {
            DemoSelectionError(String::from("default page demo is not registered"))
        });
    };
    if first != "--page-demo" {
        return Err(DemoSelectionError(format!(
            "unknown argument {first}; use --page-demo <PageDemoId>"
        )));
    }
    let Some(name) = arguments.next() else {
        return Err(DemoSelectionError(String::from(
            "--page-demo requires a registered PageDemoId",
        )));
    };
    if arguments.next().is_some() {
        return Err(DemoSelectionError(String::from(
            "only one --page-demo argument is supported",
        )));
    }
    demo_named(&name).ok_or_else(|| DemoSelectionError(format!("unknown page demo {name}")))
}

fn main() -> Result<(), Box<dyn Error>> {
    let demo = selected_demo(std::env::args().skip(1))?;
    let event_loop = EventLoop::new()?;
    let mut app = PageDemoApp::new(demo)?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_DEMO, selected_demo};

    #[test]
    fn selects_default_and_registered_demos() -> Result<(), Box<dyn std::error::Error>> {
        let default = selected_demo(Vec::new())?;
        assert_eq!(default.id().as_str(), DEFAULT_DEMO);
        let selected = selected_demo(vec![
            String::from("--page-demo"),
            String::from("shop-potion-preview"),
        ])?;
        assert_eq!(selected.id().as_str(), "shop-potion-preview");
        Ok(())
    }

    #[test]
    fn rejects_unknown_page_demo() {
        assert!(
            selected_demo(vec![String::from("--page-demo"), String::from("unknown"),]).is_err()
        );
    }
}
