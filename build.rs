use std::io::Write;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_dir = std::path::Path::new(&out_dir);

    // Generate test cases for the integration tests from the Less.js test data
    let test_data_dir = std::path::Path::new("./node_modules/@less/test-data");
    let main_dir = test_data_dir.join("less/_main");

    let destination = out_dir.join("integration_tests_generated.rs");
    let mut file = std::fs::File::create(&destination).unwrap();

    for entry in std::fs::read_dir(&main_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let filename = path.file_name().unwrap().to_str().unwrap();

        if !filename.ends_with(".less") {
            continue;
        }

        let test_name = filename.replace(".less", "").replace("-", "_");
        write!(
            file,
            "
                #[test]
                fn test_{}() {{
                    test_file({:?});
                }}
            ",
            test_name,
            path
        )
        .unwrap();
    }
}
