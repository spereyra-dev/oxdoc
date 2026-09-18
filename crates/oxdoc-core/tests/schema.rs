use serde_json::Value;

#[test]
fn representative_info_json_matches_schema() {
    let schema = read_json_schema("v1", "oxdoc-info.schema.json");
    let output = serde_json::from_str(&read_snapshot("cli_info_json.json")).unwrap();

    validate_object(&schema, &output);
}

#[test]
fn representative_extract_text_json_matches_schema() {
    let schema = read_json_schema("v1", "oxdoc-extract-text.schema.json");
    let output = serde_json::from_str(&read_snapshot("cli_extract_text_json.json")).unwrap();

    validate_object(&schema, &output);
}

#[test]
fn representative_structured_text_json_matches_schema() {
    let schema = read_json_schema("v2", "oxdoc-structured-text.schema.json");
    let output = serde_json::from_str(&read_snapshot("cli_structured_text_json.json")).unwrap();

    validate_object(&schema, &output);
    assert_eq!(
        output["schema_version"],
        schema["properties"]["schema_version"]["const"]
    );
}

#[test]
fn representative_structured_text_pptx_json_matches_schema() {
    let schema = read_json_schema("v2", "oxdoc-structured-text.schema.json");
    let output =
        serde_json::from_str(&read_snapshot("cli_structured_text_pptx_json.json")).unwrap();

    validate_object(&schema, &output);
    assert_eq!(
        output["schema_version"],
        schema["properties"]["schema_version"]["const"]
    );
}

#[test]
fn representative_docx_tables_json_matches_schema() {
    let schema = read_json_schema("v1", "oxdoc-docx-tables.schema.json");
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
    let schema = read_json_schema("v1", "oxdoc-audit.schema.json");
    let output = serde_json::from_str(&read_snapshot("cli_audit_json.json")).unwrap();

    validate_object(&schema, &output);
}

#[test]
fn representative_audit_jsonl_record_matches_schema_shape() {
    let schema = read_json_schema("v1", "oxdoc-audit-jsonl.schema.json");
    let output = serde_json::json!({
        "schema_version": 1,
        "file": "report.docx",
        "document_type": "docx",
        "audit": {
            "oxdoc_version": "1.2.0",
            "file": "report.docx",
            "document_type": "docx",
            "metadata": {
                "file": "report.docx",
                "has_macros": false,
                "custom_properties": {}
            },
            "signals": []
        },
        "warnings": [
            {
                "category": "parser",
                "code": "W001",
                "path": "word/document.xml",
                "message": "stopped after malformed XML: parse error"
            }
        ]
    });
    let error_output = serde_json::json!({
        "schema_version": 1,
        "file": "missing.docx",
        "document_type": "unknown",
        "error": {
            "code": "E001",
            "message": "No such file or directory"
        }
    });

    validate_object(&schema, &output);
    validate_object(&schema, &error_output);
}

#[test]
fn representative_all_sheets_manifest_matches_schema() {
    let schema = read_json_schema("v1", "oxdoc-all-sheets-manifest.schema.json");
    let output = serde_json::from_str(&read_snapshot("all_sheets_manifest.json")).unwrap();

    validate_object(&schema, &output);
}

#[test]
fn representative_xlsx_rows_jsonl_record_matches_schema_shape() {
    let schema = read_json_schema("v1", "oxdoc-xlsx-rows-jsonl.schema.json");
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
    let schema = read_json_schema("v1", "oxdoc-xlsx-schema.schema.json");
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
    const SCHEMA_VERSIONS: &[(&str, &[&str])] = &[
        (
            "v1",
            &[
                "oxdoc-info.schema.json",
                "oxdoc-extract-text.schema.json",
                "oxdoc-structured-text.schema.json",
                "oxdoc-docx-tables.schema.json",
                "oxdoc-audit.schema.json",
                "oxdoc-audit-jsonl.schema.json",
                "oxdoc-all-sheets-manifest.schema.json",
                "oxdoc-xlsx-rows-jsonl.schema.json",
                "oxdoc-xlsx-schema.schema.json",
                "oxdoc-pptx-slides.schema.json",
            ],
        ),
        ("v2", &["oxdoc-structured-text.schema.json"]),
    ];

    for (version, names) in SCHEMA_VERSIONS {
        for name in *names {
            assert_schema_metadata(version, name);
        }
    }
}

#[test]
fn representative_pptx_slides_json_payload_matches_schema() {
    let schema = read_json_schema("v1", "oxdoc-pptx-slides.schema.json");
    let output = serde_json::from_str(&read_snapshot("cli_pptx_slides_json.json")).unwrap();

    validate_slides_object(&schema, &output);
    assert_eq!(output["schema_version"], 1);
    assert_eq!(output["document_type"], "pptx");
}

#[test]
fn representative_pptx_slides_jsonl_record_matches_schema_shape() {
    let schema = read_json_schema("v1", "oxdoc-pptx-slides.schema.json");
    let first_line = read_snapshot("cli_pptx_slides_jsonl.jsonl")
        .lines()
        .next()
        .expect("jsonl snapshot has at least one record")
        .to_owned();
    let output: Value = serde_json::from_str(&first_line).unwrap();

    validate_slides_object(&schema, &output);

    // slide_id is optional: an @id-less slide record (key omitted) validates.
    let record: Value = serde_json::json!({
        "schema_version": 1,
        "file": "slides-deck.pptx",
        "slide_ordinal": 2,
        "slide_path": "ppt/slides/slide1.xml",
        "text": "Second Slide\n"
    });
    validate_slides_object(&schema, &record);
    assert!(record.get("slide_id").is_none());
    assert_eq!(record["schema_version"], 1);
}

