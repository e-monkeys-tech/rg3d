use crate::core::algebra::Matrix2;
use crate::{
    core::{
        algebra::{Matrix3, Matrix4, Vector2, Vector3, Vector4},
        color::Color,
    },
    renderer::framework::{error::FrameworkError, gpu_texture::GpuTexture, state::PipelineState},
    utils::log::{Log, MessageKind},
};
use glow::HasContext;
use std::{cell::RefCell, marker::PhantomData, rc::Rc};

pub struct GpuProgram {
    state: *mut PipelineState,
    id: glow::Program,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
}

#[derive(Clone, Debug)]
pub struct UniformLocation {
    id: glow::UniformLocation,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
}

unsafe fn create_shader(
    state: &mut PipelineState,
    name: String,
    actual_type: u32,
    source: &str,
) -> Result<glow::Shader, FrameworkError> {
    let merged_source = prepare_source_code(source);

    let shader = state.gl.create_shader(actual_type)?;
    state.gl.shader_source(shader, &merged_source);
    state.gl.compile_shader(shader);

    let status = state.gl.get_shader_compile_status(shader);
    let compilation_message = state.gl.get_shader_info_log(shader);

    if !status {
        Log::writeln(
            MessageKind::Error,
            format!("Failed to compile {} shader: {}", name, compilation_message),
        );
        Err(FrameworkError::ShaderCompilationFailed {
            shader_name: name,
            error_message: compilation_message,
        })
    } else {
        Log::writeln(
            MessageKind::Information,
            format!("Shader {} compiled!\n{}", name, compilation_message),
        );
        Ok(shader)
    }
}

#[allow(clippy::let_and_return)]
fn prepare_source_code(code: &str) -> String {
    let mut shared = "\n// include 'shared.glsl'\n".to_owned();

    // HACK
    #[cfg(target_arch = "wasm32")]
    {
        shared += r#"    
            precision highp float;
            precision lowp usampler2D;
            precision lowp sampler3D;
        "#;
    }

    shared += include_str!("shaders/shared.glsl");
    shared += "\n// end of include\n";

    let code = if let Some(p) = code.find('#') {
        let mut full = code.to_owned();
        let end = p + full[p..].find('\n').unwrap() + 1;
        full.insert_str(end, &shared);
        full
    } else {
        shared += code;
        shared
    };

    // HACK
    #[cfg(target_arch = "wasm32")]
    {
        code.replace("#version 330 core", "#version 300 es")
    }

    #[cfg(not(target_arch = "wasm32"))]
    code
}

pub struct GpuProgramBinding<'a> {
    pub state: &'a mut PipelineState,
    active_sampler: u32,
    id: glow::Program,
}

impl<'a> GpuProgramBinding<'a> {
    pub fn uniform_location(&self, name: &str) -> Option<UniformLocation> {
        unsafe {
            self.state
                .gl
                .get_uniform_location(self.id, name)
                .map(|l| UniformLocation {
                    id: l,
                    thread_mark: Default::default(),
                })
        }
    }

    #[inline(always)]
    pub fn set_texture(
        &mut self,
        location: &UniformLocation,
        texture: &Rc<RefCell<GpuTexture>>,
    ) -> &mut Self {
        unsafe {
            self.state
                .gl
                .uniform_1_i32(Some(&location.id), self.active_sampler as i32)
        };
        texture.borrow().bind(self.state, self.active_sampler);
        self.active_sampler += 1;
        self
    }

    #[inline(always)]
    pub fn set_bool(&mut self, location: &UniformLocation, value: bool) -> &mut Self {
        unsafe {
            self.state.gl.uniform_1_i32(
                Some(&location.id),
                if value { glow::TRUE } else { glow::FALSE } as i32,
            );
        }
        self
    }

    #[inline(always)]
    pub fn set_i32(&mut self, location: &UniformLocation, value: i32) -> &mut Self {
        unsafe {
            self.state.gl.uniform_1_i32(Some(&location.id), value);
        }
        self
    }

