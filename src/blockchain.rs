use crate::crypto::HashOf;
use serde::{Deserialize, Serialize};

pub type EpochNum = u64;
pub type BlockHeight = usize;
pub const INITIAL_EPOCH: u64 = 0;

#[derive(Hash, Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Block<T> {
    pub payload: Option<T>,
    pub prev_hash: Option<HashOf<Block<T>>>,
    pub epoch: EpochNum,
}

impl<T> Default for Block<T>
where
    T: Serialize + Clone + PartialEq + Eq + std::fmt::Debug,
{
    fn default() -> Self {
        Self::genesis_block()
    }
}

impl<T> Block<T>
where
    T: Serialize + Clone + PartialEq + Eq + std::fmt::Debug,
{
    pub fn new(payload: T, prev_hash: HashOf<Block<T>>, epoch: EpochNum) -> Self {
        Block {
            payload: Some(payload),
            prev_hash: Some(prev_hash),
            epoch,
        }
    }

    /// Genesis block with no payload, no previous hash, and an epoch number of 0
    pub fn genesis_block() -> Self {
        Block {
            payload: <Option<T>>::None,
            prev_hash: <Option<HashOf<Block<T>>>>::None,
            epoch: INITIAL_EPOCH,
        }
    }

    pub fn hash(&self) -> HashOf<Block<T>> {
        HashOf::new(&self)
    }
}

#[derive(Clone, Debug)]
pub struct BlockChain<T> {
    blocks: Vec<Block<T>>,
}

impl<T> BlockChain<T>
where
    T: Serialize + Clone + PartialEq + Eq + std::fmt::Debug,
{
    pub fn new() -> Self {
        let mut blocks = Vec::new();
        blocks.push(Block::genesis_block());
        BlockChain { blocks }
    }

    pub fn get_latest_block_hash(&self) -> HashOf<Block<T>> {
        let height = self.blocks.len();
        HashOf::new(&self.blocks[height - 1])
    }

    pub fn block_height(&self) -> BlockHeight {
        self.blocks.len()
    }

    pub fn add_block(&mut self, block: &Block<T>) {
        // TODO error handling
        if block.prev_hash.as_ref().unwrap() != &self.get_latest_block_hash() {
            panic!();
        }
        self.blocks.push(block.clone());
    }
}
