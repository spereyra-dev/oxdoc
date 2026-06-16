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
fn representative_docx_tables_json_matches_schema() {
    let schema = read_json_schema("oxdoc-docx-tables.schema.json");
    let output = serde_json::json!({
        "schema_version": 1,
        "file": "contract.docx",
        "document_type": "docx",
        "tables": [
            {
                "part_type": "main",
                "part_path": "word/document.xml",
                "table_ordinal": 1,
                "complete": true,
                "grid_column_count": 2,
                "rows": [
                    {
                        "row_ordinal": 1,
                        "grid_before": 0,
                        "grid_after": 0,
                        "complete": true,
                        "cells": [
                            {
                                "cell_ordinal": 1,
                                "grid_start": 0,
                                "grid_span": 1,
                                "vertical_merge": "none",
                                "complete": true,
                                "blocks": [
                                    {"type": "paragraph", "text": "Cell"},
                                    {
                                        "type": "table",
                                        "complete": true,
                                        "rows": [
                                            {
                                                "row_ordinal": 1,
                                                "grid_before": 0,
                                                "grid_after": 0,
                                                "complete": true,
                                                "cells": []
                                            }
                                        ]
                                    }
                                ]
                            }
                        ]
                    }
                ]
            }
        ],
        "warnings": [
            {
                "category": "parser",
                "code": "W001",
                "path": "word/document.xml",
                "message": "stopped after malformed XML: unexpected EOF with open table"
            }
        ]
    });

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
fn representative_xlsx_rows_jsonl_record_matches_schema_shape() {
    let schema = read_json_schema("oxdoc-xlsx-rows-jsonl.schema.json");
    let output: Value = serde_json::from_str(
        r##"{
            "schema_version": 1,
            "file": "typed.xlsx",
            "sheet_name": "Data",
            "row_index": 2,
            "cells": [
                {"column_index": 0, "kind": "blank", "has_formula": false},
                {"column_index": 2, "kind": "string", "raw": "text", "value": "text", "has_formula": false},
                {"column_index": 3, "kind": "boolean", "raw": "1", "value": true, "has_formula": false},
                {"column_index": 4, "kind": "number", "raw": "44927", "formatted": "2023-01-01", "has_formula": true},
                {"column_index": 5, "kind": "error", "raw": "#N/A", "has_formula": false}
            ]
        }"##,
    )
    .unwrap();

    validate_object(&schema, &output);
    assert_eq!(
        output["schema_version"],
        schema["properties"]["schema_version"]["const"]
    );
    assert!(output["row_index"].as_u64().is_some());

    let variants = schema["$defs"]["cell"]["oneOf"].as_array().unwrap();
    let cells = output["cells"].as_array().unwrap();
    assert_eq!(variants.len(), cells.len());

    for cell in cells {
        let kind = cell["kind"].as_str().unwrap();
        let definition_name = format!("{kind}Cell");
        let definition = schema["$defs"].get(&definition_name).unwrap();
        validate_cell(&schema, definition, cell);
        assert_eq!(definition["properties"]["kind"]["const"], kind);
    }

    assert!(output["cells"][3]["raw"].is_string());
    assert!(output["cells"][3].get("value").is_none());
}

#[test]
fn representative_xlsx_schema_report_matches_schema_shape() {
    let schema = read_json_schema("oxdoc-xlsx-schema.schema.json");
    let output = serde_json::json!({
        "schema_version": 1,
        "experimental": true,
        "file": "book.xlsx",
        "sheet_name": "Data",
        "scan": {
            "mode": "sampled",
            "sample_rows": 100,
            "examined_rows": 3
        },
        "header_policy": "none",
        "columns": [{
            "column_index": 0,
            "name": "A",
            "logical_type": "float64",
            "nullable": true,
            "observed_types": ["int64", "float64", "null"]
        }],
        "warnings": [{
            "code": "sampled_result",
            "message": "schema inference used a row sample; results are approximate"
        }]
    });

    let required = schema["required"].as_array().unwrap();
    let properties = schema["properties"].as_object().unwrap();
    for field in required {
        assert!(output.get(field.as_str().unwrap()).is_some());
    }
    for field in output.as_object().unwrap().keys() {
        assert!(properties.contains_key(field));
    }
    assert_eq!(output["schema_version"], 1);
    assert_eq!(output["experimental"], true);
    assert_eq!(output["header_policy"], "none");
    assert_eq!(output["scan"]["mode"], "sampled");
    assert!(output["scan"]["sample_rows"].as_u64().is_some());
    assert!(output["columns"][0]["column_index"].as_u64().is_some());
    assert_eq!(output["columns"][0]["name"], "A");
    assert_eq!(output["warnings"][0]["code"], "sampled_result");
}

#[test]
fn schemas_have_stable_public_metadata() {
    for name in [
        "oxdoc-info.schema.json",
        "oxdoc-extract-text.schema.json",
        "oxdoc-structured-text.schema.json",
        "oxdoc-docx-tables.schema.json",
        "oxdoc-audit.schema.json",
        "oxdoc-all-sheets-manifest.schema.json",
        "oxdoc-xlsx-rows-jsonl.schema.json",
        "oxdoc-xlsx-schema.schema.json",
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

fn validate_cell(schema: &Value, definition: &Value, cell: &Value) {
    let cell = cell.as_object().expect("representative cell is an object");
    let required = definition["required"]
        .as_array()
        .expect("cell variant declares required fields");
    let properties = definition["properties"]
        .as_object()
        .expect("cell variant declares properties");

    assert_eq!(
        definition["additionalProperties"].as_bool(),
        Some(false),
        "cell variant is strict"
    );

    for field in required {
        let field = field.as_str().unwrap();
        assert!(
            cell.contains_key(field),
            "missing required cell field {field}"
        );
    }

    for (field, value) in cell {
        let property = properties
            .get(field)
            .unwrap_or_else(|| panic!("cell field {field} is not declared"));
        let property = property
            .get("$ref")
            .and_then(Value::as_str)
            .map(|reference| {
                reference
                    .strip_prefix("#/$defs/cellBaseProperties/")
                    .and_then(|name| schema["$defs"]["cellBaseProperties"].get(name))
                    .unwrap_or_else(|| panic!("unsupported cell property reference {reference}"))
            })
            .unwrap_or(property);

        if let Some(expected_type) = property.get("type").and_then(Value::as_str) {
            assert_json_type(field, value, expected_type);
        }
        if let Some(expected) = property.get("const") {
            assert_eq!(value, expected, "cell field {field} has the wrong constant");
        }
        if property.get("minimum").and_then(Value::as_i64) == Some(0) {
            assert!(
                value.as_u64().is_some(),
                "cell field {field} must be a non-negative integer"
            );
        }
    }
}
