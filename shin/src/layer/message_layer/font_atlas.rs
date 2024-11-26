use std::sync::Arc;

use shin_core::format::font::{GlyphId, GlyphMipLevel, GlyphTrait, LazyFont};
use shin_render::shaders::types::texture::TextureSource;
use strum::IntoEnumIterator;
use wgpu::TextureFormat;
use winit::dpi::PhysicalSize;

use crate::render::{
    dynamic_atlas::{AtlasImage, DynamicAtlas, ImageProvider},
    overlay::{OverlayCollector, OverlayVisitable},
};

struct FontImageProvider {
    font: Arc<LazyFont>,
}

impl ImageProvider for FontImageProvider {
    const IMAGE_FORMAT: TextureFormat = TextureFormat::R8Unorm;
    const MIPMAP_LEVELS: u32 = 4;
    type Id = GlyphId;

    fn get_image(&self, id: Self::Id) -> (Vec<Vec<u8>>, (u32, u32)) {
        let glyph = self.font.get_glyph(id).unwrap();
        let size = glyph.get_info().texture_size();
        let glyph = glyph.decompress();

        let mut result = Vec::new();
        for mip_level in GlyphMipLevel::iter() {
            let image = glyph.get_image(mip_level);
            result.push(image.to_vec());
        }

        (result, size)
    }
}

const TEXTURE_SIZE: PhysicalSize<u32> = PhysicalSize::new(2048, 2048);

// TODO: later this should migrate away from the MessageLayer and ideally should be shared with all the game
pub struct FontAtlas {
    atlas: DynamicAtlas<FontImageProvider>,
}

const COMMON_CHARACTERS: &str =
    "…\u{3000}、。「」あいうえおかがきくけこさしじすせそただちっつてでとどなにねのはひまめもゃやよらりるれろわをんー亞人代右宮戦真里\u{f8f0}！？";

impl FontAtlas {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, font: Arc<LazyFont>) -> Self {
        let provider = FontImageProvider { font };
        let atlas = DynamicAtlas::new(device, provider, TEXTURE_SIZE, Some("FontAtlas"));

        // Preload some common characters (not unloadable)
        for c in COMMON_CHARACTERS.chars() {
            let glyph_id = atlas.provider().font.get_character_mapping()[c as usize];
            let _ = atlas.get_image(queue, glyph_id);
        }

        Self { atlas }
    }

    pub fn get_font(&self) -> &LazyFont {
        &self.atlas.provider().font
    }

    pub fn texture_source(&self) -> TextureSource {
        self.atlas.texture_source()
    }

    pub fn texture_size(&self) -> PhysicalSize<u32> {
        self.atlas.texture_size()
    }

    pub fn get_glyph(&self, queue: &wgpu::Queue, charcode: u16) -> AtlasImage {
        let glyph_id = self.get_font().get_character_mapping()[charcode as usize];
        self.atlas
            .get_image(queue, glyph_id)
            .expect("Could not fit image in atlas")
    }

    pub fn free_glyph(&self, charcode: u16) {
        let glyph_id = self.get_font().get_character_mapping()[charcode as usize];
        self.atlas.free_image(glyph_id);
    }

    pub fn free_space(&self) -> f32 {
        self.atlas.free_space()
    }
}

impl OverlayVisitable for FontAtlas {
    fn visit_overlay(&self, collector: &mut OverlayCollector) {
        self.atlas.visit_overlay(collector);
    }
}
