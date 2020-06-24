use crate::world::ChunkPos;
use serde::Deserialize;

/// The input of a player
#[derive(Debug, Clone, Copy)]
pub struct PlayerInput {
    pub key_move_forward: bool,
    pub key_move_left: bool,
    pub key_move_backward: bool,
    pub key_move_right: bool,
    pub key_move_up: bool,
    pub key_move_down: bool,
    pub yaw: f64,
    pub pitch: f64,
    pub flying: bool,
}

impl Default for PlayerInput {
    fn default() -> Self {
        Self {
            key_move_forward: false,
            key_move_left: false,
            key_move_backward: false,
            key_move_right: false,
            key_move_up: false,
            key_move_down: false,
            yaw: 0.0,
            pitch: 0.0,
            flying: true,
        }
    }
}

/// Some unique player id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(pub(crate) u16);

/// The render distance of a player
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub struct RenderDistance {
    pub x_max: u64,
    pub x_min: u64,
    pub y_max: u64,
    pub y_min: u64,
    pub z_max: u64,
    pub z_min: u64,
}

impl RenderDistance {
    /// Create an iterator over the chunks in the render distance around the player pos
    pub fn iterate_around_player(self, player_chunk: ChunkPos) -> impl Iterator<Item = ChunkPos> {
        RenderDistanceIterator::new(self, player_chunk)
    }

    /// Check whether a chunk is in render distance of the player
    pub fn is_chunk_visible(self, player_chunk: ChunkPos, chunk_pos: ChunkPos) -> bool {
        chunk_pos.px - player_chunk.px <= self.x_max as i64
            && chunk_pos.py - player_chunk.py <= self.y_max as i64
            && chunk_pos.pz - player_chunk.pz <= self.z_max as i64
            && player_chunk.px - chunk_pos.px <= self.x_min as i64
            && player_chunk.py - chunk_pos.py <= self.y_min as i64
            && player_chunk.pz - chunk_pos.pz <= self.z_min as i64
    }
}

pub struct RenderDistanceIterator {
    i: i64,
    j: i64,
    k: i64,
    render_distance: RenderDistance,
    player_chunk: ChunkPos,
}

impl RenderDistanceIterator {
    pub(self) fn new(render_distance: RenderDistance, player_chunk: ChunkPos) -> Self {
        Self {
            i: -(render_distance.x_min as i64),
            j: -(render_distance.y_min as i64),
            k: -(render_distance.z_min as i64),
            render_distance,
            player_chunk,
        }
    }
}

impl Iterator for RenderDistanceIterator {
    type Item = ChunkPos;

    fn next(&mut self) -> Option<Self::Item> {
        if self.k > self.render_distance.z_max as i64 {
            self.k = -(self.render_distance.z_min as i64);
            self.j += 1;
        }
        if self.j > self.render_distance.y_max as i64 {
            self.j = -(self.render_distance.y_min as i64);
            self.i += 1;
        }
        if self.i > self.render_distance.x_max as i64 {
            None
        } else {
            self.k += 1;
            Some(
                (
                    self.i + self.player_chunk.px,
                    self.j + self.player_chunk.py,
                    self.k + self.player_chunk.pz - 1,
                )
                    .into(),
            )
        }
    }
}

impl Default for RenderDistance {
    fn default() -> Self {
        Self {
            x_max: 1,
            x_min: 1,
            y_max: 1,
            y_min: 1,
            z_max: 1,
            z_min: 1,
        }
    }
}

/// All the visible chunks for a given `RenderDistance` sorted by distance
pub struct CloseChunks {
    /// The chunks
    close_chunks: Vec<ChunkPos>,
    /// The `RenderDistance` for which the chunks are valid
    render_distance: RenderDistance,
}

impl CloseChunks {
    pub fn new(render_distance: &RenderDistance) -> Self {
        Self {
            close_chunks: get_close_chunks(render_distance),
            render_distance: *render_distance,
        }
    }

    pub fn update(&mut self, render_distance: &RenderDistance) {
        if *render_distance != self.render_distance {
            self.close_chunks = get_close_chunks(render_distance);
            self.render_distance = *render_distance;
        }
    }

    pub fn get_close_chunks(&self) -> &Vec<ChunkPos> {
        &self.close_chunks
    }
}

fn get_close_chunks(render_distance: &RenderDistance) -> Vec<ChunkPos> {
    let origin = ChunkPos::from([0, 0, 0]);
    let mut adjacent_positions: Vec<_> = render_distance.iterate_around_player(origin).collect();
    adjacent_positions.sort_by_key(|pos| origin.squared_euclidian_distance(*pos));
    adjacent_positions
}
