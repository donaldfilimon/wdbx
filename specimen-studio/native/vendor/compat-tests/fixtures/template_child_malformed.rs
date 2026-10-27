#[derive(gtk3_macros::CompositeTemplate)]
#[template(string = "<interface/>")]
struct InvalidChild {
    #[template_child = "invalid"]
    child: (),
}
fn main() {}
