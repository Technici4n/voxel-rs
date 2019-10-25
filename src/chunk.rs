const BLOC_IN_CHUNK: u32 = 16; // number of bloc of data in a chunk axis
const BLOC_SIZE: u32 = 2; // number of data in a bloc axis
pub const CHUNK_SIZE : u32 = BLOC_SIZE*BLOC_IN_CHUNK; // size of an axis of chunk (number of data)
const BLOCK_LEN : usize = (BLOC_SIZE*BLOC_SIZE*BLOC_SIZE) as usize; // number of data in a block

#[derive(Clone)]
enum BlockGroup {
     Compressed(u32), // 1 bit (NxNxN) times the same data
     Uncompressed(Box<[u32; BLOCK_LEN]>), // different datas
 }

#[derive(Clone)]
pub struct Chunk{
    pub px : i64, // position of the chunkc in the world
    pub py : i64,
    pub pz : i64,
    data : Vec<BlockGroup> // data containde in the chunk
}


impl Chunk {

    pub fn new(x : i64, y : i64, z : i64) -> Chunk{
        Chunk{
            px : x,
            py : y,
            pz : z,
            data :  vec![BlockGroup::Compressed(0); (BLOC_IN_CHUNK*BLOC_IN_CHUNK*BLOC_IN_CHUNK) as usize],
            // chunk is empty
        }

    }


    pub fn get_data(&self, px:u32, py:u32, pz:u32 ) -> u32{
        match &self.data[((px/BLOC_SIZE)*BLOC_IN_CHUNK*BLOC_IN_CHUNK
        + (py/BLOC_SIZE)*BLOC_IN_CHUNK+(pz/BLOC_SIZE)) as usize] {
            BlockGroup::Compressed(block_type) => *block_type, // if compressed return the compressed type
            BlockGroup::Uncompressed(blocks) => blocks[((px%BLOC_SIZE)*4 + (py%BLOC_SIZE)*2 + (pz%BLOC_SIZE)) as usize],
            // if not compressed, return the data stored in the full array
        }

    }

    pub fn set_data(&mut self, px:u32, py:u32, pz:u32, data: u32){
        let mut x = &mut self.data[((px/BLOC_SIZE)*BLOC_IN_CHUNK*BLOC_IN_CHUNK
        + (py/BLOC_SIZE)*BLOC_IN_CHUNK+(pz/BLOC_SIZE)) as usize];

         if let BlockGroup::Compressed(block_type) = x{
            if *block_type != data{ // splitting the group into an new array
                let mut fill = [*block_type; BLOCK_LEN];
                fill[((px%BLOC_SIZE)*BLOC_SIZE*BLOC_SIZE + (py%BLOC_SIZE)*BLOC_SIZE + (pz%BLOC_SIZE)) as usize] = data;
                *x = BlockGroup::Uncompressed(Box::new(fill));

            }
        }else if let BlockGroup::Uncompressed(blocks) = x{
            blocks[((px%BLOC_SIZE)*BLOC_SIZE*BLOC_SIZE + (py%BLOC_SIZE)*BLOC_SIZE + (pz%BLOC_SIZE)) as usize] = data;
            for i in 0..BLOCK_LEN{ // if all the data in the array are the same -> merge
                if blocks[i] != data{
                    return
                }
            }
            *x = BlockGroup::Compressed(data); // mergin all block in one
        }

    }

}
