use abi_wdbx::{
    Point3D,
    spatial3d::{Point3, Radius, SpatialError},
};

#[test]
fn rejects_nonfinite_and_out_of_domain() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for axis in 0..3 {
            let mut xyz = [0.0; 3];
            xyz[axis] = bad;
            assert_eq!(
                Point3::try_new(xyz[0], xyz[1], xyz[2]),
                Err(SpatialError::NonFinite)
            );
        }
    }
    let outside = f64::from_bits(1e150_f64.to_bits() + 1);
    for bad in [outside, -outside, f64::MAX, -f64::MAX] {
        for axis in 0..3 {
            let mut xyz = [0.0; 3];
            xyz[axis] = bad;
            assert_eq!(
                Point3::try_new(xyz[0], xyz[1], xyz[2]),
                Err(SpatialError::CoordinateOutOfRange)
            );
        }
    }
    assert_eq!(
        Point3::try_new(-1e150, 1e150, f64::from_bits(1))
            .unwrap()
            .coordinates(),
        [-1e150, 1e150, f64::from_bits(1)]
    );
}

#[test]
fn widens_all_finite_f32() {
    for value in [
        f32::MIN,
        f32::MAX,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),
        -f32::from_bits(1),
        f32::from_bits(0x007f_ffff),
        1.234_567,
    ] {
        let old = Point3D {
            x: value,
            y: -value,
            z: value,
        };
        let bits = [old.x.to_bits(), old.y.to_bits(), old.z.to_bits()];
        let point = Point3::from_legacy(old).unwrap();
        assert_eq!(
            point.coordinates(),
            [f64::from(value), f64::from(-value), f64::from(value)]
        );
        assert_eq!([point.x(), point.y(), point.z()], point.coordinates());
        assert_eq!([old.x.to_bits(), old.y.to_bits(), old.z.to_bits()], bits);
    }
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            Point3::from_legacy(Point3D {
                x: value,
                y: 0.0,
                z: 0.0
            }),
            Err(SpatialError::NonFinite)
        );
    }
}

#[test]
fn normalizes_only_new_geometric_zero() {
    let old = Point3D {
        x: -0.0,
        y: 0.0,
        z: -0.0,
    };
    let old_bytes = [
        old.x.to_le_bytes(),
        old.y.to_le_bytes(),
        old.z.to_le_bytes(),
    ];
    let point = Point3::from_legacy(old).unwrap();
    assert_eq!(point, Point3::try_new(0.0, -0.0, 0.0).unwrap());
    assert_eq!(point.coordinates().map(f64::to_bits), [0; 3]);
    assert_eq!(
        [
            old.x.to_le_bytes(),
            old.y.to_le_bytes(),
            old.z.to_le_bytes()
        ],
        old_bytes
    );
    assert_eq!(old.x.to_bits(), 0x8000_0000);
}

#[test]
fn radius_domain_is_closed() {
    assert_eq!(Radius::try_new(-1.0), Err(SpatialError::NegativeRadius));
    for value in [-0.0, 0.0, 1e150, f64::from_bits(1)] {
        assert_eq!(Radius::try_new(value).unwrap().get(), value);
    }
    assert_eq!(Radius::try_new(-0.0).unwrap().get().to_bits(), 0);
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(Radius::try_new(value), Err(SpatialError::NonFinite));
    }
    assert_eq!(
        Radius::try_new(f64::from_bits(1e150_f64.to_bits() + 1)),
        Err(SpatialError::CoordinateOutOfRange)
    );
}
