use crate::blockchain::{Block, BlockChain, BlockHeight, EpochNum, INITIAL_EPOCH};
use crate::crypto::{HashOf, Keypair, PublicKey, Signed};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};

pub type NodeID = usize;

#[derive(Debug)]
struct Message<T> {
    pub recipients: HashSet<NodeID>,
    pub vote: Signed<Block<T>>,
    pub voter: NodeID,
}

#[derive(Debug)]
struct Node<T> {
    epoch_num: EpochNum,
    node_id: NodeID,
    node_set_info: NodeSetInfo,
    chains: BTreeMap<BlockHeight, Vec<BlockChain<T>>>,
    keypair: Keypair,
    proposal_recieved: bool,
    votes: HashMap<HashOf<Block<T>>, (u64, HashSet<NodeID>)>,
}

#[derive(Debug, Clone)]
struct NodeSetInfo {
    node_pub_keys: Vec<PublicKey>,
}

impl NodeSetInfo {
    pub fn get_public_key(&self, id: NodeID) -> Option<&PublicKey> {
        self.node_pub_keys.get(id)
    }

    pub fn num_nodes(&self) -> usize {
        self.node_pub_keys.len()
    }
}

impl<T> Node<T>
where
    T: Serialize + Clone + PartialEq + Eq + std::fmt::Debug,
{
    /// Creates a new node initialized at the starting epoch number
    /// with a blockchain containing only the genesis block
    pub fn new(id: NodeID, keypair: Keypair, node_set_info: NodeSetInfo) -> Self {
        assert!(
            &keypair.public
                == node_set_info
                    .get_public_key(id)
                    .expect("There should be a key associated with this node id")
        );
        let initial_chain = BlockChain::new();
        let mut chains = BTreeMap::new();
        let votes: HashMap<HashOf<Block<T>>, (u64, HashSet<NodeID>)> = HashMap::new();
        chains.insert(initial_chain.block_height(), vec![initial_chain]);
        Node {
            epoch_num: INITIAL_EPOCH,
            node_id: id,
            node_set_info,
            chains: chains,
            keypair,
            votes,
            proposal_recieved: false,
        }
    }
    /// Returns true if the Node is the leader of the current epoch, and false otherwise.
    pub fn is_leader(&self) -> bool {
        self.get_leader() == self.node_id
    }

    /// Advances the Node to the next epoch, returning the new epoch number.
    pub fn advance_epoch(&mut self) -> EpochNum {
        dbg!(&self.chains);
        self.epoch_num += 1;
        self.epoch_num
    }

    /// Propose a new block containing the provided payload
    pub fn propose(&self, payload: T) -> Message<T> {
        let epoch = self.epoch_num;
        let longest_chain = self.peek_longest_chain();
        let prev_hash = longest_chain.get_latest_block_hash();
        let block = Block::new(payload, prev_hash, epoch);
        let notarized_block = Signed::new(block, &self.keypair);

        let msg = Message {
            recipients: self.every_node_except_me(),
            vote: notarized_block,
            voter: self.node_id,
        };

        msg
    }

    /// Handle an incoming messages, checking for valid votes and proposals
    pub fn handle_message(&mut self, msg: &Message<T>) -> Vec<Message<T>> {
        let mut messages = vec![];
        let n = self.node_set_info.num_nodes();
        // First, validate the vote
        if self.validate_vote(msg.voter, &msg.vote) {
            // TODO simply increment vote counter if we've seen this block
            // TODO maybe networking layer should be responsible for not sending
            // duplicate messages
            let new_block = msg.vote.get_data();
            let hash = new_block.hash();
            let votes = self.votes.entry(hash).or_insert((0, HashSet::new()));
            if (*votes).1.insert(msg.voter) {
                votes.0 += 1;

                if (votes.0 as usize) > (2 * n) / 3 + 1 {
                    self.try_append_block(new_block)
                }

                let echo_message = Message {
                    recipients: self.every_node_except_me(),
                    vote: msg.vote.clone(),
                    voter: msg.voter,
                };

                messages.push(echo_message);

                if self.message_is_valid_proposal(msg) {
                    // If valid proposal is recieved, vote for it and forward along
                    self.proposal_recieved = true;

                    let vote_message = Message {
                        recipients: self.every_node_except_me(),
                        vote: Signed::new(new_block.clone(), &self.keypair),
                        voter: self.node_id,
                    };

                    messages.push(vote_message);
                }
            }
        }
        messages
    }

    /// A message is a valid proposal iff:
    /// 1) The epoch number of the block matches the current epoch
    /// 2) The signer of the block is the leader of the current epoch
    /// 3) The signature is valid
    // TODO check signature here too?
    fn message_is_valid_proposal(&self, msg: &Message<T>) -> bool {
        let block = msg.vote.get_data();
        let valid_vote = self.validate_vote(msg.voter, &msg.vote);
        return block.epoch == self.epoch_num && msg.voter == self.get_leader() && valid_vote;
    }

    fn get_leader(&self) -> NodeID {
        return 0;
    }

    fn try_append_block(&mut self, new_block: &Block<T>) {
        // TODO remove unwraps
        let hash = new_block.prev_hash.as_ref().unwrap();
        // Try to add to one of our longest chains
        let mut blockchain_entry = self.chains.last_entry().unwrap();
        let blockchains = blockchain_entry.get_mut();
        let mut idx = None;
        for (pos, chain) in blockchains.iter().enumerate() {
            let chain_hash = chain.get_latest_block_hash();
            if (&chain_hash == hash) {
                idx = Some(pos);
            }
        }

        if let Some(idx) = idx {
            let mut chain = blockchains.remove(idx);
            chain.add_block(new_block);
            self.chains.insert(chain.block_height(), vec![chain]);
        }
    }

    fn peek_longest_chain(&self) -> BlockChain<T> {
        let blockchains = self.chains.last_key_value().unwrap();
        // Just pick the first one, maybe random is better?
        (blockchains.1)[0].clone()
    }

    fn every_node_except_me(&self) -> HashSet<NodeID> {
        (0..self.node_id)
            .chain((self.node_id + 1..self.node_set_info.num_nodes()))
            .collect()
    }

    // TODO make err
    fn validate_vote(&self, voter: NodeID, vote: &Signed<Block<T>>) -> bool {
        let pk = self.node_set_info.get_public_key(voter).unwrap();
        vote.verify(&pk).is_ok()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::{CryptoRng, Rng};

    fn simulate_protocol<R: Rng + CryptoRng>(num_nodes: usize, rounds: usize, rand: &mut R) {
        let mut keypairs: Vec<Keypair> = (0..num_nodes).map(|_| Keypair::generate(rand)).collect();
        let node_pub_keys: Vec<PublicKey> = keypairs.iter().map(|kp| kp.public).collect();
        let node_info = NodeSetInfo { node_pub_keys };
        // TODO enumerate
        let mut nodes: Vec<Node<u64>> = (0..num_nodes)
            .zip(keypairs.drain(..))
            .map(|(id, key)| Node::new(id, key, node_info.clone()))
            .collect();

        let mut network_messages = Vec::new();
        for _ in (0..rounds) {
            for node in nodes.iter() {
                if (node.is_leader()) {
                    network_messages.push(node.propose(5));
                }
            }
            while !network_messages.is_empty() {
                let msg = network_messages.pop().unwrap();
                for recipient in msg.recipients.clone() {
                    let new_messages = nodes[recipient].handle_message(&msg);
                    network_messages.extend(new_messages);
                }
            }
            for node in nodes.iter_mut() {
                node.advance_epoch();
            }
        }
    }

    #[test]
    fn simulate() {
        let mut rng = rand::thread_rng();
        let n_nodes = 5;
        let n_rounds = 10;
        simulate_protocol(n_nodes, n_rounds, &mut rng);
    }
}
