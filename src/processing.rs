use crate::misc::{Brick, Vertex, index, process_curr_color_idx, process_vertices_faces};
use crate::tribox::*;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
};

fn tri_voxel_overlap(
    triverts: &[[f32; 3]; 3],
    origin: &[f32; 3],
    voxel_size: f32,
    size: usize,
) -> Vec<(usize, usize, usize)> {
    let mut tri_min = triverts[0];
    let mut tri_max = triverts[0];
    for vert in triverts {
        for dim in 0..3 {
            if tri_min[dim] > vert[dim] {
                tri_min[dim] = vert[dim];
            }
            if tri_max[dim] < vert[dim] {
                tri_max[dim] = vert[dim];
            }
        }
    }

    let ijk_min: Vec<usize> = (0..3)
        .map(|dim| {
            let begin = f32::floor((tri_min[dim] - origin[dim]) / voxel_size - 0.1) as isize;
            if begin < 0 { 0usize } else { begin as usize }
        })
        .collect();
    let ijk_max: Vec<usize> = (0..3)
        .map(|dim| {
            let end = f32::ceil((tri_max[dim] - origin[dim]) / voxel_size + 0.1) as usize;
            if end > size { size } else { end }
        })
        .collect();

    let mut output = Vec::new();
    for i in ijk_min[0]..ijk_max[0] {
        for j in ijk_min[1]..ijk_max[1] {
            for k in ijk_min[2]..ijk_max[2] {
                let boxcenter = [
                    origin[0] + i as f32 * voxel_size + voxel_size / 2.0,
                    origin[1] + j as f32 * voxel_size + voxel_size / 2.0,
                    origin[2] + k as f32 * voxel_size + voxel_size / 2.0,
                ];
                if tri_box_overlap(
                    &boxcenter,
                    &[voxel_size / 2.0, voxel_size / 2.0, voxel_size / 2.0],
                    triverts,
                ) {
                    output.push((i, j, k));
                }
            }
        }
    }
    output
}

fn bounding_box(vertices: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut bounding_min = vertices[0];
    let mut bounding_max = vertices[0];
    for vert in vertices.iter().skip(1) {
        for dim in 0..3 {
            if vert[dim] < bounding_min[dim] {
                bounding_min[dim] = vert[dim];
            }
            if vert[dim] > bounding_max[dim] {
                bounding_max[dim] = vert[dim];
            }
        }
    }
    (bounding_min, bounding_max)
}

fn voxel_grid(vertices: &[[f32; 3]], size: usize) -> ([f32; 3], f32) {
    let (bound_min, bound_max) = bounding_box(vertices);

    let voxel_size: f32 = (0..3)
        .map(|i| bound_max[i] - bound_min[i])
        .fold(0.0, f32::max)
        / size as f32;
    (bound_min, voxel_size)
}

pub fn obj2voxel(
    input_file_path: &str,
    size: usize,
    mtl_file_path: Option<&str>,
) -> (Vec<usize>, f32, Vec<String>) {
    let (vertices, faces, face_colors_idxs, colors) =
        load_obj(input_file_path, mtl_file_path).unwrap();
    let (origin, voxel_size) = voxel_grid(vertices.as_slice(), size);
    let mut voxels: Vec<usize> = vec![0; size.pow(3)];

    for (idx, face) in faces.iter().enumerate() {
        for k in 1..face.len() - 1 {
            let triverts = [vertices[face[0]], vertices[face[k]], vertices[face[k + 1]]];

            for (i, j, k) in tri_voxel_overlap(&triverts, &origin, voxel_size, size) {
                voxels[i * size * size + j * size + k] = if !face_colors_idxs.is_empty() {
                    face_colors_idxs[idx]
                } else {
                    1
                };
            }
        }
    }

    (voxels, voxel_size, colors)
}

