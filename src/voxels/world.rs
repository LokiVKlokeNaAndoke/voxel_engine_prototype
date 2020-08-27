use super::chunk::SChunk;
use super::chunk::{ChunkPosition, CHSIZE, CHSIZEF, CHSIZEI};
use super::terrain_generation::ProceduralGenerator;
use super::voxel::Voxel;
use crate::core::{to_vecf, ConcurrentHashMap, ConcurrentHashSet, Vec3f, Vec3i};
use amethyst::{core::components::Transform, derive::SystemDesc, ecs::prelude::*, prelude::*};
use flurry::epoch::Guard;
use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, RwLock, RwLockWriteGuard};

#[derive(Debug, Copy, Clone)]
pub struct VoxChange {
    pub new_vox: Voxel,
    pub index: [usize; 3],
}

impl VoxChange {
    pub fn new(index: [usize; 3], new_vox: Voxel) -> Self {
        Self { new_vox, index }
    }
}

#[derive(Default)]
pub struct VoxelWorld {
    chunks: ConcurrentHashMap<ChunkPosition, RwLock<SChunk>>,
    chunk_changes: ConcurrentHashMap<ChunkPosition, Mutex<VecDeque<VoxChange>>>,
    dirty: ConcurrentHashSet<ChunkPosition>,
    procedural: ProceduralGenerator,
}

impl VoxelWorld {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn chunk_at_or_create<'a>(
        &'a self,
        pos: &ChunkPosition,
        guard: &'a Guard,
    ) -> &'a RwLock<SChunk> {
        let chunk = match self.chunks.get(pos, guard) {
            Some(c) => c,
            None => {
                let mut c = SChunk::new();
                self.procedural.fill_random(&pos, &mut c.data_mut());
                self.chunks.try_insert(*pos, RwLock::new(c), guard).unwrap();
                self.chunks.get(pos, guard).unwrap()
            }
        };
        chunk
    }

    pub fn voxel_at_pos(&self, pos: &Vec3f, guard: &Guard) -> Voxel {
        let (ch, ind) = Self::to_ch_pos_index(pos);
        self.voxel_at(&ch, &ind, guard)
    }
    pub fn voxel_at(&self, chunk: &ChunkPosition, ind: &[usize; 3], guard: &Guard) -> Voxel {
        let chunk = self.chunk_at_or_create(chunk, guard).read().unwrap();
        chunk.data()[*ind]
    }

    pub fn set_voxel_at_pos(&self, pos: &Vec3f, new_vox: Voxel, guard: &Guard) {
        let (ch, ind) = Self::to_ch_pos_index(pos);
        self.set_voxel_at(&ch, &ind, new_vox, guard)
    }
    pub fn set_voxel_at(
        &self,
        chunk: &ChunkPosition,
        ind: &[usize; 3],
        new_vox: Voxel,
        guard: &Guard,
    ) {
        let ch_list = match self.chunk_changes.get(chunk, guard) {
            Some(change_list) => change_list,
            None => {
                self.chunk_changes
                    .try_insert(*chunk, Mutex::new(VecDeque::new()), guard)
                    .unwrap();
                self.chunk_changes.get(chunk, guard).unwrap()
            }
        };
        let mut ch_list = ch_list.try_lock().unwrap();
        ch_list.push_back(VoxChange::new(*ind, new_vox));
    }

    pub fn apply_voxel_changes(&self, guard: &Guard) {
        let mut borders_changed = HashSet::new();

        // TODO: when flurry supports rayon use parallel iterators
        self.chunk_changes.iter(guard).for_each(|(pos, list)| {
            let mut chunk = self.chunk_at_or_create(pos, guard).try_write().unwrap();
            let mut list = list.try_lock().unwrap();
            list.iter().for_each(|change| {
                chunk.data_mut()[change.index] = change.new_vox;

                // if on a border
                let border = SChunk::is_on_border(&change.index);
                if let Some(border_dir) = border {
                    borders_changed.insert((*pos, border_dir));
                }
            });
            list.clear()
        });

        for (chunk_pos, copy_to_dir) in borders_changed {
            let curr_chunk = self
                .chunk_at_or_create(&chunk_pos, guard)
                .try_read()
                .unwrap();

            let copy_to_vec = copy_to_dir.to_vec::<i32>();
            let next_chunk = chunk_pos.pos + copy_to_vec;
            let mut next_chunk = self
                .chunk_at_or_create(&ChunkPosition { pos: next_chunk }, guard)
                .try_write()
                .unwrap();

            next_chunk.copy_borders(&*curr_chunk, copy_to_dir.invert());
        }
    }

    pub fn to_ch_pos_index(pos: &Vec3f) -> (ChunkPosition, [usize; 3]) {
        let pos = pos / CHSIZEF;
        let ch_pos = Vec3i::new(
            pos.x.floor() as i32,
            pos.y.floor() as i32,
            pos.z.floor() as i32,
        );
        let index = pos - to_vecf(ch_pos * CHSIZEI);
        let index = [
            index.x.floor() as usize,
            index.y.floor() as usize,
            index.z.floor() as usize,
        ];

        (ChunkPosition { pos: ch_pos }, index)
    }
}
