use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use voxel_rs_common::{
    block::{Block, BlockId},
    player::RenderDistance,
    physics::BlockContainer,
    registry::Registry,
    world::{
        Chunk, ChunkPos, ChunkPosXZ,
        BlockPos,
        LightChunk,
        WorldGenerator,
    },
};
use crate::{
    light::HighestOpaqueBlock,
    light::worker::{ChunkLightingData, ChunkLightingWorker, start_lighting_worker},
    worldgen::{WorldGenerationWorker, start_worldgen_worker},
};
use lazy_static::lazy_static;

lazy_static! {
    static ref EMPTY_HOB: Arc<HighestOpaqueBlock> = {
        Arc::new(HighestOpaqueBlock::new())
    };
}

/// Server-side world
/// It is responsible for
/// * storing chunk data
/// * generating the chunks
/// * updating the lighting
pub struct World {
    /// The chunks
    chunks: HashMap<ChunkPos, ServerChunk>,
    /// The chunk columns
    chunk_columns: HashMap<ChunkPosXZ, ServerChunkColumn>,
    /// The next chunk version. When the chunk version changes, we know we must send the updated chunk to the clients.
    next_chunk_version: u64,
    /// The chunks in the worldgen queue
    worldgen_queue: HashSet<ChunkPos>,
    /// The worldgen worker
    worldgen_worker: WorldGenerationWorker,
    /// The light worker
    light_worker: ChunkLightingWorker,
}

impl World {
    pub fn new(
        block_registry: Registry<Block>,
        world_generator: Box<dyn WorldGenerator + Send>
    ) -> Self {
        Self {
            chunks: HashMap::default(),
            chunk_columns: HashMap::default(),
            next_chunk_version: 0,
            worldgen_queue: HashSet::default(),
            worldgen_worker: start_worldgen_worker(block_registry, world_generator),
            light_worker: start_lighting_worker(),
        }
    }

    /// Return some chunk if is loaded
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<Arc<Chunk>> {
        self.chunks.get(&pos).map(|server_chunk| server_chunk.chunk.clone())
    }

    /// Return block at position `pos` in the world. 0 is returned if the chunk does not exists/is not loaded
    pub fn get_block(&self, pos: BlockPos) -> BlockId {
        match self.chunks.get(&pos.containing_chunk_pos()) {
            None => 0,
            Some(server_chunk) => server_chunk.chunk.get_block_at(pos.pos_in_containing_chunk()),
        }
    }

    /// Update the highest opaque block in the column, and mark relevant chunks for a light update.
    /// To be called after every chunk loading or modification.
    fn update_chunk_column(&mut self, pos: ChunkPos) {
        let column_pos = pos.into();

        // Update chunk HOB
        let hob = HighestOpaqueBlock::from_chunk(&self.chunks.get(&pos).unwrap().chunk);
        let column = self.chunk_columns.get_mut(&column_pos).unwrap();
        column.highest_opaque_blocks.insert(pos.py, hob);

        // TODO: don't update entire column, try to be more clever

        // Update column HOB
        let mut column_hob = HighestOpaqueBlock::new();
        for (_, chunk_hob) in column.highest_opaque_blocks.iter() {
            column_hob.merge(chunk_hob);
        }
        column.highest_opaque_block = Arc::new(column_hob);
        
        for i in -1..=1 {
            for k in -1..=1 {
                self.update_column_light(column_pos.offset(i, k));
            }
        }
    }

    /// Mark an entire chunk column for light updates
    fn update_column_light(&mut self, pos: ChunkPosXZ) {
        if let Some(chunk_column) = self.chunk_columns.get(&pos) {
            for chunk_pos in chunk_column.loaded_chunks.iter() {
                let server_chunk = self.chunks.get_mut(chunk_pos).expect("Column loaded chunk is not loaded in the world");
                server_chunk.needs_light_update = true;
            }
        }
    }

