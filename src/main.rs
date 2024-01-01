use clap::{Parser, Subcommand};
use std::io::{self, BufRead};
use nalgebra::{Matrix3, Isometry3, Rotation3, Translation3, Unit, Vector3};

#[derive(Subcommand)]
enum Commands {
	/// Translates object
	Translate {
		#[clap(allow_hyphen_values = true)]
		dx: f32,
		#[clap(allow_hyphen_values = true)]
		dy: f32,
		#[clap(allow_hyphen_values = true)]
		dz: f32,
	},
	/// Warps object
	Warp
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
	#[clap(subcommand)]
	command: Commands
}

fn perpendicular_distance(point: Vector3<f32>, line: (Vector3<f32>, Vector3<f32>)) -> f32 {
    let (a, b) = line;
    let ab = b - a;
    let ap = point - a;

    let magnitude_ab = ab.magnitude();
    let projection_length = ap.dot(&ab) / (magnitude_ab * magnitude_ab);
    let projection = ab * projection_length;

    let perpendicular = ap - projection;

    perpendicular.magnitude()
}

fn create_transformation_matrix(line1: (Vector3<f32>, Vector3<f32>), line2: (Vector3<f32>, Vector3<f32>)) -> Isometry3<f32> {
    let dir1 = Unit::new_normalize(line1.1 - line1.0);
    let dir2 = Unit::new_normalize(line2.1 - line2.0);

    // Calculate the rotation required to align line2 with line1
    let rotation = Rotation3::rotation_between(&dir2, &dir1).unwrap();

    // Calculate the translation required to move the start of line2 to line1
    let translation = Translation3::from(line1.0 - line2.0);

    // Combine the translation and rotation into a single transformation
    Isometry3::from_parts(translation, rotation.into())
}

fn create_transformation_matrices(lines: Vec<(Vector3<f32>, Vector3<f32>)>) -> Vec<Isometry3<f32>> {
		let first_line = lines[0];

    let mut transformation_matrices = Vec::new();
    for line in lines {
        let matrix = create_transformation_matrix(first_line, line);
        transformation_matrices.push(matrix);
    }

    transformation_matrices
}

fn interpolate_transforms(transforms: &[Matrix3<f32>], weights: &[f32]) -> Matrix3<f32> {
    assert_eq!(transforms.len(), weights.len(), "The number of transforms and weights must be the same");

    let mut result = Matrix3::zeros();
    let sum_weights: f32 = weights.iter().sum();

    for (transform, &weight) in transforms.iter().zip(weights.iter()) {
        result += transform * weight;
    }

    result /= sum_weights;

    result
}

trait Transformer {
	fn transform(&self, pt: Vector3<f32>) -> Vector3<f32>;
}

struct WarpTransformer {
	lines: Vec<(Vector3<f32>, Vector3<f32>)>,
	transforms: Vec<Matrix3<f32>>,
}

impl WarpTransformer {
	fn new() -> Self {
		let rotation_angle = 90.0_f32.to_radians();
		let line1 = (Vector3::zeros(), Vector3::x());
		let rotation = Rotation3::from_axis_angle(&Vector3::z_axis(), rotation_angle);
		let line2 = (Vector3::zeros(), rotation * line1.1);
		let rotation = Rotation3::from_axis_angle(&Vector3::y_axis(), rotation_angle);
		let line3 = (Vector3::zeros(), rotation * line2.1);
		let lines = vec![line1, line2, line3];

		let transforms: Vec<Matrix3<f32>> = create_transformation_matrices(lines.clone())
			.iter()
			.map(|isometry| isometry.rotation.to_rotation_matrix().matrix().clone())
			.collect();	

		WarpTransformer {
			lines: lines,
			transforms: transforms
		}
	}
}

impl Transformer for WarpTransformer {
	fn transform(&self, pt: Vector3<f32>) -> Vector3<f32> {
		let weights: Vec<f32> = self.lines.iter()
		 .map(|&line| perpendicular_distance(pt, line))
		.collect();

		let interpolated_transform = interpolate_transforms(&self.transforms, &weights);
		interpolated_transform * pt
	}
}

struct TranslateTransformer {
	dx: f32,
	dy: f32,
	dz: f32,
}

impl Transformer for TranslateTransformer {
	fn transform(&self, pt: Vector3<f32>) -> Vector3<f32> {
		pt + Vector3::new(self.dx, self.dy, self.dz)
	}
}

fn main() {
	let args = Args::parse();

	let transformer: Box<dyn Transformer> = match args.command {
		Commands::Translate{dx, dy ,dz} => Box::new(TranslateTransformer {
			dx: dx,
			dy: dy,
			dz: dz,
		}),
		Commands::Warp => Box::new(WarpTransformer::new())
	};

	let stdin = io::stdin();
	for text_line in stdin.lock().lines() {
		let text_line = text_line.unwrap();
		let words: Vec<&str> = text_line.split_whitespace().collect();

		if words[0] != "v" || words.len() != 4 {
			println!("{}", text_line);
			continue;
		}

		let x = words[1].parse::<f32>().unwrap();
		let y = words[2].parse::<f32>().unwrap();
		let z = words[3].parse::<f32>().unwrap();
		let output = transformer.transform(Vector3::new(x,y,z));
	
		println!("v {} {} {}", output.x, output.y, output.z);
	}
}