    #[inline(always)]
    pub fn set_u32(&mut self, location: &UniformLocation, value: u32) -> &mut Self {
        unsafe {
            self.state.gl.uniform_1_u32(Some(&location.id), value);
        }
        self
    }

    #[inline(always)]
    pub fn set_f32(&mut self, location: &UniformLocation, value: f32) -> &mut Self {
        unsafe {
            self.state.gl.uniform_1_f32(Some(&location.id), value);
        }
        self
    }

    #[inline(always)]
    pub fn set_vector2(&mut self, location: &UniformLocation, value: &Vector2<f32>) -> &mut Self {
        unsafe {
            self.state
                .gl
                .uniform_2_f32(Some(&location.id), value.x, value.y);
        }
        self
    }

    #[inline(always)]
    pub fn set_vector3(&mut self, location: &UniformLocation, value: &Vector3<f32>) -> &mut Self {
        unsafe {
            self.state
                .gl
                .uniform_3_f32(Some(&location.id), value.x, value.y, value.z);
        }
        self
    }

    #[inline(always)]
    pub fn set_vector4(&mut self, location: &UniformLocation, value: &Vector4<f32>) -> &mut Self {
        unsafe {
            self.state
                .gl
                .uniform_4_f32(Some(&location.id), value.x, value.y, value.z, value.w);
        }
        self
    }

    #[inline(always)]
    pub fn set_i32_slice(&mut self, location: &UniformLocation, value: &[i32]) -> &mut Self {
        unsafe {
            self.state.gl.uniform_1_i32_slice(Some(&location.id), value);
        }
        self
    }

    #[inline(always)]
    pub fn set_u32_slice(&mut self, location: &UniformLocation, value: &[u32]) -> &mut Self {
        unsafe {
            self.state.gl.uniform_1_u32_slice(Some(&location.id), value);
        }
        self
    }

    #[inline(always)]
    pub fn set_f32_slice(&mut self, location: &UniformLocation, value: &[f32]) -> &mut Self {
        unsafe {
            self.state.gl.uniform_1_f32_slice(Some(&location.id), value);
        }
        self
    }

    #[inline(always)]
    pub fn set_vector2_slice(
        &mut self,
        location: &UniformLocation,
        value: &[Vector2<f32>],
    ) -> &mut Self {
        unsafe {
            self.state.gl.uniform_2_f32_slice(
                Some(&location.id),
                std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 2),
            );
        }
        self
    }

    #[inline(always)]
    pub fn set_vector3_slice(
        &mut self,
        location: &UniformLocation,
        value: &[Vector3<f32>],
    ) -> &mut Self {
        unsafe {
            self.state.gl.uniform_3_f32_slice(
                Some(&location.id),
                std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 3),
            );
        }
        self
    }

    #[inline(always)]
    pub fn set_vector4_slice(
        &mut self,
        location: &UniformLocation,
        value: &[Vector4<f32>],
    ) -> &mut Self {
        unsafe {
            self.state.gl.uniform_4_f32_slice(
                Some(&location.id),
                std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 4),
            );
        }
        self
    }

    #[inline(always)]
    pub fn set_matrix2(&mut self, location: &UniformLocation, value: &Matrix2<f32>) -> &mut Self {
        unsafe {
            self.state
                .gl
                .uniform_matrix_2_f32_slice(Some(&location.id), false, value.as_slice());
        }
        self
    }

    #[inline(always)]
    pub fn set_matrix2_array(
        &mut self,
        location: &UniformLocation,
        value: &[Matrix2<f32>],
    ) -> &mut Self {
        unsafe {
            self.state.gl.uniform_matrix_2_f32_slice(
                Some(&location.id),
                false,
                std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 4),
            );
        }
        self
    }

    #[inline(always)]
    pub fn set_matrix3(&mut self, location: &UniformLocation, value: &Matrix3<f32>) -> &mut Self {
        unsafe {
            self.state
                .gl
                .uniform_matrix_3_f32_slice(Some(&location.id), false, value.as_slice());
        }
        self
    }

    #[inline(always)]
    pub fn set_matrix3_array(
        &mut self,
        location: &UniformLocation,
        value: &[Matrix3<f32>],
    ) -> &mut Self {
        unsafe {
            self.state.gl.uniform_matrix_3_f32_slice(
                Some(&location.id),
                false,
                std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 9),
            );
        }
        self
    }

    #[inline(always)]
    pub fn set_matrix4(&mut self, location: &UniformLocation, value: &Matrix4<f32>) -> &mut Self {
        unsafe {
            self.state
                .gl
                .uniform_matrix_4_f32_slice(Some(&location.id), false, value.as_slice());
        }
        self
    }

    #[inline(always)]
    pub fn set_matrix4_array(
        &mut self,
        location: &UniformLocation,
        value: &[Matrix4<f32>],
    ) -> &mut Self {
        unsafe {
            self.state.gl.uniform_matrix_4_f32_slice(
                Some(&location.id),
                false,
                std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 16),
            );
        }
        self
    }

    #[inline(always)]
    pub fn set_linear_color(&mut self, location: &UniformLocation, value: &Color) -> &mut Self {
        unsafe {
            let srgb_a = value.srgb_to_linear_f32();
            self.state
                .gl
                .uniform_4_f32(Some(&location.id), srgb_a.x, srgb_a.y, srgb_a.z, srgb_a.w);
        }
        self
    }

    #[inline(always)]
    pub fn set_srgb_color(&mut self, location: &UniformLocation, value: &Color) -> &mut Self {
        unsafe {
            let rgba = value.as_frgba();
            self.state
                .gl
                .uniform_4_f32(Some(&location.id), rgba.x, rgba.y, rgba.z, rgba.w);
        }
        self
    }
}

