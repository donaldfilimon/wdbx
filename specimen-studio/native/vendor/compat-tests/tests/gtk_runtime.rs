use glib::subclass::prelude::*;
use gtk::{prelude::*, subclass::prelude::*};

mod imp {
    use super::*;
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(
        string = "<interface><template class='StudioGtkCompatBox' parent='GtkBox'><child><object class='GtkLabel' id='caption'><property name='label'>Studio GTK fixture</property></object></child></template></interface>"
    )]
    pub struct FixtureBox {
        #[template_child]
        pub caption: gtk::subclass::widget::TemplateChild<gtk::Label>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for FixtureBox {
        const NAME: &'static str = "StudioGtkCompatBox";
        type Type = super::FixtureBox;
        type ParentType = gtk::Box;
        fn class_init(class: &mut Self::Class) {
            Self::bind_template(class);
        }
        fn instance_init(object: &glib::subclass::InitializingObject<Self>) {
            object.init_template();
        }
    }
    impl ObjectImpl for FixtureBox {}
    impl WidgetImpl for FixtureBox {}
    impl ContainerImpl for FixtureBox {}
    impl BoxImpl for FixtureBox {}
}

glib::wrapper! {
    pub struct FixtureBox(ObjectSubclass<imp::FixtureBox>)
        @extends gtk::Widget, gtk::Container, gtk::Box,
        @implements gtk::Buildable, gtk::Orientable;
}

#[test]
fn public_gtk_derive_binds_real_template_child() {
    gtk::init().expect("native GTK runtime and declared display are required");
    let widget: FixtureBox = glib::Object::new();
    assert_eq!(widget.imp().caption.label(), "Studio GTK fixture");
}
