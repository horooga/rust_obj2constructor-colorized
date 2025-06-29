mod processing;
use processing::*;
mod misc;
use misc::help;
mod tribox;

fn main() {
    let mut args = std::env::args().skip(1);
    if args.len() < 3 {
        help();
        return;
    }
    let input_file_path = args.next().expect("Input file path not provided");
    let size = args
        .next()
        .expect("Size not provided")
        .parse::<usize>()
        .expect("size mut be a positive integer");
    let output_file_path = args.next().expect("Output file path not provided");
    let mut merge_size: Option<usize> = None;
    let mut mtl_file_path: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--max-merge-length" => {
                let val = args.next().expect("Expected value after --merge-size");
                merge_size = Some(val.parse().expect("merge_size must be a positive integer"));
            }
            "--mtl-file-path" => {
                let val = args.next().expect("Expected value after --mtl-path");
                mtl_file_path = Some(val);
            }
            unknown => {
                eprintln!("Unknown argument: {}", unknown);
                return;
            }
        }
    }
    let (voxels, voxel_size, colors) =
        obj2voxel(input_file_path.as_str(), size, mtl_file_path.as_deref());
    let bricks = merge_voxels(voxels.as_slice(), size, voxel_size, merge_size);

    let _ = save_as_obj(
        bricks.as_slice(),
        output_file_path.as_str(),
        colors.as_slice(),
        mtl_file_path.as_deref(),
    );
}
