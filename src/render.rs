pub type RenderTargetId = uint;
pub type TextureId = uint;

pub trait Renderer {
    fn draw_texture(&mut self, texture: TextureId);
    fn draw_texture_on_target(&mut self, target: RenderTargetId, texture: TextureId);
}