impl GpuProgram {
    pub fn from_source(
        state: &mut PipelineState,
        name: &str,
        vertex_source: &str,
        fragment_source: &str,
    ) -> Result<GpuProgram, FrameworkError> {
        unsafe {
            let vertex_shader = create_shader(
                state,
                format!("{}_VertexShader", name),
                glow::VERTEX_SHADER,
                vertex_source,
            )?;
            let fragment_shader = create_shader(
                state,
                format!("{}_FragmentShader", name),
                glow::FRAGMENT_SHADER,
                fragment_source,
            )?;
            let program = state.gl.create_program()?;
            state.gl.attach_shader(program, vertex_shader);
            state.gl.delete_shader(vertex_shader);
            state.gl.attach_shader(program, fragment_shader);
            state.gl.delete_shader(fragment_shader);
            state.gl.link_program(program);
            let status = state.gl.get_program_link_status(program);
            let link_message = state.gl.get_program_info_log(program);

            if !status {
                Log::writeln(
                    MessageKind::Error,
                    format!("Failed to link {} shader: {}", name, link_message),
                );
                Err(FrameworkError::ShaderLinkingFailed {
                    shader_name: name.to_owned(),
                    error_message: link_message,
                })
            } else {
                Log::writeln(
                    MessageKind::Information,
                    format!("Shader {} linked!\n{}", name, link_message),
                );
                Ok(Self {
                    state,
                    id: program,
                    thread_mark: PhantomData,
                })
            }
        }
    }

    pub fn uniform_location(
        &self,
        state: &mut PipelineState,
        name: &str,
    ) -> Result<UniformLocation, FrameworkError> {
        unsafe {
            if let Some(id) = state.gl.get_uniform_location(self.id, name) {
                Ok(UniformLocation {
                    id,
                    thread_mark: PhantomData,
                })
            } else {
                Err(FrameworkError::UnableToFindShaderUniform(name.to_owned()))
            }
        }
    }

    pub fn bind<'a>(&self, state: &'a mut PipelineState) -> GpuProgramBinding<'a> {
        state.set_program(Some(self.id));
        GpuProgramBinding {
            state,
            active_sampler: 0,
            id: self.id,
        }
    }
}

impl Drop for GpuProgram {
    fn drop(&mut self) {
        unsafe {
            (*self.state).gl.delete_program(self.id);
        }
    }
}
