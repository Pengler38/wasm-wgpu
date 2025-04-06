
#[derive(Clone)]
pub struct RgbaTexture {
    pub values: Vec<[u8; 4]>, //RGBA
    pub height: u32,
    pub width: u32,
}

impl RgbaTexture {
    pub fn set_pixel(&mut self, x: u32, y: u32, pixel: [u8; 4]) {
        let idx = (x + y * self.width) as usize;
        self.values[idx] = pixel;
    }
}


pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler
}

impl GpuTexture {
    pub fn from_rgbatexture(
        rgba: &RgbaTexture,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
    ) -> Self {
        let texture_size = wgpu::Extent3d {
            width: rgba.width,
            height: rgba.height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: Some(label),
                view_formats: &[],
            }
        );

        queue.write_texture(
            wgpu::TexelCopyTextureInfoBase {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(rgba.values.as_slice()),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * rgba.width),
                rows_per_image: Some(rgba.height),
            },
            texture_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::MirrorRepeat,
            address_mode_v: wgpu::AddressMode::MirrorRepeat,
            address_mode_w: wgpu::AddressMode::MirrorRepeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        GpuTexture {
            texture,
            view,
            sampler,
        }

    }
}
