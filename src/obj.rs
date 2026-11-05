use nalgebra_glm::Vec3;
use std::error::Error;
use std::fs;

pub struct Obj {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<Vec<usize>>,
}

impl Obj {
    pub fn load(path: &str) -> Result<Self, Box<dyn Error>> {
        Self::parse(&fs::read_to_string(path)?)
    }

    pub fn parse(source: &str) -> Result<Self, Box<dyn Error>> {
        let mut vertices = Vec::new();
        let mut faces = Vec::new();

        for line in source.lines() {
            let mut values = line.split('#').next().unwrap_or("").split_whitespace();

            match values.next() {
                Some("v") => {
                    let mut coordinates = [0.0; 3];
                    for coordinate in &mut coordinates {
                        *coordinate = values.next().ok_or("Incomplete OBJ vertex")?.parse()?;
                        if !f32::is_finite(*coordinate) {
                            return Err("Invalid OBJ coordinate".into());
                        }
                    }
                    vertices.push(Vec3::new(coordinates[0], coordinates[1], coordinates[2]));
                }
                Some("f") => {
                    let mut face = Vec::new();
                    for value in values {
                        // OBJ usa índices desde 1; los negativos cuentan desde el último vértice.
                        let index: isize = value.split('/').next().unwrap_or("").parse()?;
                        let index = if index < 0 {
                            vertices.len() as isize + index
                        } else {
                            index - 1
                        };
                        if index < 0 || index as usize >= vertices.len() {
                            return Err("OBJ vertex index out of bounds".into());
                        }
                        face.push(index as usize);
                    }
                    if face.len() < 3 {
                        return Err("An OBJ face needs at least three vertices".into());
                    }
                    faces.push(face);
                }
                _ => {}
            }
        }

        if vertices.is_empty() || faces.is_empty() {
            return Err("The OBJ model has no vertices or faces".into());
        }

        Ok(Obj { vertices, faces })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_vertex_indices_without_texture_or_normal_indices() {
        let obj = Obj::parse("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1//7 2/4/8 3\nf -3 -2 -1").unwrap();

        assert_eq!(obj.vertices[1], Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(obj.faces, vec![vec![0, 1, 2], vec![0, 1, 2]]);
        assert!(Obj::parse("v 0 0 0\nf 0 1 1").is_err());
        assert!(Obj::parse("v 0 0 0\nf 1 2 3").is_err());
    }
}
