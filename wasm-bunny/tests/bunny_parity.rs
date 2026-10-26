use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    cols: u32,
    rows: u32,
    #[serde(rename = "angleX")]
    angle_x: f64,
    #[serde(rename = "angleY")]
    angle_y: f64,
    ramp: String,
    invert: bool,
    lines: Vec<String>,
}

#[test]
fn bunny_matches_the_javascript_reference() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("fixtures/bunny.json")).expect("fixtures parse");
    assert!(!cases.is_empty());

    for case in cases {
        let actual = wasm_bunny::render(
            i64::from(case.cols),
            i64::from(case.rows),
            case.angle_x,
            case.angle_y,
            &case.ramp,
            case.invert,
        );
        assert_eq!(
            actual.len(),
            case.lines.len(),
            "row count for {}x{} at ({}, {})",
            case.cols,
            case.rows,
            case.angle_x,
            case.angle_y
        );
        for (index, (actual, expected)) in actual.iter().zip(case.lines.iter()).enumerate() {
            assert_eq!(
                actual, expected,
                "row {index} for {}x{} at ({}, {})",
                case.cols, case.rows, case.angle_x, case.angle_y
            );
        }
    }
}
