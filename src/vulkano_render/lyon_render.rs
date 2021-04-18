use crate::{api::RenderObject, layout::PositionedRenderObject, vulkano_render::VulkanContext};
use lyon::{
    lyon_algorithms::path::{
        builder::PathBuilder,
        geom::{point, Translation},
    },
    lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers},
    tessellation::{path::Path, FillVertexConstructor},
};
use std::sync::Arc;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool},
    command_buffer::{AutoCommandBufferBuilder, DynamicState},
    descriptor::PipelineLayoutAbstract,
    device::Device,
    framebuffer::{RenderPassAbstract, Subpass},
    pipeline::{vertex::SingleBufferDefinition, GraphicsPipeline},
};

mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 450
            layout(push_constant) uniform PushConstantData {
                uint width;
                uint height;
            } params;
            layout(location = 0) in vec2 position;
            void main() {
                gl_Position = vec4((position / (vec2(params.width, params.height) / 2.) - vec2(1.)) * vec2(1., -1.), 0.0, 1.0);
            }
        "
    }
}
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 450
            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = vec4(1., 1., 1., 1.);
            }
        "
    }
}


#[derive(Default, Debug, Clone)]
struct Vertex {
    position: [f32; 2],
}
vulkano::impl_vertex!(Vertex, position);
struct VertexConstructor {}
impl FillVertexConstructor<Vertex> for VertexConstructor {
    fn new_vertex(&mut self, vertex: FillVertex) -> Vertex {
        Vertex { position: [vertex.position().x, vertex.position().y] }
    }
}

pub struct LyonRenderer {
    device: Arc<Device>,
    pipeline: std::sync::Arc<
        GraphicsPipeline<
            SingleBufferDefinition<Vertex>,
            Box<dyn PipelineLayoutAbstract + Send + Sync>,
            std::sync::Arc<dyn RenderPassAbstract + Send + Sync>,
        >,
    >,
}
impl LyonRenderer {
    pub fn new(render_pass: Arc<dyn RenderPassAbstract + Send + Sync>) -> Self {
        let device = VulkanContext::get().device;

        let vs = vertex_shader::Shader::load(device.clone()).unwrap();
        let fs = fragment_shader::Shader::load(device.clone()).unwrap();

        let pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<Vertex>()
                .vertex_shader(vs.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .fragment_shader(fs.main_entry_point(), ())
                .blend_alpha_blending()
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                .build(device.clone())
                .unwrap(),
        );


        Self { pipeline, device }
    }
    pub fn render(
        &self,
        buffer_builder: &mut AutoCommandBufferBuilder,
        dynamic_state: &DynamicState,
        dimensions: &[u32; 2],
        render_objects: Vec<PositionedRenderObject>,
    ) {
        let mut lyon_vertex_buffer: VertexBuffers<Vertex, u16> = VertexBuffers::new();
        let mut fill_tesselator = FillTessellator::new();
        let mut buffers_builder =
            BuffersBuilder::new(&mut lyon_vertex_buffer, VertexConstructor {});

        for render_object in render_objects {
            if let RenderObject::Path(path_generator) = render_object.render_object {
                let untranslated: Path = path_generator(render_object.size);
                let translated = untranslated.transformed(&Translation::new(
                    render_object.position.x,
                    render_object.position.y,
                ));
                fill_tesselator.tessellate_path(
                    translated.as_slice(),
                    &FillOptions::DEFAULT,
                    &mut buffers_builder,
                );
            }
        }

        let vertex_buffer = CpuAccessibleBuffer::<[Vertex]>::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            false,
            lyon_vertex_buffer.vertices.into_iter(),
        )
        .unwrap();

        let index_buffer = CpuAccessibleBuffer::<[u16]>::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            false,
            lyon_vertex_buffer.indices.into_iter(),
        )
        .unwrap();

        let push_constants =
            vertex_shader::ty::PushConstantData { width: dimensions[0], height: dimensions[1] };
        buffer_builder
            .draw_indexed(
                self.pipeline.clone(),
                &dynamic_state,
                vertex_buffer,
                index_buffer,
                (),
                push_constants,
                vec![],
            )
            .unwrap();
    }
}
