use crate::rendering::bind_tracker::TextureBinding::Empty;
use dawn_graphics::gl::raii::texture::{GLTexture, Texture2D, TextureCube};
use dawn_graphics::gl::raii::vertex_array::VertexArray;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum TextureBinding {
    Empty,
    Texture2D(u32),
    TextureCube(u32),
}

#[cfg(any())]
mod stat_impl {
    pub struct Stat {
        early_returns: u32,
        binds: u32,
    }

    impl Stat {
        pub fn new() -> Self {
            Stat {
                early_returns: 0,
                binds: 0,
            }
        }

        pub fn early_return(&mut self) {
            self.early_returns += 1;
        }

        pub fn bind(&mut self) {
            self.binds += 1;
        }

        pub fn notify(&self) {
            println!(
                "Early Returns = {}, Binds = {}",
                self.early_returns, self.binds
            );
        }
    }
}

#[cfg(all())]
mod stat_impl {
    pub struct Stat;

    impl Stat {
        pub fn new() -> Self {
            Stat {}
        }

        pub fn early_return(&mut self) {}

        pub fn bind(&mut self) {}

        pub fn notify(&self) {}
    }
}

use stat_impl::Stat;

pub struct TextureBindTracker<const N: usize> {
    bindings: [TextureBinding; N],
    stat: Stat,
}

impl<const N: usize> TextureBindTracker<N> {
    pub fn new() -> Self {
        TextureBindTracker {
            bindings: [Empty; N],
            stat: Stat::new(),
        }
    }

    pub fn bind2d(&mut self, gl: &glow::Context, index: i32, texture: &Texture2D) {
        assert!(index < N as i32);

        let tid = texture.as_inner().0.get();
        if let TextureBinding::Texture2D(binding) = self.bindings[index as usize] {
            if binding == tid {
                self.stat.early_return();

                // Early return if the texture is already bound
                return;
            }
        }

        self.stat.bind();
        Texture2D::bind(gl, texture, index as u32);
        self.bindings[index as usize] = TextureBinding::Texture2D(tid);
    }

    pub fn bind_cube(&mut self, gl: &glow::Context, index: i32, texture: &TextureCube) {
        assert!(index < N as i32);

        let tid = texture.as_inner().0.get();
        if let TextureBinding::TextureCube(binding) = self.bindings[index as usize] {
            if binding == tid {
                self.stat.early_return();

                // Early return if the texture is already bound
                return;
            }
        }

        self.stat.bind();
        TextureCube::bind(gl, texture, index as u32);
        self.bindings[index as usize] = TextureBinding::TextureCube(tid);
    }

    pub fn unbind(&mut self, gl: &glow::Context) {
        for (i, binding) in self.bindings.iter_mut().enumerate() {
            match binding {
                TextureBinding::Texture2D(_) => {
                    Texture2D::unbind(gl, i as u32);
                }
                TextureBinding::TextureCube(_) => {
                    TextureCube::unbind(gl, i as u32);
                }
                Empty => {}
            }

            *binding = Empty;
        }

        self.stat.notify();
        self.stat = Stat::new();
    }
}

pub struct VAOBindTracker {
    bound: Option<u32>,
    stat: Stat,
}

impl VAOBindTracker {
    pub fn new() -> Self {
        VAOBindTracker {
            bound: None,
            stat: Stat::new(),
        }
    }

    pub fn bind(&mut self, gl: &glow::Context, vao: &VertexArray) {
        let vid = vao.as_inner().0.get();
        if let Some(bound_id) = self.bound {
            if bound_id == vid {
                self.stat.early_return();

                // Early return if the VAO is already bound
                return;
            }
        }

        self.stat.bind();
        VertexArray::bind(gl, vao);
        self.bound = Some(vid);
    }

    pub fn unbind(&mut self, gl: &glow::Context) {
        if self.bound.is_some() {
            VertexArray::unbind(gl);
            self.bound = None;
        }

        self.stat.notify();
        self.stat = Stat::new();
    }
}
