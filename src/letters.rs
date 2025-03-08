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
    fn append_apply(self, f: fn(Model) -> Model) -> Self{
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

    fn mult(&self, x: f32, y: f32, z: f32) -> Model {
        let mut clone = self.clone();
        for vert in &mut clone.verts {
            *vert = [x * vert[0], y * vert[1], z * vert[2]]
        }
        clone
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
    
//fn mirror_y(self) -> Self {
//    self.flip().mult(1.0, -1.0, 1.0)
//}
//
//fn mirror_z(self) -> Self {
//    self.flip().mult(1.0, 1.0, -1.0)
//}

pub fn create_alphabet_models() -> Vec<Model> {
    let a = Model::rect_2d(
        [
            (0.25, 0.1),
            (0.4, 0.1),
            (0.1, 0.9),
            (0.0, 0.8),
        ],
    ).append_apply(mirror_x).append_rect_2d(
        [
            (0.25, 0.35),
            (0.2, 0.5),
            (-0.2, 0.5),
            (-0.25, 0.35),
        ],
    ).append_tri_2d(
        [
            (0.0, 0.8),
            (0.1, 0.9),
            (-0.1, 0.9),
        ],
    );
    let b = Model::new_2d(&[], &[]);
    let c = Model::new_2d(&[], &[]);
    let d = Model::new_2d(&[], &[]);
    let e = Model::new_2d(&[], &[]);
    let f = Model::new_2d(&[], &[]);
    let g = Model::new_2d(&[], &[]);
    let h = Model::new_2d(&[], &[]);
    let i = Model::new_2d(&[], &[]);
    let j = Model::new_2d(&[], &[]);
    let k = Model::new_2d(&[], &[]);
    let l = Model::new_2d(&[], &[]);
    let m = Model::new_2d(&[], &[]);
    let n = Model::new_2d(&[], &[]);
    let o = Model::new_2d(&[], &[]);
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
