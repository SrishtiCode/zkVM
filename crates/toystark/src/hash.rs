fn mix64(mut x: u64) -> u64{
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58476d1ce4e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d049bb133111eb);
    x ^= x >> 31;
} 

pub fn hash_leaf(value: u64) -> u64{
    mix64(value ^ 0x9E37_79B9_7F4A_7C15)
}

pub fn hash_pair(left: u64, right;u64) -> u64{
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
            "Not correct number of hashes"
        );
        let mut layers = vec![leaf_hashes.to_vec()];
        while layers.last().unwrap().len() > 1{
            let prev = layers.last().unwrap();
            let next: Vec<u64> = prev.chunks(2).map(|pair| hash_pair(pair[0], pair[1])).collect();
            layers.push(next);
        }
        MerkleTree{layers}
    }


}