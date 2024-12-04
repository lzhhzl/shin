use std::{sync::Arc, time::Duration};

use anyhow::Context;
use enum_map::{Enum, EnumMap};
use glam::vec4;
use shin_audio::AudioManager;
use shin_core::vm::command::types::LayerProperty;
use shin_input::{Action, ActionState, RawInputState};
use shin_render::{render_pass::RenderPass, shaders::types::vertices::FloatColor4, PassKind};
use shin_window::{AppContext, ShinApp};
use tracing::debug;
use winit::keyboard::KeyCode;

use crate::{
    asset::{
        picture::Picture,
        system::{locate_assets, AssetLoadContext, AssetServer},
    },
    cli::Cli,
    layer::{
        render_params::TransformParams, Layer, NewDrawableLayerWrapper, PictureLayer, TileLayer,
    },
};

#[derive(Debug, Enum)]
pub enum AppAction {
    ToggleFullscreen,
}

impl Action for AppAction {
    fn lower(raw_input_state: &RawInputState) -> EnumMap<Self, bool> {
        EnumMap::from_fn(|action| match action {
            AppAction::ToggleFullscreen => raw_input_state.keyboard.contains(&KeyCode::F11),
        })
    }
}

pub struct App {
    audio_manager: Arc<AudioManager>,
    asset_server: Arc<AssetServer>,
    picture_layer: NewDrawableLayerWrapper<PictureLayer>,
    tile_layer: NewDrawableLayerWrapper<TileLayer>,
}

impl ShinApp for App {
    type Parameters = Cli;
    type EventType = ();
    type ActionType = AppAction;

    fn init(context: AppContext<Self>, cli: Self::Parameters) -> anyhow::Result<Self> {
        let audio_manager = Arc::new(AudioManager::new());

        let asset_io = locate_assets(cli.assets_dir.as_deref()).context("Failed to locate assets. Consult the README for instructions on how to set up the game.")?;

        debug!("Asset IO: {:#?}", asset_io);

        let asset_server = Arc::new(AssetServer::new(
            asset_io.into(),
            AssetLoadContext {
                wgpu_device: context.wgpu.device.clone(),
                wgpu_queue: context.wgpu.queue.clone(),
            },
        ));

        let picture_name = "/picture/text001.pic";

        let picture = asset_server.load_sync::<Picture>(picture_name).unwrap();

        let mut picture_layer = PictureLayer::new(picture, Some(picture_name.to_string()));
        picture_layer
            .properties_mut()
            .property_tweener_mut(LayerProperty::TranslateZ)
            .fast_forward_to(1500.0);

        let picture_layer = NewDrawableLayerWrapper::new(picture_layer);

        let tile_layer = TileLayer::new(
            FloatColor4::PASTEL_PINK,
            vec4(-1088.0, -612.0, 2176.0, 1224.0),
        );
        let tile_layer = NewDrawableLayerWrapper::new(tile_layer);

        Ok(Self {
            audio_manager,
            asset_server,
            picture_layer,
            tile_layer,
        })
    }

    fn custom_event(&mut self, _context: AppContext<Self>, _event: Self::EventType) {
        todo!()
    }

    fn update(
        &mut self,
        context: AppContext<Self>,
        input: EnumMap<Self::ActionType, ActionState>,
        elapsed_time: Duration,
    ) {
        if input[AppAction::ToggleFullscreen].is_clicked {
            context.winit.toggle_fullscreen();
        }
    }

    fn render(&mut self, pass: &mut RenderPass) {
        let transform = TransformParams::default();

        pass.push_debug("opaque_pass");
        self.picture_layer
            .render(pass, &transform, 2, PassKind::Opaque);
        self.tile_layer
            .render(pass, &transform, 1, PassKind::Opaque);
        pass.pop_debug();

        pass.push_debug("transparent_pass");
        self.tile_layer
            .render(pass, &transform, 3, PassKind::Transparent);
        self.picture_layer
            .render(pass, &transform, 4, PassKind::Transparent);
        pass.pop_debug();
    }
}
