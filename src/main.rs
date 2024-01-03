use clap::{Parser, Subcommand};
use nalgebra::{Isometry3, Matrix3, Rotation3, Translation3, Unit, Vector3};
use std::io::{self, BufRead};

#[derive(Clone, Copy)]
struct Line {
	origin: Vector3<f32>,
	heading: Vector3<f32>,
}

fn parse_vector3(s: &str) -> Result<Vector3<f32>, &'static str> {
	let parts: Vec<&str> = s.split(',').collect();
	if parts.len() != 3 {
		return Err("Each vector must have exactly three coordinates");
	}
	let coords: Result<Vec<f32>, _> = parts.iter().map(|&num| num.parse::<f32>()).collect();
	match coords {
		Ok(coords) if coords.len() == 3 => Ok(Vector3::new(coords[0], coords[1], coords[2])),
		_ => Err("Invalid vector format"),
	}
}

fn parse_line(s: &str) -> Result<Line, &'static str> {
	let vectors: Vec<&str> = s.split_whitespace().collect();
	if vectors.len() != 2 {
		return Err("Each line must be defined by exactly two vectors");
	}
	let origin = parse_vector3(vectors[0])?;
	let heading = parse_vector3(vectors[1])?;
	Ok(Line { origin, heading })
}

#[derive(Subcommand)]
enum Commands {
	/// Translates object
	Translate {
		#[clap(allow_hyphen_values = true, value_parser = parse_vector3, value_name="vector", help="vector with comma separated values")]
		translation: Vector3<f32>,
	},
	/// Rotates object
	Rotate {
		#[clap(allow_hyphen_values = true, value_parser = parse_vector3, value_name="vector", help="vector with comma separated values")]
		axis: Vector3<f32>,
		#[clap(allow_hyphen_values = true)]
		angle: f32,
	},
	/// Scales object
	Scale {
		#[clap(allow_hyphen_values = true, value_parser = parse_vector3, value_name="vector", help="vector with comma separated values")]
		scale: Vector3<f32>,
	},
	/// Warps object. This transformation is non-linear
	Warp {
		#[clap(long, allow_hyphen_values = true, value_parser = parse_line, long="line", value_name="line", help="Specifies a line with two vectors. Should be used multiple times")]
		lines: Vec<Line>,
	},
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
	lines: Vec<Line>,
	transforms: Vec<Matrix3<f32>>,
}

impl WarpTransformer {
	fn new(lines: Vec<Line>) -> Self {
		let transforms: Vec<Matrix3<f32>> = Self::create_transformation_matrices(lines.clone())
			.iter()
			.map(|isometry| isometry.rotation.to_rotation_matrix().matrix().clone())
			.collect();

		WarpTransformer {
			lines: lines,
			transforms: transforms,
		}
	}

	fn perpendicular_distance(point: Vector3<f32>, line: Line) -> f32 {
		let ab = line.heading - line.origin;
		let ap = point - line.origin;

		let magnitude_ab = ab.magnitude();
		let projection_length = ap.dot(&ab) / (magnitude_ab * magnitude_ab);
		let projection = ab * projection_length;

		let perpendicular = ap - projection;

		perpendicular.magnitude()
	}

	fn create_transformation_matrix(line1: Line, line2: Line) -> Isometry3<f32> {
		let dir1 = Unit::new_normalize(line1.heading - line1.origin);
		let dir2 = Unit::new_normalize(line2.heading - line2.origin);

		// Calculate the rotation required to align line2 with line1
		let rotation = Rotation3::rotation_between(&dir2, &dir1).unwrap();

		// Calculate the translation required to move the start of line2 to line1
		let translation = Translation3::from(line1.origin - line2.origin);

		// Combine the translation and rotation into a single transformation
		Isometry3::from_parts(translation, rotation.into())
	}

	fn create_transformation_matrices(lines: Vec<Line>) -> Vec<Isometry3<f32>> {
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
			result += transform * weight; //TODO this isn't right
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
	xyz: Vector3<f32>,
}

impl Transformer for TranslateTransformer {
	fn transform(&self, pt: Vector3<f32>) -> Vector3<f32> {
		pt + self.xyz
	}
}

struct RotateTransformer {
	axis: Vector3<f32>,
	angle: f32,
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
	xyz: Vector3<f32>,
}

impl Transformer for ScaleTransformer {
	fn transform(&self, pt: Vector3<f32>) -> Vector3<f32> {
		Vector3::new(pt.x * self.xyz.x, pt.y * self.xyz.y, pt.z * self.xyz.z)
	}
}

fn main() {
	let args = Args::parse();

	let transformer: Box<dyn Transformer> = match args.command {
		Commands::Rotate { axis, angle } => Box::new(RotateTransformer {
			axis: axis,
			angle: angle,
		}),
		Commands::Translate { translation } => Box::new(TranslateTransformer { xyz: translation }),
		Commands::Scale { scale } => Box::new(ScaleTransformer { xyz: scale }),
		Commands::Warp { lines } => Box::new(WarpTransformer::new(match lines.len() {
			0 => vec![
				Line {
					origin: Vector3::new(0f32, 0f32, 0f32),
					heading: Vector3::new(1f32, 0f32, 0f32),
				},
				Line {
					origin: Vector3::new(0f32, 0f32, 0f32),
					heading: Vector3::new(0f32, 0f32, 1f32),
				},
			],
			1 => {
				eprintln!("A minimum of two lines is required.");
				return;
			}
			_ => lines,
		})),
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