#[test]
fn v2_payload_fails_frozen_v1_validation() {
    // The v2 payload adds `schema_version`, which the frozen v1 schema does not
    // declare; with `additionalProperties: false` this is the documented,
    // intentional breakage strict v1 validators must migrate for.
    let v1_schema = read_json_schema("v1", "oxdoc-structured-text.schema.json");
    let v2_payload = serde_json::json!({
        "schema_version": 2,
        "file": "contract.docx",
        "document_type": "docx",
        "blocks": []
    });

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(move || {
        let schema = v1_schema.clone();
        let payload = v2_payload.clone();
        validate_object(&schema, &payload)
    });
    std::panic::set_hook(previous_hook);

    assert!(
        result.is_err(),
        "a v2 payload must fail the frozen v1 schema (undeclared schema_version)"
    );
}

#[test]
fn slides_payload_fails_frozen_structured_text_validation() {
    // The slides payload declares `slides` (not `blocks`) and pins
    // `schema_version: 1`/`document_type: "pptx"`; the structured-text
    // schemas (v1 and frozen v2) reject it — this is a NEW contract, not a
    // structured-text widening.
    let payload: Value = serde_json::from_str(&read_snapshot("cli_pptx_slides_json.json")).unwrap();

    for version in ["v2", "v1"] {
        let schema = read_json_schema(version, "oxdoc-structured-text.schema.json");

        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = std::panic::catch_unwind({
            let schema = schema.clone();
            let payload = payload.clone();
            move || validate_object(&schema, &payload)
        });
        std::panic::set_hook(previous_hook);

        assert!(
            result.is_err(),
            "a slides payload must fail the frozen {version} structured-text schema"
        );
    }
}

fn assert_schema_metadata(version: &str, name: &str) {
    let schema = read_json_schema(version, name);

    assert_eq!(
        schema.get("$schema").and_then(Value::as_str),
        Some("https://json-schema.org/draft/2020-12/schema")
    );
    assert!(
        schema
            .get("$id")
            .and_then(Value::as_str)
            .is_some_and(|id| id.ends_with(&format!("/schemas/{version}/{name}")))
    );
    assert_eq!(schema.get("type").and_then(Value::as_str), Some("object"));

    if let Some(branches) = schema.get("oneOf").and_then(Value::as_array) {
        // oneOf schemas keep their strictness either on the referenced
        // branch definitions (payload/record $refs) or on the top level next
        // to inline discrimination branches (audit-jsonl).
        let mut inline_branch = false;
        for branch in branches {
            match branch.get("$ref").and_then(Value::as_str) {
                Some(reference) => {
                    let pointer = reference.strip_prefix('#').unwrap_or_else(|| {
                        panic!("unsupported reference {reference} in {version}/{name}")
                    });
                    let definition = schema.pointer(pointer).unwrap_or_else(|| {
                        panic!("unresolved reference {reference} in {version}/{name}")
                    });
                    assert_eq!(
                        definition
                            .get("additionalProperties")
                            .and_then(Value::as_bool),
                        Some(false),
                        "oneOf branch {reference} in {version}/{name} is strict"
                    );
                }
                None => inline_branch = true,
            }
        }
        if inline_branch {
            assert_eq!(
                schema.get("additionalProperties").and_then(Value::as_bool),
                Some(false),
                "inline oneOf branches keep the top-level strictness in {version}/{name}"
            );
        }
    } else {
        assert_eq!(
            schema.get("additionalProperties").and_then(Value::as_bool),
            Some(false)
        );
    }
}

fn read_json_schema(version: &str, name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas")
        .join(version)
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
    validate_against(schema, output, schema);
}

fn validate_against(definition: &Value, output: &Value, root: &Value) {
    let output = output
        .as_object()
        .expect("representative output is an object");
    let required = definition
        .get("required")
        .and_then(Value::as_array)
        .expect("schema declares required fields");
    let properties = definition
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
        let property = property
            .get("$ref")
            .and_then(Value::as_str)
            .filter(|reference| reference.starts_with('#'))
            .map(|reference| resolve_schema_reference(root, reference))
            .unwrap_or(property);
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

fn resolve_schema_reference<'a>(schema: &'a Value, reference: &str) -> &'a Value {
    let pointer = reference
        .strip_prefix('#')
        .unwrap_or_else(|| panic!("unsupported schema reference {reference}"));
    schema
        .pointer(pointer)
        .unwrap_or_else(|| panic!("unresolved schema reference {reference}"))
}

fn validate_slides_object(schema: &Value, output: &Value) {
    let branches = schema
        .get("oneOf")
        .and_then(Value::as_array)
        .expect("slides schema declares a top-level oneOf");
    let matched = branches
        .iter()
        .map(|branch| {
            resolve_schema_reference(
                schema,
                branch
                    .get("$ref")
                    .and_then(Value::as_str)
                    .expect("slides oneOf branches are $refs"),
            )
        })
        .find(|definition| {
            definition["required"]
                .as_array()
                .expect("branch declares required fields")
                .iter()
                .all(|field| output.get(field.as_str().unwrap()).is_some())
        })
        .expect("slides output matches exactly one schema branch");

    validate_against(matched, output, schema);
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
