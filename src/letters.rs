// letters.rs 
// Preston Engler
//
// Is it more efficient to create models in Blender and import them using widely available rust
// libraries? Of course.
//
// Is it more fun to write helper methods to create models of letters with just a few f32s?
// Definitely.


#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vert {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vert {
    fn new_white(position: [f32; 3]) -> Self {
        Vert {
            position,
            color: [1.0, 1.0, 1.0],
        }
    }
}

//The vertex buffer desc of Vert
const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
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
        self.vert_pos_op( |arr| [x * arr[0], y * arr[1], z * arr[2]] )
    }

    // uses function f on all the vert positions
    fn vert_pos_op<F>(mut self, f: F) -> Self 
    where F: Fn([f32;3]) -> [f32;3] {
        for vert in &mut self.verts {
            *vert = Vert{ position: f(vert.position), color: vert.color }
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
    m.flip().vert_pos_op(|arr| [arr[0], ((arr[1] - 0.5) * -1.0) + 0.5, arr[2]])
}
//
//fn mirror_z(self) -> Self {
//    self.flip().mult(1.0, 1.0, -1.0)
//}

pub fn create_alphabet_models() -> Vec<Model> {
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
    let d = Model::new_2d(&[], &[]);
    let b = d.clone() // B is just 2 D's
        .vert_pos_op(|v| [v[0], (v[1] * 0.5) + 0.5, v[2]])
        .append(
            d.clone()
            .vert_pos_op(|v| [v[0], v[1] * 0.5, v[2]])
        );
    let e = Model::rect_2d( // Top and bottom flanges of E
        [
            (-0.2, 0.0),
            (0.5, 0.0),
            (0.5, 0.2),
            (-0.2, 0.2),
        ]
    ).append_apply(mirror_y).append_rect_2d( // Vertical part of E
        [
            (-0.5, 0.0),
            (-0.2, 0.0),
            (-0.2, 1.0),
            (-0.5, 1.0),
        ]
    ).append_rect_2d( // Middle flange of E
        [
            (-0.2, 0.4),
            (0.5, 0.4),
            (0.5, 0.6),
            (-0.2, 0.6),
        ]
    );
    let f = Model::new_2d(&[], &[]); //F shares parts with E
    let g = Model::new_2d(&[], &[]);
    let h = Model::rect_2d( // Vertical part of H
        [
            (0.5, 0.0),
            (0.5, 1.0),
            (0.2, 1.0),
            (0.2, 0.0),
        ]
    ).append_apply(mirror_x).append_rect_2d( // Horizontal part of H
        [
            (-0.4, 0.4),
            (0.4, 0.4),
            (0.4, 0.6),
            (-0.4, 0.6),
        ]
    );
    let i = Model::new_2d(&[], &[]);
    let j = Model::new_2d(&[], &[]);
    let k = Model::new_2d(&[], &[]);
    let l = Model::rect_2d( // Horizontal part of L
        [
            (-0.2, 0.0),
            (0.5, 0.0),
            (0.5, 0.2),
            (-0.2, 0.2),
        ]
    ).append_rect_2d( // Vertical part of L
        [
            (-0.5, 0.0),
            (-0.2, 0.0),
            (-0.2, 1.0),
            (-0.5, 1.0),
        ]
    );
    // m will be done at a later line
    let n = Model::new_2d(&[], &[]);
    let o = Model::rect_2d( // The diagonal part of the O
        [
            (0.25,0.0),
            (0.5,0.25),
            (0.3,0.35),
            (0.15,0.2),
        ]
    ).append_apply(mirror_y).append_rect_2d( // The vertical part of the O
        [
            (0.3,0.35),
            (0.5,0.25),
            (0.5,0.75),
            (0.3,0.65),
        ]
    ).append_apply(mirror_x).append(
        Model::rect_2d( // The horizontal part of the O
            [
                (-0.25, 0.0),
                (0.25, 0.0),
                (0.15, 0.2),
                (-0.15,0.2),
            ]
        ).append_apply(mirror_y)
    );
    let p = Model::new_2d(&[], &[]);
    let q = Model::new_2d(&[], &[]);
    let r = Model::new_2d(&[], &[]);
    let s = Model::new_2d(&[], &[]);
    let t = Model::new_2d(&[], &[]);
    let u = Model::new_2d(&[], &[]);
    let w = v.clone()
        .vert_pos_op(|v| [(v[0] * 0.6) + 0.25, v[1], v[2]] ) // Add 0.5 to the x dimension of each vert
        .append_apply(mirror_x);
    let x = Model::new_2d(&[], &[]);
    let y = Model::new_2d(&[], &[]);
    let z = Model::new_2d(&[], &[]);

    let m = mirror_y(w.clone()); //Simply an upside down M

    vec![a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z]
}