    /// Set the chunk at some position
    pub fn set_chunk(&mut self, chunk: Arc<Chunk>) {
        let pos = chunk.pos;
        let server_chunk = self.chunks.entry(pos).or_insert_with(|| {
            ServerChunk { 
                chunk: chunk.clone(),
                light_chunk: Arc::new(LightChunk::new(pos)),
                version: 0,
                is_in_light_queue: false,
                needs_light_update: true,
            }
        });
        server_chunk.chunk = chunk;
        server_chunk.needs_light_update = true;
        server_chunk.version = self.next_chunk_version;
        self.next_chunk_version += 1;

        let chunk_column = self.chunk_columns.entry(pos.into()).or_insert_with(|| {
            ServerChunkColumn {
                highest_opaque_block: Arc::new(HighestOpaqueBlock::new()),
                highest_opaque_blocks: HashMap::new(),
                loaded_chunks: HashSet::new(),
            }
        });
        chunk_column.loaded_chunks.insert(pos);
        // highest_opaque_block and highest_opaque_blocks will be updated in update_chunk_col

        self.update_chunk_column(pos);
    }

    /// Fetch the new chunk meshes from the worldgen worker
    pub fn get_new_generated_chunks(&mut self) {
        // TODO: maybe don't update all the light column every time
        // TODO: if there are multiple chunks in the same column this may save time
        while let Some(chunk) = self.worldgen_worker.get_result() {
            self.worldgen_queue.remove(&chunk.pos);
            self.set_chunk(Arc::new(chunk));
        }
    }

    /// Fetch the new light chunks from the light worker
    pub fn get_new_light_chunks(&mut self) {
        while let Some(light_chunk) = self.light_worker.get_result() {
            if let Some(mut server_chunk) = self.chunks.get_mut(&light_chunk.pos) {
                server_chunk.light_chunk = light_chunk;
                server_chunk.is_in_light_queue = false;
                server_chunk.version = self.next_chunk_version;
                self.next_chunk_version += 1;
            }
        }
    }

    /// Start the lighting of a few chunks
    pub fn enqueue_chunks_for_lighting(&mut self, player_close_chunks: &[ChunkPos]) {
        for pos in player_close_chunks {
            if let Some(server_chunk) = self.chunks.get(&pos) {
                if server_chunk.needs_light_update && !server_chunk.is_in_light_queue {
                    let res = self.light_worker.enqueue(self.create_chunk_lighting_data(*pos));
                    match res {
                        // If the lighting queue is not full, update chunk status
                        Ok(()) => {
                            let server_chunk = self.chunks.get_mut(&pos).expect("Logic error");
                            server_chunk.needs_light_update = false;
                            server_chunk.is_in_light_queue = true;
                        },
                        // If the lighting queue is full, stop
                        Err(_) => break,
                    }
                }
            }
        }
    }

    /// Create a `ChunkLightingData` for a loaded chunk
    fn create_chunk_lighting_data(&self, pos: ChunkPos) -> ChunkLightingData {
        let mut chunks = Vec::with_capacity(27);
        let mut highest_opaque_blocks = Vec::with_capacity(9);

        for i in -1..=1 {
            for k in -1..=1 {
                let pos: ChunkPosXZ = pos.offset(i, 0, k).into();
                highest_opaque_blocks.push(
                    (*self.chunk_columns
                        .get(&pos)
                        .map(|server_chunk| &server_chunk.highest_opaque_block)
                        .unwrap_or_else(|| &*EMPTY_HOB))
                        .clone()
                );
            }
        }

        for i in -1..=1 {
            for j in -1..=1 {
                for k in -1..=1 {
                    let pos = pos.offset(i, j, k);
                    chunks.push(self.get_chunk(pos));
                }
            }
        }

        ChunkLightingData { chunks, highest_opaque_blocks }
    }