pub fn merge_voxels(
    voxels: &[usize],
    size: usize,
    voxel_size: f32,
    max_merge_length: Option<usize>,
) -> Vec<Brick> {
    let mut merged = vec![0; voxels.len()];
    let mut bricks: Vec<Brick> = Vec::new();
    let mut voxel_count = 0;
    let mut prev_merge_color_idx = 1;

    for z in 0..size {
        if let Some(max_merge_length_some) = max_merge_length {
            for y in 0..size {
                let mut x = 0;
                while x < size {
                    if voxels[index(x, y, z, size)] == 0 {
                        x += 1;
                        continue;
                    }
                    let start_x = x;
                    let mut x_merged = 0;

                    while x < size
                        && x_merged < max_merge_length_some
                        && voxels[index(x, y, z, size)] != 0
                        && voxels[index(x, y, z, size)] == prev_merge_color_idx
                    {
                        if voxels[index(x, y, z, size)] != 0 {
                            prev_merge_color_idx = voxels[index(x, y, z, size)];
                        }
                        x += 1;
                        x_merged += 1;
                    }

                    bricks.push(Brick {
                        min: [
                            (start_x as f32 - 0.5) * voxel_size,
                            (y as f32 - 0.5) * voxel_size,
                            (z as f32 - 0.5) * voxel_size,
                        ],
                        max: [
                            ((x - 1) as f32 + 0.5) * voxel_size,
                            (y as f32 + 0.5) * voxel_size,
                            (z as f32 + 0.5) * voxel_size,
                        ],
                        color_idx: voxels[index(start_x, y, z, size)],
                    });
                    if x_merged == 0 {
                        prev_merge_color_idx = voxels[index(x, y, z, size)];
                    }

                    for i in 0..x_merged {
                        merged[index(start_x + i, y, z, size)] = 1;
                    }

                    voxel_count += x_merged;
                }
            }

            for x in 0..size {
                let mut y = 0;
                let mut prev_merge_color_idx = voxels[index(0, y, z, size)];
                while y < size {
                    if voxels[index(x, y, z, size)] == 0 || merged[index(x, y, z, size)] == 1 {
                        y += 1;
                        continue;
                    }
                    let start_y = y;
                    let mut y_merged = 0;

                    while y < size
                        && y_merged < max_merge_length_some
                        && voxels[index(x, y, z, size)] != 0
                        && merged[index(x, y, z, size)] == 0
                        && voxels[index(x, y, z, size)] == prev_merge_color_idx
                    {
                        if voxels[index(x, y, z, size)] != 0 {
                            prev_merge_color_idx = voxels[index(x, y, z, size)];
                        }
                        y += 1;
                        y_merged += 1;
                    }

                    bricks.push(Brick {
                        min: [
                            (x as f32 - 0.5) * voxel_size,
                            (start_y as f32 - 0.5) * voxel_size,
                            (z as f32 - 0.5) * voxel_size,
                        ],
                        max: [
                            (x as f32 + 0.5) * voxel_size,
                            ((y - 1) as f32 + 0.5) * voxel_size,
                            (z as f32 + 0.5) * voxel_size,
                        ],
                        color_idx: voxels[index(x, start_y, z, size)],
                    });
                    if y_merged == 0 {
                        prev_merge_color_idx = voxels[index(x, y, z, size)];
                    }

                    for i in 0..y_merged {
                        merged[index(x, start_y + i, z, size)] = 1;
                    }

                    voxel_count += y_merged;
                }
            }
        }

        for y in 0..size {
            for x in 0..size {
                if voxels[index(x, y, z, size)] != 0 && merged[index(x, y, z, size)] == 0 {
                    bricks.push(Brick {
                        min: [
                            (x as f32 - 0.5) * voxel_size,
                            (y as f32 - 0.5) * voxel_size,
                            (z as f32 - 0.5) * voxel_size,
                        ],
                        max: [
                            (x as f32 + 0.5) * voxel_size,
                            (y as f32 + 0.5) * voxel_size,
                            (z as f32 + 0.5) * voxel_size,
                        ],
                        color_idx: voxels[index(x, y, z, size)],
                    });

                    voxel_count += 1;
                }
            }
        }
    }
    println!("Loaded {} bricks from {} voxels", bricks.len(), voxel_count);

    bricks
}

