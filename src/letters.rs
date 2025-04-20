// letters.rs 
// Preston Engler
//
// Is it more efficient to create models in Blender and import them using widely available rust
// libraries? Of course.
//
// Is it more fun to write helper methods to create models of letters with just a few f32s?
// Definitely.

use crate::texture;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vert {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vert {
    fn new_white(position: [f32; 3]) -> Self {
        Vert {
            position,
            tex_coords: [position[0], position[1]],
        }
    }
}

//The vertex buffer desc of Vert
const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];
pub fn desc() -> wgpu::VertexBufferLayout<'static>{
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vert>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRIBS,
    }
}

#[derive(Clone, Debug)]
pub struct Model {
    pub verts: Vec<Vert>,
    pub tri_idxs: Vec<[u16; 3]>,
}

impl Model {

    pub fn number_indices(&self) -> u32 {
        self.tri_idxs.len() as u32 * 3
    }

    // Takes in verts and indices, except the verts are only the x and y
    fn new_2d(vs: &[(f32, f32)], ts: &[[u16; 3]]) -> Self {
        let mut verts: Vec<Vert> = vec![];
        for &(x, y) in vs {
            verts.push(Vert::new_white([x, y, 0.0]));
        }
        let mut tri_idxs: Vec<[u16; 3]> = vec![];
        for &t in ts {
            tri_idxs.push(t);
        }
        Model {
            verts,
            tri_idxs,
        }
    }

    // Supply the verts in counter-clockwise order so the tri points the right way
    fn tri_2d(vs: [(f32, f32); 3]) -> Self {
        Self::new_2d(&vs, &[[0, 1, 2]])
    }

    // Create a Model (and it's indexed tris) from a 2d tristrip
    // The first 3 verts must form a counter-clockwise tri, then the rest of the verts will follow
    // in a zig-zag fashion
    fn tristrip_2d(vs: &[(f32, f32)]) -> Self {
        let mut indices: Vec<[u16; 3]> = vec![];
        let mut flip = false;
        // Every other tri must be flipped for the tristrip to be the right direction
        for i in 0u16..(vs.len()-2) as u16 {
            if flip {
                indices.push([i, i+2, i+1]);
            } else {
                indices.push([i, i+1, i+2])
            }
            flip = !flip;
        }
        Self::new_2d(&vs, indices.as_slice())
    }

    fn append_tri_2d(self, vs: [(f32, f32); 3]) -> Self {
        self.append(Self::tri_2d(vs))
    }

    // Supply the verts in counter-clockwise order so the tris point the right way
    fn rect_2d(vs: [(f32, f32); 4]) -> Self {
        Self::new_2d(
            &vs,
            &[
                [0, 1, 3],
                [1, 2, 3],
            ],
        )
    }

    fn _subdivided_rect(subdivisions: u32, vs: [(f32, f32); 4]) -> Self {
        let _ = (subdivisions, vs);
        todo!()
    }

    fn append_rect_2d(self, vs: [(f32, f32); 4]) -> Self {
        self.append(Self::rect_2d(vs))
    }

    // Appends self with f applied to a copy of self
    fn append_apply(self, f: impl FnOnce(Model) -> Model) -> Self{
        let clone = self.clone();
        self.append(f(clone))
    }

    // Apply must change the indices appropriately to work with the right verts
    // TODO: optimize model by checking if a vert is used already, combine those if possible
    fn append(mut self, mut m: Model) -> Self {
        //Correct m's indices by adding the len of self.verts
        for tri_idx in &mut m.tri_idxs {
            for idx in tri_idx {
                *idx += self.verts.len() as u16;
            }
        }
        self.tri_idxs.append(&mut m.tri_idxs);
        self.verts.append(&mut m.verts);
        self
    }
    
    //Flips the triangle so it's pointing in the opposite direction
    fn flip(mut self) -> Self {
        for idx in &mut self.tri_idxs {
            *idx = [
                idx[0],
                idx[2],
                idx[1],
            ];
        }
        self
    }

    fn mult(self, x: f32, y: f32, z: f32) -> Model {
        self.vert_mod( |arr| [x * arr[0], y * arr[1], z * arr[2]] )
    }

    // uses function f on all the vert positions
    fn vert_mod<F>(mut self, f: F) -> Self 
    where F: Fn([f32;3]) -> [f32;3] {
        for vert in &mut self.verts {
            *vert = Vert{ position: f(vert.position), tex_coords: vert.tex_coords }
        }
        self
    }

    // Resets the texture coordinates to = the x+0.5 and y vertex positions
    // Use only when the model x and y coords are within x=[-0.5,0.5] and y=[0,1],
    // unless you actually want clamping/wrapping on the texture
    fn reset_tex_coords(mut self) -> Self {
        for vert in &mut self.verts {
            vert.tex_coords = [vert.position[0] + 0.5, vert.position[1]];
        }
        self
    }