    /// Start the worldgen of a few chunks
    pub fn enqueue_chunks_for_worldgen(&mut self, player_close_chunks: &[ChunkPos]) {
        for pos in player_close_chunks {
            if !self.chunks.contains_key(pos) && !self.worldgen_queue.contains(pos) {
                let res = self.worldgen_worker.enqueue(*pos);
                match res {
                    // If the worldgen queue is not full, update chunk status
                    Ok(()) => {
                        self.worldgen_queue.insert(*pos);
                    },
                    // If the worldgen queue is full, stop
                    Err(_) => break,
                }
            }
        }
    }

    /// Drop far chunks
    pub fn drop_far_chunks(&mut self, player_positions: &[(ChunkPos, RenderDistance)]) {
        let loaded_chunks = self.chunks.keys().cloned().collect::<Vec<_>>();
        'chunks: for chunk_pos in loaded_chunks {
            for (player_chunk, render_distance) in player_positions {
                if render_distance.is_chunk_visible(*player_chunk, chunk_pos) {
                    continue 'chunks
                }
            }
            self.unload_chunk(chunk_pos);
        }
    }

    /// Unload chunk
    // TODO: persist to disk
    fn unload_chunk(&mut self, pos: ChunkPos) {
        self.chunks.remove(&pos);
        let column_pos = ChunkPosXZ::from(pos);
        let col = self.chunk_columns.get_mut(&column_pos).expect("No chunk column");
        col.loaded_chunks.remove(&pos);
        col.highest_opaque_blocks.remove(&pos.py);
        if col.loaded_chunks.len() == 0 {
            self.chunk_columns.remove(&column_pos);
        }
    }

    /// Get chunks to send to a player this frame, and update the `PlayerData` accordingly. Start generating some chunks if necessary
    pub fn send_chunks_to_player(&mut self, player_chunk: ChunkPos, data: &mut super::PlayerData) -> Vec<(Arc<Chunk>, Arc<LightChunk>)>{
        const MAX_CHUNKS: usize = 20;
        let mut updates = Vec::new();
        for pos in data.close_chunks.get_close_chunks() {
            let pos = pos.offset_by_pos(player_chunk);
            if let Some(server_chunk) = self.chunks.get(&pos) {
                // Send the chunk to the player
                let loaded = data.loaded_chunks.insert(pos, server_chunk.version);
                if let Some(old_client_version) = loaded {
                    if old_client_version < server_chunk.version {
                        updates.push((server_chunk.chunk.clone(), server_chunk.light_chunk.clone()));
                    }
                } else {
                    updates.push((server_chunk.chunk.clone(), server_chunk.light_chunk.clone()));
                }
                if updates.len() == MAX_CHUNKS {
                    break
                }
            } else {
                // Generate the chunk
                let res = self.worldgen_worker.enqueue(pos);
                if res.is_ok() {
                    self.worldgen_queue.insert(pos);
                }
            }
        }
        updates
    }

    /// Number of loaded chunks
    pub fn num_loaded_chunks(&self) -> usize {
        self.chunks.len()
    }

    /// Number of loaded chunk columns
    pub fn num_loaded_chunk_columns(&self) -> usize {
        self.chunk_columns.len()
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

/// The data for each chunk stored by the server
struct ServerChunk {
    /// The chunk itself
    pub chunk: Arc<Chunk>,
    /// The light chunk
    pub light_chunk: Arc<LightChunk>,
    /// The current chunk version
    pub version: u64,
    /// True if the chunk is in the light queue
    pub is_in_light_queue: bool,
    /// True if the chunk needs a light update, for example before it never had one or because it changed.
    pub needs_light_update: bool,
}

/// The data for each chunk column stored by the server
struct ServerChunkColumn {
    /// The highest opaque block in the column
    pub highest_opaque_block: Arc<HighestOpaqueBlock>,
    /// The highest opaque block in each chunk in the column
    pub highest_opaque_blocks: HashMap<i64, HighestOpaqueBlock>,
    /// The loaded chunks from this column
    pub loaded_chunks: HashSet<ChunkPos>,
}