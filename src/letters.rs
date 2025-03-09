type Vert = [f32; 3];

//The vertex buffer desc of Vert
const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];
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
            verts.push([x, y, 0.0]);
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
        self.vert_op(|arr| [x * arr[0], y * arr[1], z * arr[2]])
    }

    fn vert_op<F>(mut self, f: F) -> Self 
    where F: Fn([f32;3]) -> [f32;3] {
        for vert in &mut self.verts {
            *vert = f(*vert)
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
    m.flip().vert_op(|arr| [arr[0], ((arr[1] - 0.5) * -1.0) + 0.5, arr[2]])
}
//
//fn mirror_z(self) -> Self {
//    self.flip().mult(1.0, 1.0, -1.0)
//}

pub fn create_alphabet_models() -> Vec<Model> {
    let a = Model::rect_2d( // Diagonal part of A
        [
            (0.3, 0.1),
            (0.5, 0.1),
            (0.1, 1.0),
            (0.0, 0.85),
        ]
    ).append_apply(mirror_x).append_rect_2d( // Center bar of A
        [
            (0.25, 0.45),
            (0.2, 0.6),
            (-0.2, 0.6),
            (-0.25, 0.45),
        ]
    ).append_tri_2d( // Additional tri to connect the two diagonal parts of A
        [
            (0.0, 0.85),
            (0.1, 1.0),
            (-0.1, 1.0),
        ]
    );
    let b = Model::new_2d(&[], &[]);
    let c = Model::new_2d(&[], &[]);
    let d = Model::new_2d(&[], &[]);
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
    let f = Model::new_2d(&[], &[]);
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
    let m = Model::new_2d(&[], &[]);
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
    let v = Model::new_2d(&[], &[]);
    let w = Model::new_2d(&[], &[]);
    let x = Model::new_2d(&[], &[]);
    let y = Model::new_2d(&[], &[]);
    let z = Model::new_2d(&[], &[]);
    vec![a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z]
}
