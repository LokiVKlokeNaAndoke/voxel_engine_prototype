use super::Voxel;
use crate::core::{to_uarr, to_vecf, Vec3f, Vec3i};
use crate::directions::Directions;
use crate::voxels::chunk_mesh::ChunkMeshData;
use amethyst::ecs::prelude::*;
use amethyst::renderer::rendy::mesh::MeshBuilder;
use bitflags::_core::cmp::Ordering;
use ndarray::prelude::*;
use ndarray::Zip;
use serde::{Deserialize, Serialize};
use std::convert::identity;

pub const CHSIZE: usize = 8;
pub const CHSIZEI: i32 = CHSIZE as i32;
pub const CHSIZEF: f32 = CHSIZE as f32;

pub type SChunk = Chunk<CHSIZE>;

#[derive(Debug)]
pub struct Chunk<const N: usize> {
    data: Array3<Voxel>,
}

impl<const N: usize> Chunk<N> {
    const NI: i32 = N as i32;

    pub fn new() -> Self {
        Chunk {
            data: Array3::default([N + 2, N + 2, N + 2]),
        }
    }

    pub fn data_mut(&mut self) -> ArrayViewMut3<Voxel> {
        self.data.slice_mut(s![1..-1, 1..-1, 1..-1])
    }
    pub fn data(&self) -> ArrayView3<Voxel> {
        self.data.slice(s![1..-1, 1..-1, 1..-1])
    }

    fn copy_face_up(
        data: &mut Array3<Voxel>,
        index_transform: impl Fn((i32, i32, i32)) -> (i32, i32, i32),
    ) {
        let one: Vec3i = [1, 1, 1].into();
        for x in 0..(Self::NI) {
            for z in 0..(Self::NI) {
                let (x, y, z) = index_transform((x, Self::NI - 1, z));
                let index: Vec3i = [x, y, z].into();
                data[to_uarr(index + one)];
            }
        }
    }

    pub fn copy_borders(&mut self, other: &Self, dir: Directions) {
        let dir: Directions = dir.to_vec::<i32>().into();
        match dir {
            x if x == Directions::NORTH => {
                Self::copy_face_up(&mut self.data, |p| {
                    Self::rotate90_yz(Self::rotate90_yz(Self::rotate90_yz(p)))
                });
            }
            x if x == Directions::SOUTH => {
                Self::copy_face_up(&mut self.data, |p| Self::rotate90_yz(Self::rotate90_yz(p)));
            }
            x if x == Directions::WEST => {
                Self::copy_face_up(&mut self.data, |p| Self::rotate90_xy(p));
            }
            x if x == Directions::EAST => {
                Self::copy_face_up(&mut self.data, |p| {
                    Self::rotate90_xy(Self::rotate90_xy(Self::rotate90_xy(p)))
                });
            }
            x if x == Directions::UP => {
                Self::copy_face_up(&mut self.data, |p| identity(p));
            }
            x if x == Directions::DOWN => {
                Self::copy_face_up(&mut self.data, |p| Self::rotate90_xy(Self::rotate90_xy(p)));
            }
            x if x == (Directions::UP | Directions::EAST) => {
                Self::copy_face_up(&mut self.data, |p| identity(p));
            }
            _ => todo!("add all 26 combinations of directions"),
        }
    }

    pub fn mesh(&self) -> ChunkMeshData {
        let one: Vec3i = [1, 1, 1].into();
        let onef: Vec3f = [1., 1., 1.].into();

        let mut chunk_mesh = ChunkMeshData::new();
        for x in 0..Self::NI {
            for y in 0..Self::NI {
                for z in 0..Self::NI {
                    let pos: Vec3i = [x, y, z].into();
                    if self.data[to_uarr(pos + one)].is_transparent() {
                        // if current voxel is transparent
                        continue;
                    }
                    // if current voxel is solid
                    for dir in Directions::all().into_iter() {
                        let spos: Vec3i = pos + dir.to_vec::<i32>();
                        if self.data[to_uarr(spos + one)].is_transparent() {
                            // if adjacent voxel is transparent
                            chunk_mesh.insert_quad(to_vecf(pos) + onef / 2., dir);
                        }
                    }
                }
            }
        }

        chunk_mesh
    }

