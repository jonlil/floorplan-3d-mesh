use immense::{vertex, write_meshes, ExportConfig, Mesh, MeshGrouping, Rule, Tf};
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
struct Point {
    x: f32,
    y: f32,
}

#[derive(Debug, PartialEq)]
struct PointZ {
    x: f32,
    y: f32,
    z: f32,
}

const FLOOR_Z_INDEX: f32 = 0.0;
const ROOF_Z_INDEX: f32 = 10.0;

impl PointZ {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn floor(x: f32, y: f32) -> Self {
        Self::new(x, y, FLOOR_Z_INDEX)
    }

    fn roof(x: f32, y: f32) -> Self {
        Self::new(x, y, ROOF_Z_INDEX)
    }
}

impl Point {
    fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", content = "value")]
enum FloorplanValue {
    #[serde(rename = "rectanglelabels")]
    Rectanglelabels {},
    #[serde(rename = "polygonlabels")]
    Polygon {
        #[serde(default = "Vec::new")]
        points: Vec<[f32; 2]>,
    },
}

#[derive(Debug)]
struct Floorplan {
    items: Vec<FloorplanValue>,
}

fn read_floorplan_from_file<P: AsRef<Path>>(path: P) -> Result<Floorplan, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let items = serde_json::from_reader(reader)?;

    Ok(Floorplan { items })
}

fn save_mesh_object(mesh: Rc<Mesh>, filename: String) {
    let rule = Rule::new().push(Tf::tx(1.0), mesh);
    let meshes = rule.generate();

    let output = File::create(filename).expect("Failed creating output file");

    write_meshes(
        ExportConfig {
            grouping: MeshGrouping::ByColor,
            export_colors: None,
        },
        meshes,
        &mut BufWriter::new(output),
    )
    .expect("Failed rendering scene");
}

fn generate_meshes(filename: &str, meshes: Vec<Rc<Mesh>>) {
    for (index, mesh) in meshes.into_iter().enumerate() {
        save_mesh_object(
            mesh,
            format!(
                "data/{filename}_{index}.obj",
                filename = filename,
                index = index
            ),
        );
    }
}

impl From<Floorplan3DMesh> for Rc<Mesh> {
    fn from(value: Floorplan3DMesh) -> Rc<Mesh> {
        let vertices = value.vertices.iter().map(|v| vertex(v.x, v.y, v.z));
        Mesh::from(vertices.collect(), None, value.faces)
    }
}

fn process_shapes(points: Vec<Vec<Point>>) -> Vec<Rc<Mesh>> {
    // Create a mesh for each vector of points
    points
        .into_iter()
        .map(Floorplan3DMesh::from)
        .map(|v| v.into())
        .collect()
}

fn main() {
    let floorplan = read_floorplan_from_file("input.json").unwrap();

    let polygons: Vec<Vec<Point>> = floorplan
        .items
        .into_iter()
        .filter_map(|value| match value {
            FloorplanValue::Polygon { points } => {
                if points.len() <= 0 {
                    return None;
                }

                Some(
                    points
                        .into_iter()
                        .map(|point| Point::new(point[0], point[1]))
                        .collect(),
                )
            }
            _ => None,
        })
        .collect();

    let shapes = process_shapes(polygons);
    generate_meshes("wall", shapes);
}

#[derive(Debug, PartialEq)]
struct FloorplanMesh {
    vertices: Vec<Point>,
    faces: Vec<Vec<usize>>,
}

impl From<Vec<Point>> for Floorplan2DMesh {
    fn from(value: Vec<Point>) -> Self {
        let faces: Vec<usize> = (1..=value.len()).collect();
        Self {
            vertices: value,
            faces: vec![faces],
        }
    }
}

impl From<Vec<Point>> for Floorplan3DMesh {
    fn from(value: Vec<Point>) -> Self {
        let shape_2d_points = value.len();
        let mut vertices: Vec<PointZ> = Vec::with_capacity(shape_2d_points * 2);
        let mut faces: MeshFaces = vec![];

        for point in &value {
            vertices.push(PointZ::roof(point.x, point.y));
        }
        for point in value {
            vertices.push(PointZ::floor(point.x, point.y));
        }

        faces.push((1..=shape_2d_points).collect());
        for index in 1..=shape_2d_points {
            let start = match index == shape_2d_points {
                true => 1,
                false => index + 1,
            };

            let p1 = index;
            let p2 = start;
            let p3 = p2 + shape_2d_points;
            let p4 = p1 + shape_2d_points;

            faces.push(vec![p1, p2, p3, p4]);
        }
        faces.push((shape_2d_points + 1..=2 * shape_2d_points).collect());

        Self { vertices, faces }
    }
}

type MeshFaces = Vec<Vec<usize>>;

#[derive(Debug, PartialEq)]
struct Floorplan2DMesh {
    vertices: Vec<Point>,
    faces: MeshFaces,
}

#[derive(Debug, PartialEq)]
struct Floorplan3DMesh {
    vertices: Vec<PointZ>,
    faces: MeshFaces,
}

#[test]
fn test_create_2d_mesh() {
    let points = vec![
        Point::new(0.0, 0.0),
        Point::new(0.0, 5.0),
        Point::new(5.0, 5.0),
        Point::new(5.0, 4.0),
        Point::new(1.0, 4.0),
        Point::new(1.0, 0.0),
    ];

    assert_eq!(
        Floorplan2DMesh::from(vec![
            Point::new(0.0, 0.0),
            Point::new(0.0, 5.0),
            Point::new(5.0, 5.0),
            Point::new(5.0, 4.0),
            Point::new(1.0, 4.0),
            Point::new(1.0, 0.0),
        ]),
        Floorplan2DMesh {
            vertices: points,
            faces: vec![vec![1, 2, 3, 4, 5, 6],],
        },
    );
}

#[test]
fn test_meshing_polygon() {
    assert_eq!(
        Floorplan3DMesh::from(vec![
            Point::new(0.0, 0.0),
            Point::new(0.0, 5.0),
            Point::new(5.0, 5.0),
            Point::new(5.0, 4.0),
            Point::new(1.0, 4.0),
            Point::new(1.0, 0.0),
        ]),
        Floorplan3DMesh {
            vertices: vec![
                PointZ::roof(0.0, 0.0),
                PointZ::roof(0.0, 5.0),
                PointZ::roof(5.0, 5.0),
                PointZ::roof(5.0, 4.0),
                PointZ::roof(1.0, 4.0),
                PointZ::roof(1.0, 0.0),
                PointZ::floor(0.0, 0.0),
                PointZ::floor(0.0, 5.0),
                PointZ::floor(5.0, 5.0),
                PointZ::floor(5.0, 4.0),
                PointZ::floor(1.0, 4.0),
                PointZ::floor(1.0, 0.0),
            ],
            faces: vec![
                vec![1, 2, 3, 4, 5, 6],
                vec![1, 2, 8, 7],
                vec![2, 3, 9, 8],
                vec![3, 4, 10, 9],
                vec![4, 5, 11, 10],
                vec![5, 6, 12, 11],
                vec![6, 1, 7, 12],
                vec![7, 8, 9, 10, 11, 12],
            ],
        }
    );
}
