use std::collections::HashMap;
use std::sync::Arc;
use voxel_rs_common::{
    block::BlockMesh,
    physics::BlockContainer,
    player::{CloseChunks, RenderDistance},
    world::{BlockPos, ChunkPos, Chunk, LightChunk},
};
use crate::render::WorldRenderer;
use crate::render::world::{ChunkMeshData, MeshingWorker, start_meshing_worker};

/// Client-side world.
/// It is currently responsible for:
/// * storing chunk data
/// * meshing and rendering the chunks
pub struct World {
    /// The chunks
    chunks: HashMap<ChunkPos, ClientChunk>,
    /// The meshing worker
    meshing_worker: MeshingWorker,
    /// The chunks the player can see
    close_chunks: CloseChunks,
    /// The renderer
    renderer: WorldRenderer,
}

impl World {
    /// Create a new empty world using the provided chunks
    pub fn new(block_meshes: Vec<BlockMesh>, renderer: WorldRenderer) -> Self {
        Self {
            chunks: HashMap::new(),
            meshing_worker: start_meshing_worker(block_meshes),
            close_chunks: CloseChunks::new(&RenderDistance::default()),
            renderer,
        }
    }

    /// Receive a new chunk from the server
    pub fn add_chunk(&mut self, chunk: Arc<Chunk>, light_chunk: Arc<LightChunk>) {
        // TODO: make sure this only happens once
        let chunk_pos = chunk.pos;
        self.chunks.insert(chunk_pos, ClientChunk {
            chunk,
            light_chunk,
            is_in_meshing_queue: false,
            needs_remesh: true,
        });
        // Queue adjacent chunks for meshing
        for i in -1..=1 {
            for j in -1..=1 {
                for k in -1..=1 {
                    let adjacent_chunk_pos = chunk_pos.offset(i, j, k);
                    if let Some(client_chunk) = self.chunks.get_mut(&adjacent_chunk_pos) {
                        client_chunk.needs_remesh = true;
                    }
                }
            }
        }
    }

    /// Fetch the new chunk meshes from the meshing worker
    pub fn get_new_chunk_meshes(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        while let Some(mesh) = self.meshing_worker.get_result() {
            if let Some(client_chunk) = self.chunks.get_mut(&mesh.0) {
                client_chunk.is_in_meshing_queue = false;
                self.renderer.update_chunk_mesh(device, encoder, mesh);
            }
        }
    }

    /// Remove chunks that are too far for the player
    pub fn remove_far_chunks(&mut self, player_chunk: ChunkPos, render_distance: &RenderDistance) {
        let Self { ref mut chunks, ref mut renderer, .. } = self;
        chunks.retain(|chunk_pos, _| {
            if render_distance.is_chunk_visible(player_chunk, *chunk_pos) {
                true
            } else {
                renderer.remove_chunk_mesh(*chunk_pos);
                false
            }
        })
    }

    /// Start the meshing of a few chunks
    pub fn enqueue_chunks_for_meshing(&mut self, player_chunk: ChunkPos, render_distance: &RenderDistance) {
        self.close_chunks.update(render_distance);
        for pos in self.close_chunks.get_close_chunks() {
            let pos = pos.offset_by_pos(player_chunk);
            if let Some(client_chunk) = self.chunks.get(&pos) {
                if client_chunk.needs_remesh && !client_chunk.is_in_meshing_queue {
                    let res = self.meshing_worker.enqueue(self.create_chunk_mesh_data(pos));
                    match res {
                        // If the meshing queue is not full, update chunk status
                        Ok(()) => {
                            let client_chunk = self.chunks.get_mut(&pos).expect("Logic error");
                            client_chunk.needs_remesh = false;
                            client_chunk.is_in_meshing_queue = true;
                        },
                        // If the meshing queue is full, stop
                        Err(_) => break,
                    }
                }
            }
        }
    }

    /// Create a `ChunkMeshData` for a loaded chunk
    fn create_chunk_mesh_data(&self, pos: ChunkPos) -> ChunkMeshData {
        let client_chunk = self.chunks.get(&pos).expect("no chunk at current position to create ChunkMeshData");
        let mut all_chunks: [Option<Arc<Chunk>>; 27] = Default::default();
        let mut all_light_chunks: [Option<Arc<LightChunk>>; 27] = Default::default();
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    let np = pos.offset(i - 1, j - 1, k - 1);
                    let idx = (i * 9 + j * 3 + k) as usize;
                    let adj_client_chunk = self.chunks.get(&np);
                    all_chunks[idx] = adj_client_chunk.map(|c| c.chunk.clone());
                    all_light_chunks[idx] = adj_client_chunk.map(|c| c.light_chunk.clone());
                }
            }
        }

        ChunkMeshData {
            chunk: client_chunk.chunk.clone(),
            light_chunk: client_chunk.light_chunk.clone(),
            all_chunks,
            all_light_chunks,
        }
    }

    /// Render the chunks
    pub fn render_chunks(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        buffers: crate::window::WindowBuffers,
        data: &crate::window::WindowData,
        frustum: &crate::render::Frustum,
        enable_culling: bool,
        pointed_block: Option<(BlockPos, usize)>,
        models: &[crate::render::world::Model],
    ) {
        // TODO: remove some of the parameters and calculate them here instead
        self.get_new_chunk_meshes(device, encoder);
        self.renderer.render(device, encoder, buffers, data, frustum, enable_culling, pointed_block, models);
    }

    /// Number of loaded chunks
    pub fn num_loaded_chunks(&self) -> usize {
        self.chunks.len()
    }
}

impl BlockContainer for World {
    fn is_block_full(&self, pos: BlockPos) -> bool {
        // TODO: use BlockRegistry
        match self.chunks.get(&pos.containing_chunk_pos()) {
            None => false,
            Some(chunk) => chunk.chunk.get_block_at(pos.pos_in_containing_chunk()) != 0,
        }
    }
}

/// The data for each chunk stored by the client
struct ClientChunk {
    /// The chunk itself
    pub chunk: Arc<Chunk>,
    /// The light chunk
    pub light_chunk: Arc<LightChunk>,
    /// True if the chunk is in the meshing queue
    pub is_in_meshing_queue: bool,
    /// True if the chunk needs to be meshed, for example before it never was meshed or because it changed.
    pub needs_remesh: bool,
}