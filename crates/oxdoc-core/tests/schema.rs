use serde_json::Value;

#[test]
fn representative_info_json_matches_schema() {
    let schema = read_json_schema("oxdoc-info.schema.json");
    let output = serde_json::from_str(&read_snapshot("cli_info_json.json")).unwrap();

    validate_object(&schema, &output);
}

#[test]
fn representative_extract_text_json_matches_schema() {
    let schema = read_json_schema("oxdoc-extract-text.schema.json");
    let output = serde_json::from_str(&read_snapshot("cli_extract_text_json.json")).unwrap();

    validate_object(&schema, &output);
}

#[test]
fn representative_structured_text_json_matches_schema() {
    let schema = read_json_schema("oxdoc-structured-text.schema.json");
    let output = serde_json::from_str(&read_snapshot("cli_structured_text_json.json")).unwrap();

    validate_object(&schema, &output);
}

#[test]
fn representative_audit_json_matches_schema() {
    let schema = read_json_schema("oxdoc-audit.schema.json");
    let output = serde_json::from_str(&read_snapshot("cli_audit_json.json")).unwrap();

    validate_object(&schema, &output);
}

#[test]
fn representative_all_sheets_manifest_matches_schema() {
    let schema = read_json_schema("oxdoc-all-sheets-manifest.schema.json");
    let output = serde_json::from_str(&read_snapshot("all_sheets_manifest.json")).unwrap();

    validate_object(&schema, &output);
}

#[test]
fn schemas_have_stable_public_metadata() {
    for name in [
        "oxdoc-info.schema.json",
        "oxdoc-extract-text.schema.json",
        "oxdoc-structured-text.schema.json",
        "oxdoc-audit.schema.json",
        "oxdoc-all-sheets-manifest.schema.json",
    ] {
        let schema = read_json_schema(name);

        assert_eq!(
            schema.get("$schema").and_then(Value::as_str),
            Some("https://json-schema.org/draft/2020-12/schema")
        );
        assert!(
            schema
                .get("$id")
                .and_then(Value::as_str)
                .is_some_and(|id| id.ends_with(&format!("/schemas/v1/{name}")))
        );
        assert_eq!(schema.get("type").and_then(Value::as_str), Some("object"));
        assert_eq!(
            schema.get("additionalProperties").and_then(Value::as_bool),
            Some(false)
        );
    }
}

fn read_json_schema(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/v1")
        .join(name);
    let source = std::fs::read_to_string(path).unwrap();

    serde_json::from_str(&source).unwrap()
}

fn read_snapshot(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/snapshots")
        .join(name);

    std::fs::read_to_string(path).unwrap()
}

fn validate_object(schema: &Value, output: &Value) {
    let output = output
        .as_object()
        .expect("representative output is an object");
    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .expect("schema declares required fields");
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .expect("schema declares properties");

    for field in required {
        let field = field.as_str().expect("required field names are strings");
        assert!(output.contains_key(field), "missing required field {field}");
    }

    for (field, value) in output {
        let property = properties
            .get(field)
            .unwrap_or_else(|| panic!("field {field} is not declared in schema"));
        let expected_type = property
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("field {field} is missing a type"));

        assert_json_type(field, value, expected_type);

        if expected_type == "integer" && property.get("minimum").and_then(Value::as_i64) == Some(0)
        {
            assert!(
                value.as_u64().is_some(),
                "field {field} must be a non-negative integer"
            );
        }
    }
}

fn assert_json_type(field: &str, value: &Value, expected_type: &str) {
    let matches = match expected_type {
        "boolean" => value.is_boolean(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "string" => value.is_string(),
        "object" => value.is_object(),
        "array" => value.is_array(),
        other => panic!("unsupported test schema type {other} for field {field}"),
    };

    assert!(
        matches,
        "field {field} has value {value:?}, expected {expected_type}"
    );
}
