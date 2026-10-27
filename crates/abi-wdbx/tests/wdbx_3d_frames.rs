use abi_wdbx::spatial::Point3D;
use abi_wdbx::spatial3d::{
    CartesianFrameRef, CartesianPoint3, FrameConversion, FrameError, Point3, SpatialError,
};

const IDENTITY: [[i8; 3]; 3] = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];

fn frame(value: u8) -> CartesianFrameRef {
    CartesianFrameRef::from_definition_digest([value; 32])
}

fn p(x: f64, y: f64, z: f64) -> Point3 {
    Point3::try_new(x, y, z).unwrap()
}

fn conversion(rotation: [[i8; 3]; 3], scale: f64, translation: Point3) -> FrameConversion {
    FrameConversion::try_new(frame(1), frame(2), rotation, scale, translation).unwrap()
}

#[test]
fn unbound_legacy_is_not_meters() {
    let original = Point3D {
        x: 1.,
        y: -0.,
        z: 3.,
    };
    let bits = [
        original.x.to_bits(),
        original.y.to_bits(),
        original.z.to_bits(),
    ];
    let view = CartesianPoint3::from_legacy(original).unwrap();
    assert_eq!(view.frame(), None);
    assert_eq!(view.point(), p(1., 0., 3.));
    assert_eq!(
        conversion(IDENTITY, 1., p(0., 0., 0.)).convert(view),
        Err(FrameError::Unbound)
    );
    assert_eq!(
        [
            original.x.to_bits(),
            original.y.to_bits(),
            original.z.to_bits()
        ],
        bits
    );
    assert!(
        CartesianPoint3::from_legacy(Point3D {
            x: f32::NAN,
            ..original
        })
        .is_err()
    );
}

#[test]
fn mixed_frame_and_changed_definition_are_rejected() {
    let c = conversion(IDENTITY, 1., p(0., 0., 0.));
    let mut changed_digest = frame(1).definition_digest();
    changed_digest[31] ^= 1;
    for wrong in [
        frame(2),
        CartesianFrameRef::from_definition_digest(changed_digest),
    ] {
        assert_eq!(
            c.convert(CartesianPoint3::new(p(1., 2., 3.), wrong)),
            Err(FrameError::MismatchedSource)
        );
    }
    let output = c
        .convert(CartesianPoint3::new(p(1., 2., 3.), frame(1)))
        .unwrap();
    assert_eq!(output.frame(), Some(frame(2)));
    assert_eq!(output.point(), p(1., 2., 3.));
    assert_eq!(c.convert(output), Err(FrameError::MismatchedSource));
}

#[test]
fn explicit_rotation_scale_and_target_translation() {
    let c = conversion([[0, -1, 0], [1, 0, 0], [0, 0, 1]], 0.5, p(10., 20., 30.));
    let input = CartesianPoint3::new(p(2., 4., 6.), frame(1));
    let output = c.convert(input).unwrap();
    assert_eq!(output, CartesianPoint3::new(p(8., 21., 33.), frame(2)));
    assert_eq!(input.point(), p(2., 4., 6.));
}

#[test]
fn all_proper_axis_rotations_match_integer_oracle() {
    // Permutations and parity are independently tabulated, not derived from
    // the production determinant. Small integers make every expected f64 exact.
    let permutations = [
        ([0, 1, 2], 1),
        ([0, 2, 1], -1),
        ([1, 0, 2], -1),
        ([1, 2, 0], 1),
        ([2, 0, 1], 1),
        ([2, 1, 0], -1),
    ];
    let mut accepted = 0;
    for (axes, parity) in permutations {
        for sx in [-1i8, 1] {
            for sy in [-1i8, 1] {
                for sz in [-1i8, 1] {
                    let signs = [sx, sy, sz];
                    let mut matrix = [[0; 3]; 3];
                    for i in 0..3 {
                        matrix[i][axes[i]] = signs[i];
                    }
                    let result =
                        FrameConversion::try_new(frame(1), frame(2), matrix, 2., p(10., 20., 30.));
                    if parity * sx * sy * sz == -1 {
                        assert_eq!(result.unwrap_err(), FrameError::UnsupportedRotation);
                        continue;
                    }
                    let input = [2i32, -3, 5];
                    let translation = [10, 20, 30];
                    let expected: [f64; 3] = std::array::from_fn(|i| {
                        f64::from(i32::from(signs[i]) * input[axes[i]] * 2 + translation[i])
                    });
                    let output = result
                        .unwrap()
                        .convert(CartesianPoint3::new(p(2., -3., 5.), frame(1)))
                        .unwrap();
                    assert_eq!(output.point().coordinates(), expected);
                    assert_eq!(output.frame(), Some(frame(2)));
                    accepted += 1;
                }
            }
        }
    }
    assert_eq!(accepted, 24);
}

#[test]
fn singular_sheared_reflected_and_non_unit_matrices_refuse() {
    for matrix in [
        [[0; 3]; 3],
        [[1, 0, 0], [1, 0, 0], [0, 0, 1]],
        [[1, 1, 0], [0, 1, 0], [0, 0, 1]],
        [[-1, 0, 0], [0, 1, 0], [0, 0, 1]],
        [[2, 0, 0], [0, 1, 0], [0, 0, 1]],
        [[i8::MIN, 0, 0], [0, 1, 0], [0, 0, 1]],
    ] {
        assert_eq!(
            FrameConversion::try_new(frame(1), frame(2), matrix, 1., p(0., 0., 0.)).unwrap_err(),
            FrameError::UnsupportedRotation
        );
    }
}

#[test]
fn invalid_scales_and_transformed_coordinates_refuse() {
    for scale in [0., -0., -1., f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            FrameConversion::try_new(frame(1), frame(2), IDENTITY, scale, p(0., 0., 0.))
                .unwrap_err(),
            FrameError::InvalidScale
        );
    }
    let input = CartesianPoint3::new(p(1e150, 0., 0.), frame(1));
    for (scale, translation, expected) in [
        (2., p(0., 0., 0.), SpatialError::CoordinateOutOfRange),
        (1., p(1e150, 0., 0.), SpatialError::CoordinateOutOfRange),
        (f64::MAX, p(0., 0., 0.), SpatialError::NonFinite),
    ] {
        assert_eq!(
            conversion(IDENTITY, scale, translation).convert(input),
            Err(FrameError::Numeric(expected))
        );
    }
}

#[test]
fn signed_zero_subnormals_and_domain_boundary_survive() {
    let tiny = f64::from_bits(1);
    for input in [p(-0., 0., -0.), p(tiny, -tiny, 0.), p(1e150, -1e150, tiny)] {
        let output = conversion(IDENTITY, 1., p(0., 0., 0.))
            .convert(CartesianPoint3::new(input, frame(1)))
            .unwrap();
        assert_eq!(output.point(), input);
    }
    let output = conversion(IDENTITY, 1., p(-0., -0., -0.))
        .convert(CartesianPoint3::new(p(-0., -0., -0.), frame(1)))
        .unwrap();
    assert!(
        output
            .point()
            .coordinates()
            .iter()
            .all(|v| v.to_bits() == 0)
    );
}
