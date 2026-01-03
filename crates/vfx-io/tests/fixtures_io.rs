use std::path::PathBuf;
use vfx_io::{read, AttrValue, PixelData};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn read_png_fixture() {
    let image = read(fixture_path("sample.png")).expect("read png");
    assert_eq!(image.width, 640);
    assert_eq!(image.height, 426);
    assert_eq!(image.channels, 3);
    assert_eq!(
        image.metadata.attrs.get("ImageWidth").and_then(|v| v.as_u32()),
        Some(640)
    );
    assert_eq!(
        image.metadata.attrs.get("ImageHeight").and_then(|v| v.as_u32()),
        Some(426)
    );
}

#[test]
fn read_jpeg_fixture() {
    let image = read(fixture_path("owl.jpg")).expect("read jpeg");
    assert_eq!(image.width, 1446);
    assert_eq!(image.height, 1920);
    assert_eq!(image.channels, 3);
    assert_eq!(
        image.metadata.attrs.get("ImageWidth").and_then(|v| v.as_u32()),
        Some(1446)
    );
    assert_eq!(
        image.metadata.attrs.get("ImageHeight").and_then(|v| v.as_u32()),
        Some(1920)
    );
}

#[test]
fn read_tiff_fixture() {
    let image = read(fixture_path("sample.tiff")).expect("read tiff");
    assert_eq!(image.width, 640);
    assert_eq!(image.height, 426);
    assert_eq!(image.channels, 3);
    assert_eq!(
        image.metadata.attrs.get("ImageWidth").and_then(|v| v.as_u32()),
        Some(640)
    );
    assert_eq!(
        image.metadata.attrs.get("ImageHeight").and_then(|v| v.as_u32()),
        Some(426)
    );
}

#[test]
fn read_hdr_fixture() {
    let image = read(fixture_path("test.hdr")).expect("read hdr");
    assert_eq!(image.width, 1024);
    assert_eq!(image.height, 512);
    assert_eq!(image.channels, 3);

    let width = image
        .metadata
        .attrs
        .get("ImageWidth")
        .and_then(|v| v.as_u32());
    let height = image
        .metadata
        .attrs
        .get("ImageHeight")
        .and_then(|v| v.as_u32());

    assert_eq!(width, Some(1024));
    assert_eq!(height, Some(512));

    match image.data {
        PixelData::F32(_) => {}
        _ => panic!("HDR should decode to f32"),
    }
}

#[test]
fn read_exr_fixture() {
    let image = read(fixture_path("test.exr")).expect("read exr");
    assert_eq!(image.width, 911);
    assert_eq!(image.height, 876);
    assert_eq!(image.channels, 4);

    let width = image
        .metadata
        .attrs
        .get("ImageWidth")
        .and_then(|v| v.as_u32());
    let height = image
        .metadata
        .attrs
        .get("ImageHeight")
        .and_then(|v| v.as_u32());

    assert!(width.unwrap_or(0) >= image.width);
    assert!(height.unwrap_or(0) >= image.height);

    assert!(matches!(
        image.metadata.attrs.get("Compression"),
        Some(AttrValue::Str(_))
    ));
}