fn load_obj(
    path: &str,
    mtl_file_path: Option<&str>,
) -> std::io::Result<(Vec<[f32; 3]>, Vec<Vec<usize>>, Vec<usize>, Vec<String>)> {
    let file = File::open(path).expect("Failed to open {path}");
    let reader = BufReader::new(file);

    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut faces: Vec<Vec<usize>> = Vec::new();
    let mut curr_color_idx: usize = 999999999;
    let mut face_colors_idxs: Vec<usize> = Vec::new();
    let mut colors: Vec<String> = vec![];
    if let Some(path) = mtl_file_path {
        colors = load_mtl(path)?;
    }

    for line_res in reader.lines() {
        let line = line_res?;
        let line = line.trim();

        if line.starts_with("v ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let x: f32 = parts[1].parse().unwrap_or(0.0);
                let y: f32 = parts[2].parse().unwrap_or(0.0);
                let z: f32 = parts[3].parse().unwrap_or(0.0);
                vertices.push([x, y, z]);
            }
        } else if line.starts_with("f ") {
            let parts: Vec<&str> = line.split_whitespace().skip(1).collect();
            let face_idxs: Vec<usize> = parts
                .iter()
                .map(|p| {
                    p.split('/')
                        .next()
                        .unwrap()
                        .parse::<usize>()
                        .unwrap_or(0)
                        .saturating_sub(1)
                })
                .collect();

            if face_idxs.len() >= 3 {
                faces.push(face_idxs);
                if mtl_file_path.is_some() {
                    face_colors_idxs.push(curr_color_idx);
                }
            }
        } else if line.starts_with("usemtl ") {
            let color: &str = line.split_whitespace().nth(1).unwrap();
            process_curr_color_idx(color, colors.as_slice(), &mut curr_color_idx);
        }
    }

    Ok((vertices, faces, face_colors_idxs, colors))
}

fn load_mtl(path: &str) -> std::io::Result<Vec<String>> {
    let file = File::open(path).expect("Failed to open {path}");
    let reader = BufReader::new(file);
    let mut colors = Vec::new();
    let mut current_color: Option<String> = None;

    for line_res in reader.lines() {
        let line = line_res?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();

        if let Some(string) = parts.next() {
            if string == "newmtl" {
                if let Some(color) = current_color.as_ref() {
                    colors.push(color.clone());
                }

                current_color = Some(parts.next().unwrap().to_string());
            }
        }
    }

    if let Some(color) = current_color.as_ref() {
        colors.push(color.clone());
    }
    println!("Colors used: {:?}", colors);

    Ok(colors)
}

pub fn save_as_obj(
    bricks: &[Brick],
    output_path: &str,
    colors: &[String],
    mtl_file_path: Option<&str>,
) -> std::io::Result<()> {
    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);
    let mut prev_color_idx = 999999999;
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut vertex_map: HashMap<Vertex, usize> = HashMap::new();
    let mut faces: Vec<[usize; 4]> = Vec::new();
    let mut face_count: usize = 0;
    let mut bricks = bricks.to_vec();
    bricks.sort_by_key(|b| b.color_idx);

    writeln!(writer, "# Exported bricks as OBJ")?;
    if let Some(path) = mtl_file_path {
        writeln!(writer, "mtllib {path}")?;
    }

    for brick in bricks.iter() {
        process_vertices_faces(
            &mut vertices,
            &mut vertex_map,
            &mut faces,
            brick.min,
            brick.max,
        );
    }

    for v in &vertices {
        writeln!(writer, "v {} {} {}", v[0], v[1], v[2])?;
    }

    for face in faces {
        let brick_color_idx = bricks[face_count / 6].color_idx;
        if brick_color_idx != prev_color_idx && mtl_file_path.is_some() {
            writeln!(writer, "usemtl {}", colors[brick_color_idx])?;
            prev_color_idx = brick_color_idx;
        }

        writeln!(
            writer,
            "f {} {} {} {}",
            face[0] + 1,
            face[1] + 1,
            face[2] + 1,
            face[3] + 1
        )?;

        face_count += 1;
    }

    Ok(())
}
