
#[cfg(test)]
#[test]
fn canonical_budget_layout_reference() {
    println!("AttachmentContext {}", std::mem::size_of::<AttachmentContext<'_>>());
    println!("Visit {}", std::mem::size_of::<Visit<'_>>());
    println!("Arc<AttachmentContext> {}", std::mem::size_of::<Arc<AttachmentContext<'_>>>());
    println!("RouteCapture {}", std::mem::size_of::<RouteCapture>());
    println!("Span {}", std::mem::size_of::<Span>());
}
