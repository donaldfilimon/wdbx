use glib::{prelude::*, variant::ToVariant};

#[derive(Debug, PartialEq, Eq, Clone, Copy, glib::Enum)]
#[repr(i32)]
#[enum_type(name = "StudioGtkCompatSignal")]
enum Signal {
    Rest = 7,
    Active = 23,
}

#[test]
fn enum_value_and_clone_closure_roundtrip() {
    for signal in [Signal::Rest, Signal::Active] {
        let value = signal.to_value();
        assert_eq!(value.type_(), Signal::static_type());
        assert_eq!(value.type_().name(), "StudioGtkCompatSignal");
        assert_eq!(value.get::<Signal>().unwrap(), signal);
    }
    let message = std::rc::Rc::new(String::from("studio compatibility"));
    let copy_message = glib::clone!(@strong message => move || message.to_string());
    drop(message);
    assert_eq!(copy_message(), "studio compatibility");
}

#[test]
fn optimized_string_iteration_covers_empty_unicode_and_mixed_ends() {
    for values in [vec![], vec!["独"], vec!["α", "middle", "🦀", "終"]] {
        let variant = values.to_variant();
        assert_eq!(
            variant.array_iter_str().unwrap().collect::<Vec<_>>(),
            values
        );
        assert_eq!(
            variant.array_iter_str().unwrap().rev().collect::<Vec<_>>(),
            values.iter().copied().rev().collect::<Vec<_>>()
        );
        assert_eq!(
            variant.array_iter_str().unwrap().last(),
            values.last().copied()
        );
        let mut iterator = variant.array_iter_str().unwrap();
        if values.len() == 4 {
            assert_eq!(iterator.next(), Some("α"));
            assert_eq!(iterator.next_back(), Some("終"));
            assert_eq!(iterator.nth(1), Some("🦀"));
            assert_eq!(iterator.next(), None);
            assert_eq!(variant.array_iter_str().unwrap().nth_back(1), Some("🦀"));
        } else {
            assert_eq!(iterator.nth(usize::MAX), None);
            assert_eq!(iterator.next_back(), None);
        }
    }
}