    // Deduplicates vertices. Remember to check for 0.0 == -0.0
    fn _optimizing_pass(self) -> Model {
        todo!()
    }

    //Pass in a list of the exterior edges to extrude, or can I automatically detect exterior
    //edges?
    fn _extrude(self) -> Model {
        todo!()
    }
}

fn mirror_x(m: Model) -> Model {
    m.flip().mult(-1.0, 1.0, 1.0)
}
    
fn mirror_y(m: Model) -> Model {
    m.flip().vert_mod(|arr| [arr[0], ((arr[1] - 0.5) * -1.0) + 0.5, arr[2]])
}

// mirror over '/'
fn mirror_forward_slash(m: Model) -> Model {
    m.flip().vert_mod(|arr| [arr[1] - 0.5, arr[0] + 0.5, arr[2]])
}
// mirror over '\'
fn mirror_back_slash(m: Model) -> Model {
    m.flip().vert_mod(|arr| [0.5 - arr[1], 0.5 - arr[0], arr[2]])
}
//
//fn mirror_z(self) -> Self {
//    self.flip().mult(1.0, 1.0, -1.0)
//}

pub fn create_alphabet_models() -> Vec<Model> {
    // Helper models
    let vertical_line = Model::tristrip_2d(&[
        (-0.5, 0.0),
        (-0.3, 0.0),
        (-0.5, 0.2),
        (-0.3, 0.2),
        (-0.5, 0.4),
        (-0.3, 0.4),
        (-0.5, 0.6),
        (-0.3, 0.6),
        (-0.5, 0.8),
        (-0.3, 0.8),
        (-0.5, 1.0),
        (-0.3, 1.0),
    ]);
    let vertical_line_thick = vertical_line.clone().vert_mod(|a| [(a[0] + 0.5) * 1.5 - 0.5, a[1], a[2]]);
    // Arc with dimensions x=[0.15, 0.5], y=[0.0, 0.35]
    let arc = Model::tristrip_2d(&[
        (0.15,0.0),
        (0.15,0.2),
        (0.25,0.02),
        (0.25, 0.25),
        (0.4, 0.10),
    ]).flip().append_apply(mirror_back_slash);

    // Letter models
    let v = Model::rect_2d( // Diagonal part of V
        [
            (0.3, 0.9),
            (0.0, 0.15),
            (0.1, 0.0),
            (0.5, 0.9),
        ]
    ).append_apply(mirror_x).append_tri_2d( // Additional tri to connect the two diagonal parts of V
        [
            (0.0, 0.15),
            (-0.1, 0.0),
            (0.1, 0.0),
        ]
    );
    let a = mirror_y(v.clone()).append_rect_2d( // Center bar of A
        [
            (0.25, 0.45),
            (0.2, 0.6),
            (-0.2, 0.6),
            (-0.25, 0.45),
        ]
    );
    let c = Model::new_2d(&[], &[]);
    let d = vertical_line_thick.clone().append(arc.clone().vert_mod(
        |a| [(a[0] - 0.15) / 0.35 * 0.6 - 0.2, a[1], a[2]]
    ).append(Model::tristrip_2d(&[
        (0.057142, 0.35),
        (0.4, 0.35),
        (0.1, 0.5),
        (0.43, 0.5),
    ])).append_apply(mirror_y));
    let b = d.clone() // B is just 2 D's
        .vert_mod(|v| [v[0], (v[1] * 0.5) + 0.5, v[2]])
        .append(
            d.clone()
            .vert_mod(|v| [v[0], v[1] * 0.5, v[2]])
        );
    let e = Model::tristrip_2d( // The horizontal E parts
        &[
            (0.5, 0.0),
            (0.5, 0.2),
            (0.25, 0.0),
            (0.25, 0.2),
            (0.0, 0.0),
            (0.0, 0.2),
            (-0.25, 0.0),
            (-0.25, 0.2),
            (-0.3, 0.0),
            (-0.3, 0.2),
        ]
    ).append_apply(mirror_y).append( // The middle horizontal E part
        Model::tristrip_2d(&[
            (0.5, 0.4),
            (0.5, 0.6),
            (0.25, 0.4),
            (0.25, 0.6),
            (0.0, 0.4),
            (0.0, 0.6),
            (-0.3, 0.4),
            (-0.3, 0.6),
        ])
    ).append( // The Vertical E part
        vertical_line.clone()
    );
    let f = Model::new_2d(&[], &[]); //F shares parts with E
    let g = Model::new_2d(&[], &[]);
    let h = vertical_line_thick.clone( // Vertical part of H
    ).append_apply(mirror_x).append( // Horizontal part of H
        Model::tristrip_2d(&[
            (-0.2, 0.6),
            (-0.2, 0.4),
            (0.0, 0.6),
            (0.0, 0.4),
            (0.2, 0.6),
            (0.2, 0.4),
        ])
    );
    let i = Model::new_2d(&[], &[]);
    let j = Model::new_2d(&[], &[]);
    let k = Model::new_2d(&[], &[]);
    let l = Model::tristrip_2d( // The horizontal L portion
        &[
            (0.5, 0.0),
            (0.5, 0.2),
            (0.25, 0.0),
            (0.25, 0.2),
            (0.0, 0.0),
            (0.0, 0.2),
            (-0.25, 0.0),
            (-0.25, 0.2),
            (-0.3, 0.0),
            (-0.3, 0.2),
            (-0.5, 0.0),
        ]
    ).append_apply(mirror_forward_slash);
    // m will be done at a later line
    let n = Model::new_2d(&[], &[]);
    let o = arc.clone( // The diagonal part of the O
    ).append_apply(mirror_y).append( // The vertical part of the O
        Model::tristrip_2d(&[
            (0.3,0.35),
            (0.5,0.35),
            (0.3,0.5),
            (0.5,0.5),
            (0.3,0.65),
            (0.5,0.65),
        ])
    ).append_apply(mirror_x).append(
        Model::tristrip_2d( // The horizontal part of the O
            &[
                (-0.15,0.2),
                (-0.15, 0.0),
                (0.0, 0.2),
                (0.0, 0.0),
                (0.15, 0.2),
                (0.15, 0.0),
            ]
        ).append_apply(mirror_y)
    );
    let p = arc.clone().vert_mod(
        |a| [(a[0] - 0.15) / 0.35 * 0.6 - 0.2, a[1] + 0.15, a[2]]
    ).append_apply(mirror_y).vert_mod(
        |a| [a[0], a[1] + 0.15, a[2]]
    ).append(Model::tristrip_2d(&[ // Modified copy of vertical_line_thick to match verts with the curve
        (-0.5, 0.0),
        (-0.2, 0.0),
        (-0.5, 0.2),
        (-0.2, 0.2),
        (-0.5, 0.3),
        (-0.2, 0.3),
        (-0.5, 0.5),
        (-0.2, 0.5),
        (-0.5, 0.6),
        (-0.2, 0.6),
        (-0.5, 0.8),
        (-0.2, 0.8),
        (-0.5, 1.0),
        (-0.2, 1.0),
    ]));
    let q = Model::new_2d(&[], &[]);
    let r = p.clone().append(Model::tristrip_2d(&[
        (-0.02857, 0.32),
        (-0.2, 0.3),
        (0.1, 0.25),
        (-0.1, 0.25),
        (0.25, 0.15),
        (0.0, 0.15),
        (0.4, 0.0),
        (0.1, 0.0),
    ]));
    let s = Model::new_2d(&[], &[]);
    let t = Model::new_2d(&[], &[]);
    let u = Model::new_2d(&[], &[]);
    let w = Model::tristrip_2d(
        &[
            (0.0, 1.0),
            (0.1, 1.0),
            (0.0, 0.65),
            (0.15, 0.7),
            (0.05, 0.3),
            (0.20, 0.4),
            (0.1, 0.0),
            (0.20, 0.0),
        ]
    ).append(
        Model::tristrip_2d(&[
            (0.20, 0.0),
            (0.20, 0.4),
            (0.3, 0.0),
            (0.25, 0.7),
            (0.4, 0.5),
            (0.3, 1.0),
            (0.5, 1.0),
        ])
    ).flip().append_apply(mirror_x);
    let x = Model::new_2d(&[], &[]);
    let y = Model::new_2d(&[], &[]);
    let z = Model::new_2d(&[], &[]);

    let m = mirror_y(w.clone()); //Simply an upside down M

    vec![a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z]
        .into_iter()
        .map(|l| l.reset_tex_coords())
        .collect()
}

// Outputs a generated RGBA texture
// Just a simple test gradient for now
pub fn create_letter_texture() -> texture::RgbaTexture {
    const SIZE: usize = 512;
    let mut tex = texture::RgbaTexture {
        values: Vec::with_capacity(SIZE * SIZE),
        height: SIZE as u32,
        width: SIZE as u32,
    };
    tex.values.resize(SIZE * SIZE, [0, 0, 0, 0]);
    
    for y in 0..tex.height {
        for x in 0..tex.width {
            let a = ((y * 255) as f32 / SIZE as f32) as u8;
            tex.set_pixel(x, y, [a, a, 100, 255]);
        }
    }
    tex
}
