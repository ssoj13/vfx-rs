//! Test serde_yaml tag support

use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
enum Transform {
    #[serde(rename = "MatrixTransform")]
    Matrix { matrix: Vec<f64> },
    #[serde(rename = "FileTransform")]
    File { src: String },
}

#[test]
fn test_standard_tag() {
    // Standard YAML tag format: !Tag
    let yaml = "!MatrixTransform {matrix: [1, 0, 0, 1]}";
    let result: Result<Transform, _> = serde_yaml::from_str(yaml);
    eprintln!("!Tag result: {:?}", result);
    assert!(result.is_ok(), "!Tag format should work");
}

#[test]
fn test_ocio_tag() {
    // OCIO tag format: !<Tag>
    let yaml = "!<MatrixTransform> {matrix: [1, 0, 0, 1]}";
    let result: Result<Transform, _> = serde_yaml::from_str(yaml);
    eprintln!("!<Tag> result: {:?}", result);
    // Check if this format works
    if result.is_err() {
        eprintln!("!<Tag> format NOT supported by serde_yaml");
    }
}

#[test]
fn test_nested_in_struct() {
    #[derive(Debug, Deserialize)]
    struct Container {
        transform: Transform,
    }
    
    let yaml = "transform: !MatrixTransform {matrix: [1, 0, 0, 1]}";
    let result: Result<Container, _> = serde_yaml::from_str(yaml);
    eprintln!("Nested result: {:?}", result);
    assert!(result.is_ok(), "Nested tag should work");
}
