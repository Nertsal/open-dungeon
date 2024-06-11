use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Shape {
    Circle { radius: Coord },
    Rectangle { width: Coord, height: Coord },
}

impl Shape {
    pub fn circle<T: Float>(radius: T) -> Self {
        Self::Circle {
            radius: radius.as_r32(),
        }
    }

    pub fn square<T: Float>(size: T) -> Self {
        let size = size.as_r32();
        Self::Rectangle {
            width: size,
            height: size,
        }
    }

    pub fn rectangle<T: Float, U: Float>(width: T, height: U) -> Self {
        Self::Rectangle {
            width: width.as_r32(),
            height: height.as_r32(),
        }
    }

    pub fn to_parry(self) -> Box<dyn parry2d::shape::Shape> {
        match self {
            Shape::Circle { radius } => Box::new(parry2d::shape::Ball::new(radius.as_f32())),
            Shape::Rectangle { width, height } => {
                let aabb = Aabb2::ZERO.extend_symmetric(vec2(width, height).as_f32() / 2.0);
                let points = aabb.corners().map(|p| {
                    let vec2(x, y) = p;
                    parry2d::math::Point::new(x, y)
                });
                // All rectangles are convex
                Box::new(parry2d::shape::ConvexPolygon::from_convex_hull(&points).unwrap())
            }
        }
    }
}
