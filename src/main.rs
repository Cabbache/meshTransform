use clap::{Parser, Subcommand};
use nalgebra::{Isometry3, Matrix3, Rotation3, Translation3, Unit, Vector3};
use std::io::{self, BufRead};

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
	/// Rotates object
	Rotate {
		#[clap(allow_hyphen_values = true)]
		x: f32,
		#[clap(allow_hyphen_values = true)]
		y: f32,
		#[clap(allow_hyphen_values = true)]
		z: f32,
		#[clap(allow_hyphen_values = true)]
		angle: f32,
	},
	/// Scales object
	Scale {
		#[clap(allow_hyphen_values = true)]
		x: f32,
		#[clap(allow_hyphen_values = true)]
		y: f32,
		#[clap(allow_hyphen_values = true)]
		z: f32,	
	},
	/// Warps object
	Warp,
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
	#[clap(subcommand)]
	command: Commands,
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

		let transforms: Vec<Matrix3<f32>> = Self::create_transformation_matrices(lines.clone())
			.iter()
			.map(|isometry| isometry.rotation.to_rotation_matrix().matrix().clone())
			.collect();

		WarpTransformer {
			lines: lines,
			transforms: transforms,
		}
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

	fn create_transformation_matrix(
		line1: (Vector3<f32>, Vector3<f32>),
		line2: (Vector3<f32>, Vector3<f32>),
	) -> Isometry3<f32> {
		let dir1 = Unit::new_normalize(line1.1 - line1.0);
		let dir2 = Unit::new_normalize(line2.1 - line2.0);

		// Calculate the rotation required to align line2 with line1
		let rotation = Rotation3::rotation_between(&dir2, &dir1).unwrap();

		// Calculate the translation required to move the start of line2 to line1
		let translation = Translation3::from(line1.0 - line2.0);

		// Combine the translation and rotation into a single transformation
		Isometry3::from_parts(translation, rotation.into())
	}

	fn create_transformation_matrices(
		lines: Vec<(Vector3<f32>, Vector3<f32>)>,
	) -> Vec<Isometry3<f32>> {
		let first_line = lines[0];

		let mut transformation_matrices = Vec::new();
		for line in lines {
			let matrix = Self::create_transformation_matrix(first_line, line);
			transformation_matrices.push(matrix);
		}

		transformation_matrices
	}

	fn interpolate_transforms(transforms: &[Matrix3<f32>], weights: &[f32]) -> Matrix3<f32> {
		assert_eq!(
			transforms.len(),
			weights.len(),
			"The number of transforms and weights must be the same"
		);

		let mut result = Matrix3::zeros();
		let sum_weights: f32 = weights.iter().sum();

		for (transform, &weight) in transforms.iter().zip(weights.iter()) {
			result += transform * weight;
		}

		result /= sum_weights;

		result
	}
}

impl Transformer for WarpTransformer {
	fn transform(&self, pt: Vector3<f32>) -> Vector3<f32> {
		let weights: Vec<f32> = self
			.lines
			.iter()
			.map(|&line| Self::perpendicular_distance(pt, line))
			.collect();

		let interpolated_transform = Self::interpolate_transforms(&self.transforms, &weights);
		interpolated_transform * pt
	}
}

struct TranslateTransformer {
	xyz: Vector3<f32>
}

impl Transformer for TranslateTransformer {
	fn transform(&self, pt: Vector3<f32>) -> Vector3<f32> {
		pt + self.xyz
	}
}

struct RotateTransformer {
	axis: Vector3<f32>,
	angle: f32
}

impl Transformer for RotateTransformer {
	fn transform(&self, pt: Vector3<f32>) -> Vector3<f32> {
		let u = self.axis.normalize();
		let sin_angle = self.angle.sin();
		let cos_angle = self.angle.cos();

		let term1 = pt.scale(cos_angle);
		let term2 = u.cross(&pt).scale(sin_angle);
		let term3 = u.scale(u.dot(&pt) * (1.0 - cos_angle));

		term1 + term2 + term3
	}
}

struct ScaleTransformer {
	xyz: Vector3<f32>
}

impl Transformer for ScaleTransformer {
	fn transform(&self, pt: Vector3<f32>) -> Vector3<f32> {
		Vector3::new(
			pt.x * self.xyz.x,
			pt.y * self.xyz.y,
			pt.z * self.xyz.z,
		)
	}
}

fn main() {
	let args = Args::parse();

	let transformer: Box<dyn Transformer> = match args.command {
		Commands::Translate { dx, dy, dz } => Box::new(TranslateTransformer {
			xyz: Vector3::new(dx, dy, dz)
		}),
		Commands::Rotate {x,y,z,angle} => Box::new(RotateTransformer {
			axis: Vector3::new(x,y,z),
			angle: angle
		}),
		Commands::Scale {x,y,z} => Box::new(ScaleTransformer{
			xyz: Vector3::new(x,y,z)
		}),
		Commands::Warp => Box::new(WarpTransformer::new()),
	};

	let stdin = io::stdin();
	for text_line in stdin.lock().lines() {
		let text_line = text_line.unwrap();
		let words: Vec<&str> = text_line.split_whitespace().collect();

		if words[0] != "v" && words[0] != "vertex" || words.len() != 4 {
			println!("{}", text_line);
			continue;
		}

		let x = words[1].parse::<f32>().unwrap();
		let y = words[2].parse::<f32>().unwrap();
		let z = words[3].parse::<f32>().unwrap();
		let output = transformer.transform(Vector3::new(x, y, z));

		println!("{} {} {} {}", words[0], output.x, output.y, output.z);
	}
}
