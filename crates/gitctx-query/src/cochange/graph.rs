//! In-memory co-change graph built on `petgraph` per GRAPH-04.
//!
//! Wraps an undirected `petgraph::Graph` where nodes are entities (files or functions)
//! and edges carry coupling metrics (support, confidence, lift).

use crate::cochange::metrics::CoChangeMetrics;
use petgraph::graph::{NodeIndex, UnGraph};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Entity type discriminator for graph nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    File,
    Function,
}

/// A node in the co-change graph representing a file or function entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityNode {
    /// File or function discriminator.
    pub entity_type: EntityType,
    /// Entity identifier: file path or qualified name.
    pub name: String,
}

/// An edge in the co-change graph carrying coupling metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoChangeEdge {
    /// Raw co-occurrence count.
    pub support: u32,
    /// Time-decay-weighted support.
    pub weighted_support: f64,
    /// P(B|A) — confidence from the first node to the second.
    pub confidence_a_to_b: f64,
    /// P(A|B) — confidence from the second node to the first.
    pub confidence_b_to_a: f64,
    /// Lift — co-occurrence above random chance.
    pub lift: f64,
}

/// A coupled entity result from graph traversal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoupledEntity {
    /// Entity name (file path or qualified name).
    pub name: String,
    /// Entity type.
    pub entity_type: EntityType,
    /// Raw support count.
    pub support: u32,
    /// Time-decay-weighted support.
    pub weighted_support: f64,
    /// Directional confidence: P(coupled | target).
    pub confidence: f64,
    /// Lift.
    pub lift: f64,
}

/// In-memory co-change multigraph wrapping `petgraph`.
pub struct CoChangeGraph {
    graph: UnGraph<EntityNode, CoChangeEdge>,
    name_to_index: HashMap<String, NodeIndex>,
}

impl CoChangeGraph {
    /// Build a graph from computed co-change metrics.
    pub fn from_metrics(metrics: &[CoChangeMetrics], entity_type: EntityType) -> Self {
        let mut graph = UnGraph::new_undirected();
        let mut name_to_index: HashMap<String, NodeIndex> = HashMap::new();

        for m in metrics {
            // Ensure both nodes exist
            let idx_a = *name_to_index.entry(m.entity_a.clone()).or_insert_with(|| {
                graph.add_node(EntityNode {
                    entity_type,
                    name: m.entity_a.clone(),
                })
            });
            let idx_b = *name_to_index.entry(m.entity_b.clone()).or_insert_with(|| {
                graph.add_node(EntityNode {
                    entity_type,
                    name: m.entity_b.clone(),
                })
            });

            graph.add_edge(
                idx_a,
                idx_b,
                CoChangeEdge {
                    support: m.support,
                    weighted_support: m.weighted_support,
                    confidence_a_to_b: m.confidence_a_to_b,
                    confidence_b_to_a: m.confidence_b_to_a,
                    lift: m.lift,
                },
            );
        }

        Self {
            graph,
            name_to_index,
        }
    }

    /// Return the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Return the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Get all coupled entities for a given entity, sorted by weighted_support descending.
    pub fn neighbors_of(&self, entity: &str) -> Vec<CoupledEntity> {
        let Some(&idx) = self.name_to_index.get(entity) else {
            return Vec::new();
        };

        let mut results = Vec::new();
        for edge_ref in self.graph.edges(idx) {
            let neighbor_idx = if edge_ref.source() == idx {
                edge_ref.target()
            } else {
                edge_ref.source()
            };

            let neighbor_node = &self.graph[neighbor_idx];
            let edge = edge_ref.weight();

            // Determine directional confidence: P(neighbor | entity)
            // If entity is entity_a in the edge, use confidence_a_to_b (P(B|A))
            // If entity is entity_b, use confidence_b_to_a (P(A|B))
            let confidence = if self.graph[idx].name <= neighbor_node.name {
                edge.confidence_a_to_b
            } else {
                edge.confidence_b_to_a
            };

            results.push(CoupledEntity {
                name: neighbor_node.name.clone(),
                entity_type: neighbor_node.entity_type,
                support: edge.support,
                weighted_support: edge.weighted_support,
                confidence,
                lift: edge.lift,
            });
        }

        results.sort_by(|a, b| {
            b.weighted_support
                .partial_cmp(&a.weighted_support)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Get the top-k coupled entities for a given entity by weighted_support.
    pub fn top_coupled(&self, entity: &str, k: usize) -> Vec<CoupledEntity> {
        let mut neighbors = self.neighbors_of(entity);
        neighbors.truncate(k);
        neighbors
    }

    /// Check if an entity exists in the graph.
    pub fn contains(&self, entity: &str) -> bool {
        self.name_to_index.contains_key(entity)
    }

    /// Return the entity type of a named node if present in the graph.
    pub fn entity_type(&self, entity: &str) -> Option<EntityType> {
        self.name_to_index
            .get(entity)
            .map(|&idx| self.graph[idx].entity_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_metric(a: &str, b: &str, support: u32, ws: f64) -> CoChangeMetrics {
        CoChangeMetrics {
            entity_a: a.to_string(),
            entity_b: b.to_string(),
            support,
            weighted_support: ws,
            confidence_a_to_b: 0.8,
            confidence_b_to_a: 0.6,
            lift: 1.5,
            total_qualifying_commits: 10,
        }
    }

    #[test]
    fn test_empty_graph() {
        let graph = CoChangeGraph::from_metrics(&[], EntityType::File);
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_single_edge() {
        let metrics = vec![make_metric("a.rs", "b.rs", 3, 2.5)];
        let graph = CoChangeGraph::from_metrics(&metrics, EntityType::File);
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        assert!(graph.contains("a.rs"));
        assert!(graph.contains("b.rs"));
        assert!(!graph.contains("c.rs"));
    }

    #[test]
    fn test_neighbors_of() {
        let metrics = vec![
            make_metric("a.rs", "b.rs", 5, 4.0),
            make_metric("a.rs", "c.rs", 2, 1.5),
        ];
        let graph = CoChangeGraph::from_metrics(&metrics, EntityType::File);

        let neighbors = graph.neighbors_of("a.rs");
        assert_eq!(neighbors.len(), 2);
        // Should be sorted by weighted_support descending
        assert_eq!(neighbors[0].name, "b.rs");
        assert_eq!(neighbors[1].name, "c.rs");
    }

    #[test]
    fn test_top_coupled() {
        let metrics = vec![
            make_metric("a.rs", "b.rs", 5, 4.0),
            make_metric("a.rs", "c.rs", 3, 3.0),
            make_metric("a.rs", "d.rs", 2, 1.0),
        ];
        let graph = CoChangeGraph::from_metrics(&metrics, EntityType::File);

        let top2 = graph.top_coupled("a.rs", 2);
        assert_eq!(top2.len(), 2);
        assert_eq!(top2[0].name, "b.rs");
        assert_eq!(top2[1].name, "c.rs");
    }

    #[test]
    fn test_neighbors_of_nonexistent() {
        let graph = CoChangeGraph::from_metrics(&[], EntityType::File);
        let neighbors = graph.neighbors_of("nonexistent.rs");
        assert!(neighbors.is_empty());
    }
}