    fn transpose_xy((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
        (y, x, z)
    }
    fn transpose_xz((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
        (z, y, x)
    }
    fn transpose_yz((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
        (x, z, y)
    }
    fn reverse_x((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
        (Self::NI - x - 1, y, z)
    }
    fn reverse_y((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
        (x, Self::NI - y - 1, z)
    }
    pub fn rotate90_xy((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
        Self::reverse_x(Self::transpose_xy((x, y, z)))
    }
    pub fn rotate90_xz((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
        Self::reverse_x(Self::transpose_xz((x, y, z)))
    }
    pub fn rotate90_yz((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
        Self::reverse_y(Self::transpose_yz((x, y, z)))
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub struct ChunkPosition {
    pub pos: Vec3i,
}

impl ChunkPosition {
    pub fn new(pos: Vec3i) -> Self {
        ChunkPosition { pos }
    }
}

impl From<Vec3i> for ChunkPosition {
    fn from(value: Vec3i) -> Self {
        ChunkPosition::new(value)
    }
}

impl Default for ChunkPosition {
    fn default() -> Self {
        Self {
            pos: Vec3i::zeros(),
        }
    }
}

impl Ord for ChunkPosition {
    fn cmp(&self, other: &Self) -> Ordering {
        self.pos
            .x
            .cmp(&other.pos.x)
            .then(self.pos.y.cmp(&other.pos.y))
            .then(self.pos.z.cmp(&other.pos.z))
    }
}

impl PartialOrd for ChunkPosition {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Component for ChunkPosition {
    type Storage = DenseVecStorage<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALLCH: usize = 3;
    const SMALLCHI: i32 = SMALLCH as i32;
    type SmallChunk = Chunk<SMALLCH>;

    #[test]
    fn chunk_data_dimensions() {
        let mut chunk = SmallChunk::new();

        let data = chunk.data_mut();

        assert_eq!(data.shape(), &[SMALLCH, SMALLCH, SMALLCH]);
    }

    fn check(
        expected: &Array3<i32>,
        control: &Array3<i32>,
        fn_view: impl Fn((i32, i32, i32)) -> (i32, i32, i32),
    ) {
        let mut actual = Array3::default([SMALLCH, SMALLCH, SMALLCH]);
        Zip::indexed(&mut actual).apply(|(x, y, z), actual| {
            let (x, y, z) = fn_view((x as i32, y as i32, z as i32));
            *actual = control[[x as usize, y as usize, z as usize]];
        });
        assert_eq!(expected, &dbg!(actual));
    }

    /// expected results were checked by hand with a python visualization
    #[test]
    fn rotate_xy() {
        let control = array![
            [[1, 10, 19], [2, 11, 20], [3, 12, 21]],
            [[4, 13, 22], [5, 14, 23], [6, 15, 24]],
            [[7, 16, 25], [8, 17, 26], [9, 18, 27]]
        ];
        let expected = array![
            [[7, 16, 25], [4, 13, 22], [1, 10, 19]],
            [[8, 17, 26], [5, 14, 23], [2, 11, 20]],
            [[9, 18, 27], [6, 15, 24], [3, 12, 21]]
        ];

        check(&expected, &control, |p| SmallChunk::rotate90_xy(p));
    }

    /// expected results were checked by hand with a python visualization
    #[test]
    fn rotate_xz() {
        let control = array![
            [[1, 10, 19], [2, 11, 20], [3, 12, 21]],
            [[4, 13, 22], [5, 14, 23], [6, 15, 24]],
            [[7, 16, 25], [8, 17, 26], [9, 18, 27]]
        ];
        let expected = array![
            [[7, 4, 1], [8, 5, 2], [9, 6, 3]],
            [[16, 13, 10], [17, 14, 11], [18, 15, 12]],
            [[25, 22, 19], [26, 23, 20], [27, 24, 21]]
        ];

        check(&expected, &control, |p| SmallChunk::rotate90_xz(p));
    }

    /// expected results were checked by hand with a python visualization
    #[test]
    fn rotate_yz() {
        let control = array![
            [[1, 10, 19], [2, 11, 20], [3, 12, 21]],
            [[4, 13, 22], [5, 14, 23], [6, 15, 24]],
            [[7, 16, 25], [8, 17, 26], [9, 18, 27]]
        ];
        let expected = array![
            [[3, 2, 1], [12, 11, 10], [21, 20, 19]],
            [[6, 5, 4], [15, 14, 13], [24, 23, 22]],
            [[9, 8, 7], [18, 17, 16], [27, 26, 25]]
        ];

        check(&expected, &control, |p| SmallChunk::rotate90_yz(p));
    }

    #[test]
    fn copy_up() {
        let up = array![
            [[1, 10, 19], [2, 11, 20], [3, 12, 21]],
            [[4, 13, 22], [5, 14, 23], [6, 15, 24]],
            [[7, 16, 25], [8, 17, 26], [9, 18, 27]]
        ]
        .map(|v| Voxel::from(*v as u16));
        let mut upch = SmallChunk::new();
        for x in 0..SMALLCH {
            for y in 0..SMALLCH {
                for z in 0..SMALLCH {
                    upch.data_mut()[(x, y, z)] = up[(x, y, z)];
                }
            }
        }

        let mut this = SmallChunk::new();

        this.copy_borders(&upch, Directions::UP);

        let up_data = upch.data.map(|v| v.id);
        let up_slice = up_data.index_axis(Axis(1), 1);
        let this_data = this.data.map(|v| v.id);
        let this_slice = this_data.index_axis(Axis(1), 4);

        assert_eq!(up_slice, this_slice);
    }
}
