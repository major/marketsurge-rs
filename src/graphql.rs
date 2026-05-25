use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// GraphQL request envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound(serialize = "V: Serialize", deserialize = "V: DeserializeOwned"))]
pub struct GraphQLRequest<V> {
    #[serde(
        default,
        rename = "operationName",
        skip_serializing_if = "str::is_empty"
    )]
    pub operation_name: String,
    pub variables: V,
    pub query: String,
}

impl<V> GraphQLRequest<V> {
    /// Creates a GraphQL request envelope.
    #[must_use]
    pub fn new(operation_name: impl Into<String>, variables: V, query: impl Into<String>) -> Self {
        Self {
            operation_name: operation_name.into(),
            variables,
            query: query.into(),
        }
    }
}

/// GraphQL response envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: DeserializeOwned"))]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLFieldError>>,
}

/// Single GraphQL field error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphQLFieldError {
    pub message: String,
    pub path: Option<Vec<serde_json::Value>>,
    pub extensions: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Variables {
        id: u32,
    }

    #[test]
    fn request_serializes_operation_name_when_present() {
        let request = GraphQLRequest {
            operation_name: "GetThing".to_string(),
            variables: Variables { id: 7 },
            query: "query GetThing($id: Int!) { thing(id: $id) { id } }".to_string(),
        };

        let value = serde_json::to_value(&request).expect("serialize request");

        assert_eq!(value["operationName"], "GetThing");
        assert_eq!(value["variables"]["id"], 7);
        assert_eq!(
            value["query"],
            "query GetThing($id: Int!) { thing(id: $id) { id } }"
        );

        let round_trip: GraphQLRequest<Variables> =
            serde_json::from_value(value).expect("deserialize request");
        assert_eq!(round_trip.operation_name, "GetThing");
        assert_eq!(round_trip.variables, Variables { id: 7 });
    }

    #[test]
    fn request_omits_empty_operation_name() {
        let request = GraphQLRequest {
            operation_name: String::new(),
            variables: Variables { id: 42 },
            query: "query { thing { id } }".to_string(),
        };

        let value = serde_json::to_value(&request).expect("serialize request");

        assert!(value.get("operationName").is_none());

        let round_trip: GraphQLRequest<Variables> =
            serde_json::from_value(value).expect("deserialize request");
        assert_eq!(round_trip.operation_name, "");
        assert_eq!(round_trip.variables, Variables { id: 42 });
    }

    #[test]
    fn response_deserializes_data_and_empty_errors() {
        let json = serde_json::json!({
            "data": { "id": 7 },
            "errors": []
        });

        let response: GraphQLResponse<Variables> =
            serde_json::from_value(json).expect("deserialize response");

        assert_eq!(response.data, Some(Variables { id: 7 }));
        assert_eq!(response.errors, Some(vec![]));

        let value = serde_json::to_value(&response).expect("serialize response");
        assert_eq!(value["data"]["id"], 7);
        assert_eq!(value["errors"], serde_json::json!([]));
    }

    #[test]
    fn field_error_deserializes_all_fields() {
        let json = serde_json::json!({
            "message": "boom",
            "path": ["thing", 0, "id"],
            "extensions": { "code": "BAD_THING" }
        });

        let error: GraphQLFieldError =
            serde_json::from_value(json).expect("deserialize field error");

        assert_eq!(error.message, "boom");
        assert_eq!(
            error.path,
            Some(vec![
                serde_json::Value::String("thing".to_string()),
                serde_json::Value::from(0),
                serde_json::Value::String("id".to_string()),
            ])
        );
        assert_eq!(
            error.extensions,
            Some(serde_json::json!({ "code": "BAD_THING" }))
        );

        let value = serde_json::to_value(&error).expect("serialize field error");
        assert_eq!(value["message"], "boom");
        assert_eq!(value["path"], serde_json::json!(["thing", 0, "id"]));
    }
}
