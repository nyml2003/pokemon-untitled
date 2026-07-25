use game_view::GameView;
use punctum_gpu::{
    GpuAtlas, GpuCell, GpuClip, GpuImage, GpuPlanError, SubmissionPlan, Viewport as GridViewport,
    plan_composite,
};
use punctum_grid::Surface;
use punctum_ui::UiFrame;

use crate::{
    NativeAssets, NativeTextLabel, TextScale, error::FramePlanError, ui::plan_ui_frame,
    world::plan_game_view,
};

pub struct FramePass {
    gpu: SubmissionPlan,
    labels: Vec<NativeTextLabel>,
    text_scale: TextScale,
}

impl FramePass {
    pub fn viewport(&self) -> GridViewport {
        self.gpu.viewport
    }

    pub fn gpu(&self) -> &SubmissionPlan {
        &self.gpu
    }

    pub fn labels(&self) -> &[NativeTextLabel] {
        &self.labels
    }

    pub fn text_scale(&self) -> TextScale {
        self.text_scale
    }
}

pub struct FramePlan {
    passes: Vec<FramePass>,
}

impl FramePlan {
    /// Converts an already-resolved pixel UI frame at the adapter boundary.
    /// The UI crate stays independent from atlas IDs and GPU plans.
    pub fn from_ui_frame<Action>(
        frame: &UiFrame<Action>,
        assets: &NativeAssets,
        text_scale: TextScale,
    ) -> Result<Self, FramePlanError> {
        let (gpu, labels) = plan_ui_frame(frame, assets)?;
        Ok(Self::single(gpu, labels, text_scale))
    }

    pub fn from_game_view(
        view: &GameView,
        assets: &NativeAssets,
        viewport: GridViewport,
        text_scale: TextScale,
    ) -> Result<Self, FramePlanError> {
        let (gpu, labels) = plan_game_view(view, assets, viewport)?;
        Ok(Self::single(gpu, labels, text_scale))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        surface: &Surface<GpuCell>,
        images: &[GpuImage],
        atlas: &GpuAtlas,
        max_instances: u32,
        viewport: GridViewport,
        clip: GpuClip,
        labels: impl IntoIterator<Item = NativeTextLabel>,
        text_scale: TextScale,
    ) -> Result<Self, GpuPlanError> {
        Ok(Self::single(
            plan_composite(surface, images, atlas, max_instances, viewport, clip)?,
            labels.into_iter().collect(),
            text_scale,
        ))
    }

    fn single(gpu: SubmissionPlan, labels: Vec<NativeTextLabel>, text_scale: TextScale) -> Self {
        Self {
            passes: vec![FramePass {
                gpu,
                labels,
                text_scale,
            }],
        }
    }

    /// Keeps independent viewport mappings separate while rendering them in order.
    pub fn compose(mut base: Self, overlay: Self) -> Self {
        base.passes.extend(overlay.passes);
        base
    }

    pub fn passes(&self) -> &[FramePass] {
        &self.passes
    }

    pub fn viewport(&self) -> GridViewport {
        self.passes[0].viewport()
    }

    pub fn gpu(&self) -> &SubmissionPlan {
        self.passes[0].gpu()
    }

    pub fn labels(&self) -> &[NativeTextLabel] {
        self.passes[0].labels()
    }

    pub fn text_scale(&self) -> TextScale {
        self.passes[0].text_scale()
    }
}
