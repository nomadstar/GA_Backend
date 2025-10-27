#[test]
fn run_rutacritica_clique_example_runs() {
    // This integration test simply invokes the example runner which uses
    // the algorithms fallback data when Excel files are missing. The
    // test passes if the function executes without panicking.
    quickshift::rutacritica::clique::run_clique_example();
}
