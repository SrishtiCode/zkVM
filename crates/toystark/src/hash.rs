fn mix64(mut x: u64) -> u64{
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58476d1ce4e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d049bb133111eb);
    x ^= x >> 31;
    x
} 

pub fn hash_leaf(value: u64) -> u64{
    mix64(value ^ 0x9E37_79B9_7F4A_7C15)
}

pub fn hash_pair(left: u64, right: u64) -> u64{
    mix64(left.rotate_left(17) ^ mix64(right))
}

#[derive(PartialEq, Debug, Clone, Eq)]
pub struct MerklePath{
    pub leaf_index : usize,
    pub siblings : Vec<u64>,
}

impl MerklePath{
    pub fn verify(&self, root: u64, leaf_hash: u64) -> bool{
        let mut current = leaf_hash;
        let mut index = self.leaf_index;
        for &sibling in &self.siblings{
            current = if index % 2 == 0{
                hash_pair(current, sibling)
            }else{
                hash_pair(sibling, current)
            };
            index /= 2;
        }
        root == current
    }
}

#[derive(Debug, Clone)]

pub struct MerkleTree{
    layers: Vec<Vec<u64>>,
}

impl MerkleTree{

    pub fn build(leaf_hashes : &[u64]) -> Self{
        assert!(
            leaf_hashes.len().is_power_of_two() && !leaf_hashes.is_empty(),
            "Merkle tree needs a nonzero power-of-two number of leaves, got {}",
            leaf_hashes.len()
        );
        let mut layers = vec![leaf_hashes.to_vec()];
        while layers.last().unwrap().len() > 1{
            let prev = layers.last().unwrap();
            let next: Vec<u64> = prev.chunks(2).map(|pair| hash_pair(pair[0], pair[1])).collect();
            layers.push(next);
        }
        MerkleTree{layers}
    }

    pub fn from_values(values:&[u64]) -> Self {
        let leaves: Vec<u64> = values.iter().map(|&v| hash_leaf(v)).collect();
        Self::build(&leaves)  
    }    

    pub fn root(&self) -> u64{
        self.layers.last().unwrap()[0]
    }   

    pub fn num_leaves(&self) -> usize{
        self.layers[0].len()
    }    

    pub fn open(&self, index: usize) -> MerklePath{
        assert!(index < self.num_leaves(), "leaf index {index} out of range");
        let mut siblings = Vec::with_capacity(self.layers.len() - 1);
        let mut idx = index;
        for layer in &self.layers[..self.layers.len() - 1]{
            siblings.push(layer[idx ^ 1]);
            idx /= 2;
        }   
        MerklePath{leaf_index: index, siblings}
    }   

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_and_verify_every_leaf() {
        let values: Vec<u64> = (0..16).collect();
        let tree = MerkleTree::from_values(&values);
        for (i, &v) in values.iter().enumerate() {
            let path = tree.open(i);
            assert!(path.verify(tree.root(), hash_leaf(v)), "leaf {i} failed to verify");
        }
    }

    #[test]
    fn tampered_leaf_fails_verification() {
        let values: Vec<u64> = (0..8).collect();
        let tree = MerkleTree::from_values(&values);
        let path = tree.open(3);
        assert!(!path.verify(tree.root(), hash_leaf(999)));
    }

    #[test]
    fn tampered_sibling_fails_verification() {
        let values: Vec<u64> = (0..8).collect();
        let tree = MerkleTree::from_values(&values);
        let mut path = tree.open(2);
        path.siblings[0] ^= 1; 
        assert!(!path.verify(tree.root(), hash_leaf(2)));
    }

    #[test]
    fn single_leaf_tree_has_leaf_as_root() {
        let tree = MerkleTree::from_values(&[42]);
        assert_eq!(tree.root(), hash_leaf(42));
        let path = tree.open(0);
        assert!(path.siblings.is_empty());
        assert!(path.verify(tree.root(), hash_leaf(42)));
    }

    #[test]
    #[should_panic(expected = "power-of-two")]
    fn rejects_non_power_of_two_leaf_count() {
        let _ = MerkleTree::from_values(&[1, 2, 3]);
    }
}
