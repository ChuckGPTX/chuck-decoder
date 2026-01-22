
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use serde::{Deserialize, Serialize};

/// The Map of the Chip.
/// Keys = Stabilizer Indices (0, 1, 2...)
/// Values = List of Qubits they measure (e.g., [12, 13, 19, 20])
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodingGraph {
    adjacency: HashMap<usize, Vec<u32>>,
}

impl DecodingGraph {
    pub fn new() -> Self {
        Self { adjacency: HashMap::new() }
    }

    /// Load the graph topology from a JSON file
    pub fn from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let adjacency: HashMap<String, Vec<u32>> = serde_json::from_reader(reader)?;
        
        // Convert String keys "0" back to usize 0
        let mut parsed_adjacency = HashMap::new();
        for (k, v) in adjacency {
            if let Ok(idx) = k.parse::<usize>() {
                parsed_adjacency.insert(idx, v);
            }
        }
        
        Ok(Self { adjacency: parsed_adjacency })
    }

    pub fn add_stabilizer(&mut self, stabilizer_idx: usize, qubits: Vec<u32>) {
        self.adjacency.insert(stabilizer_idx, qubits);
    }

    pub fn get_neighbors(&self, stabilizer_idx: usize) -> &[u32] {
        self.adjacency.get(&stabilizer_idx).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChuckDecoder {
    qubit_weights: HashMap<u32, f64>,
    syndrome_cache: HashMap<String, String>,
    graph: DecodingGraph,
    pub total_decodes: u64,
    pub cache_hits: u64,
}

impl ChuckDecoder {
    pub fn new(weights: HashMap<u32, f64>, graph: DecodingGraph) -> Self {
        Self {
            qubit_weights: weights,
            syndrome_cache: HashMap::new(),
            graph,
            total_decodes: 0,
            cache_hits: 0,
        }
    }

    pub fn decode(&mut self, syndrome: &str) -> String {
        self.total_decodes += 1;
        if let Some(correction) = self.syndrome_cache.get(syndrome) {
            self.cache_hits += 1;
            return correction.clone();
        }
        let correction = self.adaptive_decode(syndrome);
        self.syndrome_cache.insert(syndrome.to_string(), correction.clone());
        correction
    }

    fn adaptive_decode(&self, syndrome: &str) -> String {
        let syndrome_bits: Vec<bool> = syndrome.chars().map(|c| c == '1').collect();
        let mut suspects = Vec::new();
        
        for (i, &triggered) in syndrome_bits.iter().enumerate() {
            if triggered {
                // NOW USING THE DYNAMIC GRAPH!
                let neighbors = self.graph.get_neighbors(i);
                suspects.extend_from_slice(neighbors);
            }
        }

        if suspects.is_empty() { return "NO_CORRECTION".to_string(); }

        let best_guess = suspects.iter()
            .max_by(|a, b| {
                let w_a = self.qubit_weights.get(a).unwrap_or(&0.01);
                let w_b = self.qubit_weights.get(b).unwrap_or(&0.01);
                w_a.partial_cmp(w_b).unwrap()
            })
            .unwrap();

        format!("FLIP_Q{}", best_guess)
    }
    
    pub fn weights(&self) -> &HashMap<u32, f64> { &self.qubit_weights }
}
