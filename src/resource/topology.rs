//! Resource topology: how resources are wired to one another.

use crate::core::macros::{tmf_struct, tmf_value};
use crate::core::{ConnectionPoint, Ref, TmfType};

use super::specification::{ConnectionAssociationType, ResourceGraphRelationshipType};

tmf_value! {
    /// A graph of resources: vertices joined by [`Connection`] edges.
    ///
    /// The instance counterpart of
    /// [`ResourceGraphSpecification`](super::ResourceGraphSpecification). The
    /// specification says what may be wired together; this says what *is*, and
    /// it is what a [`ResourceFunction`] declares as its internal connectivity.
    ///
    /// This is one of the handful of v5 schemas declared as a plain object
    /// rather than an `Entity`/`Extensible`, so it carries **no `@type`,
    /// `@baseType`, `@schemaLocation` or `href`** — unusually for a type that
    /// other schemas hold a `ResourceGraphRef` to. Emitting a `@type` anyway
    /// would add a member the specification does not define, so it does not.
    ///
    /// [`ResourceFunction`]: super::ResourceKind::Function
    pub struct ResourceGraph {
        /// Identifier of the graph.
        id: String,
        /// Descriptive name for the graph.
        name: String,
        /// Narrative description.
        description: String,
        /// Relationships to other graphs.
        graph_relationship: Vec<ResourceGraphRelationship>,
        /// The edges of the graph.
        connection: Vec<Connection>,
    }
}

// Declared by hand because `tmf_value!` types have no discriminator of their
// own: `ResourceGraph` has no `@type`, but `ResourceGraphRelationship` holds a
// `ResourceGraphRef` to one, and `Ref<T>` needs a name for the *reference*.
impl TmfType for ResourceGraph {
    const TYPE_NAME: &'static str = "ResourceGraph";
    const REF_TYPE_NAME: &'static str = "ResourceGraphRef";
}

tmf_struct! {
    @name = "ResourceGraphRelationship";
    /// A link between two [`ResourceGraph`]s.
    pub struct ResourceGraphRelationship {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this relationship.
        href: String,
        /// What the relationship means.
        relationship_type: ResourceGraphRelationshipType,
        /// The graph at the other end.
        resource_graph: Ref<ResourceGraph>,
    }
}

tmf_struct! {
    @name = "Connection";
    /// An edge in a [`ResourceGraph`], joining two or more [`Endpoint`]s.
    pub struct Connection {
        /// Identifier of the edge.
        id: String,
        /// Canonical URI of this connection.
        href: String,
        /// Descriptive name for the edge.
        name: String,
        /// How the endpoints associate.
        association_type: ConnectionAssociationType,
        /// The vertices this edge joins.
        ///
        /// TMF639 requires at least two. That is a constraint on the payload
        /// rather than on the type: a response carrying one is still parsed,
        /// because refusing to read a whole inventory over a malformed edge
        /// serves nobody — see the requiredness rule in
        /// [`core::macros`](crate::core::macros).
        endpoint: Vec<Endpoint>,
    }
}

tmf_struct! {
    @name = "EndpointRef";
    /// A vertex in a [`ResourceGraph`].
    ///
    /// The v5 schema is an `EntityRef` with two members added, so this is a
    /// struct rather than a [`Ref<T>`](crate::core::Ref) — a reference that
    /// carries its own data is no longer just a pointer.
    pub struct Endpoint {
        /// Identifier of the referenced endpoint.
        id: String,
        /// Canonical URI of the referenced endpoint.
        href: String,
        /// Name of the referenced endpoint.
        name: String,
        /// Whether this endpoint is a source rather than a sink.
        ///
        /// TMF639 defaults it to `true`, and says connectivity is
        /// bidirectional when every endpoint of a connection is a source.
        /// Absent here means the server did not say, which is not the same as
        /// `false` — read it with `unwrap_or(true)` to apply the default.
        is_root: bool,
        /// The connection point this endpoint attaches to.
        connection_point: Ref<ConnectionPoint>,
        @renamed {
            /// The concrete class of the referenced endpoint.
            "@referredType" referred_type: String,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_graph_round_trips_with_its_edges() {
        // `ResourceGraph` carries no `@type` of its own — the v5 schema does
        // not define one — while the `Connection` and `EndpointRef` inside it do.
        let json = r#"{"id":"g1","name":"fabric","connection":[{"id":"c1","associationType":"pointtoPoint","endpoint":[{"id":"e1","isRoot":true,"@type":"EndpointRef"},{"id":"e2","isRoot":false,"@type":"EndpointRef"}],"@type":"Connection"}]}"#;
        let graph: ResourceGraph = serde_json::from_str(json).unwrap();

        let edges = graph.connection.as_ref().unwrap();
        assert_eq!(
            edges[0].association_type,
            Some(ConnectionAssociationType::PointToPoint)
        );
        assert_eq!(edges[0].endpoint.as_ref().unwrap().len(), 2);

        assert_eq!(
            serde_json::to_value(&graph).unwrap(),
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
    }

    #[test]
    fn an_absent_is_root_is_not_false() {
        // TMF639 defaults `isRoot` to true, so a missing member and an explicit
        // `false` mean opposite things. Typing it as a plain `bool` would
        // collapse them and silently reverse the direction of an edge.
        let endpoint: Endpoint = serde_json::from_str(r#"{"id":"e1"}"#).unwrap();
        assert_eq!(endpoint.is_root, None);
        assert!(endpoint.is_root.unwrap_or(true), "the spec default is true");
    }
}
