use crate::poseidon::Poseidon;
use field::Field;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerklePath<F: Field> {
    pub leaf_index: usize,
    pub siblings: Vec<F>,
}

impl<F: Field> MerklePath<F> {
    pub fn verify(&self, hasher: &Poseidon<F>, root: F, leaf_hash: F) -> bool {
        let mut current = leaf_hash;
        let mut index = self.leaf_index;
        for &sibling in &self.siblings {
            current = if index % 2 == 0 {
                hasher.hash_two(current, sibling)
            } else {
                hasher.hash_two(sibling, current)
            };
            index /= 2;
        }
        current == root
    }
}

#[derive(Debug, Clone)]
pub struct MerkleTree<F: Field> {
    layers: Vec<Vec<F>>,
}

impl<F: Field> MerkleTree<F> {
    pub fn build(hasher: &Poseidon<F>, leaf_hashes: &[F]) -> Self {
        assert!(
            leaf_hashes.len().is_power_of_two() && !leaf_hashes.is_empty(),
            "Merkle tree needs a nonzero power-of-two number of leaves, got {}",
            leaf_hashes.len()
        );
        let mut layers = vec![leaf_hashes.to_vec()];
        while layers.last().unwrap().len() > 1 {
            let prev = layers.last().unwrap();
            let next: Vec<F> = prev.chunks(2).map(|pair| hasher.hash_two(pair[0], pair[1])).collect();
            layers.push(next);
        }
        MerkleTree { layers }
    }

    pub fn from_values(hasher: &Poseidon<F>, values: &[F]) -> Self {
        let leaves: Vec<F> = values.iter().map(|&v| hash_leaf(hasher, v)).collect();
        Self::build(hasher, &leaves)
    }

    pub fn from_rows(hasher: &Poseidon<F>, rows: &[Vec<F>]) -> Self {
        let leaves: Vec<F> = rows.iter().map(|row| hasher.hash_many(row)).collect();
        Self::build(hasher, &leaves)
    }

    pub fn root(&self) -> F {
        self.layers.last().unwrap()[0]
    }

    pub fn num_leaves(&self) -> usize {
        self.layers[0].len()
    }

    pub fn open(&self, index: usize) -> MerklePath<F> {
        assert!(index < self.num_leaves(), "leaf index {index} out of range");
        let mut siblings = Vec::with_capacity(self.layers.len() - 1);
        let mut idx = index;
        for layer in &self.layers[..self.layers.len() - 1] {
            siblings.push(layer[idx ^ 1]);
            idx /= 2;
        }
        MerklePath { leaf_index: index, siblings }
    }
}

pub fn hash_leaf<F: Field>(hasher: &Poseidon<F>, value: F) -> F {
    hasher.hash_many(&[value])
}

#[cfg(test)]
mod tests {
    use super::*;
    use field::ToyField;

    fn hasher() -> Poseidon<ToyField> {
        Poseidon::new(5)
    }

    fn tf(x: u64) -> ToyField {
        ToyField::from_u64(x)
    }

    #[test]
    fn build_and_verify_every_leaf() {
        let h = hasher();
        let values: Vec<ToyField> = (0..16).map(tf).collect();
        let tree = MerkleTree::from_values(&h, &values);
        for (i, &v) in values.iter().enumerate() {
            let path = tree.open(i);
            assert!(path.verify(&h, tree.root(), hash_leaf(&h, v)), "leaf {i} failed to verify");
        }
    }

    #[test]
    fn tampered_leaf_fails_verification() {
        let h = hasher();
        let values: Vec<ToyField> = (0..8).map(tf).collect();
        let tree = MerkleTree::from_values(&h, &values);
        let path = tree.open(3);
        assert!(!path.verify(&h, tree.root(), hash_leaf(&h, tf(99))));
    }

    #[test]
    fn tampered_sibling_fails_verification() {
        let h = hasher();
        let values: Vec<ToyField> = (0..8).map(tf).collect();
        let tree = MerkleTree::from_values(&h, &values);
        let mut path = tree.open(2);
        path.siblings[0] += ToyField::one();
        assert!(!path.verify(&h, tree.root(), hash_leaf(&h, tf(2))));
    }

    #[test]
    fn wrong_leaf_index_fails_verification() {
        let h = hasher();
        let values: Vec<ToyField> = (0..8).map(tf).collect();
        let tree = MerkleTree::from_values(&h, &values);
        let mut path = tree.open(2);
        path.leaf_index = 5; 
        assert!(!path.verify(&h, tree.root(), hash_leaf(&h, tf(2))));
    }

    #[test]
    fn single_leaf_tree_has_leaf_as_root() {
        let h = hasher();
        let tree = MerkleTree::from_values(&h, &[tf(42)]);
        assert_eq!(tree.root(), hash_leaf(&h, tf(42)));
        let path = tree.open(0);
        assert!(path.siblings.is_empty());
        assert!(path.verify(&h, tree.root(), hash_leaf(&h, tf(42))));
    }

    #[test]
    #[should_panic(expected = "power-of-two")]
    fn rejects_non_power_of_two_leaf_count() {
        let h = hasher();
        let _ = MerkleTree::from_values(&h, &[tf(1), tf(2), tf(3)]);
    }

    #[test]
    fn from_rows_commits_multi_column_leaves() {
        let h = hasher();
        let rows: Vec<Vec<ToyField>> = vec![
            vec![tf(1), tf(2), tf(3)],
            vec![tf(4), tf(5), tf(6)],
            vec![tf(7), tf(8), tf(9)],
            vec![tf(10), tf(11), tf(12)],
        ];
        let tree = MerkleTree::from_rows(&h, &rows);
        for (i, row) in rows.iter().enumerate() {
            let leaf_hash = h.hash_many(row);
            let path = tree.open(i);
            assert!(path.verify(&h, tree.root(), leaf_hash));
        }
    }

    #[test]
    fn from_rows_with_different_column_values_gives_different_root() {
        let h = hasher();
        let rows_a: Vec<Vec<ToyField>> = vec![vec![tf(1), tf(2)], vec![tf(3), tf(4)]];
        let rows_b: Vec<Vec<ToyField>> = vec![vec![tf(1), tf(2)], vec![tf(3), tf(5)]]; 
        let tree_a = MerkleTree::from_rows(&h, &rows_a);
        let tree_b = MerkleTree::from_rows(&h, &rows_b);
        assert_ne!(tree_a.root(), tree_b.root());
    }

    #[test]
    fn different_hasher_instances_with_same_alpha_agree() {
        let h1 = hasher();
        let h2 = hasher();
        let values: Vec<ToyField> = (0..8).map(tf).collect();
        let tree1 = MerkleTree::from_values(&h1, &values);
        let tree2 = MerkleTree::from_values(&h2, &values);
        assert_eq!(tree1.root(), tree2.root());
    }
}
