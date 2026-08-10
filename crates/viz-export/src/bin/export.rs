use std::fs;
use std::path::Path;

fn write_json<T: serde::Serialize>(dir: &Path, filename: &str, value: &T) {
    let path = dir.join(filename);
    let json = serde_json::to_string_pretty(value).expect("serialization should not fail");
    fs::write(&path, json).unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));
    println!("wrote {}", path.display());
}

fn main() {
    let out_dir = Path::new("web/public/artifacts");
    fs::create_dir_all(out_dir).expect("failed to create output directory");

    let cpu = viz_export::cpu_export::export_cpu(32);
    write_json(out_dir, "trace.json", &cpu);
    write_json(out_dir, "air.json", &cpu.air_rows);

    let poly = viz_export::toy_stark_export::export_polynomial();
    write_json(out_dir, "lde.json", &poly);

    let fri = viz_export::toy_stark_export::export_fri();
    write_json(out_dir, "fri_rounds.json", &fri);

    let proof = viz_export::toy_stark_export::export_proof();
    write_json(out_dir, "proof.json", &proof);

    println!("\ndone. proof accepted: {}", proof.accepted);
    assert!(proof.accepted, "exported proof should verify — something is wrong if it doesn't");
}
